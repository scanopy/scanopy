#!/bin/bash
set -euo pipefail

# SNMP Test Environment — manages 23 snmpd instances on a Proxmox LXC
# Subnet: 192.168.4.0/22 (hosts at 192.168.7.230–252)
# Usage: tools/snmp/snmp-test-env.sh deploy|verify|status

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SNMPGET="${SNMPGET:-/opt/homebrew/opt/net-snmp/bin/snmpget}"
SNMPWALK="${SNMPWALK:-/opt/homebrew/opt/net-snmp/bin/snmpwalk}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

# The device list — addresses, versions, communities, sysNames, unit names and per-host v3 users —
# is generated from the typed definitions in
# backend/src/daemon/discovery/integration/snmp/sim/ and written to lxc/generated/lab.env by
# `make snmp-fixtures`. It used to be repeated here by hand, where it could disagree with the
# agents it was verifying.
LAB_ENV="$SCRIPT_DIR/lxc/generated/lab.env"

# Sourced on demand rather than at startup, because `fixtures` and `deploy` are what create it.
require_lab_env() {
    [ "${LAB_ENV_LOADED:-}" = "1" ] && return
    if [ ! -f "$LAB_ENV" ]; then
        printf "${RED}✗${NC} %s is missing — run 'make snmp-fixtures' first.\n" "$LAB_ENV" >&2
        exit 1
    fi
    # shellcheck source=/dev/null
    . "$LAB_ENV"
    LAB_ENV_LOADED=1
}

# SNMPv3 passphrases. The user names come from lab.env; these do not, because the verify path is
# the one consumer that needs the secret rather than the identity.
V3_AUTH_PASS="${V3_AUTH_PASS:-authpass12345}"
V3_PRIV_PASS="${V3_PRIV_PASS:-privpass12345}"
V3_CTX_USER="${V3_CTX_USER:-scanopyctx}"
V3_CTX_AUTH_PASS="${V3_CTX_AUTH_PASS:-ctxauthpass12345}"
V3_CTX_PRIV_PASS="${V3_CTX_PRIV_PASS:-ctxprivpass12345}"
V3_USER="${V3_USER:-scanopyv3}"

# Deploy target: the Proxmox VM that hosts the LXC agents, reached over SSH at the first host in
# the lab (its management IP doubles as switch-core-01's macvlan address). The VM accepts publickey
# auth only, so the key is required. Override either with SNMP_VM_HOST / SNMP_SSH_KEY.
#
# Resolved lazily: the lab's addresses come from the generated lab.env, and `fixtures` is what
# creates it, so this cannot be read at startup.
vm_host() {
    if [ -n "${SNMP_VM_HOST:-}" ]; then
        echo "$SNMP_VM_HOST"
        return
    fi
    require_lab_env
    echo "${HOSTS[0]}"
}
SSH_KEY="${SNMP_SSH_KEY:-$HOME/.ssh/snmp-test-vm}"
REMOTE_DIR="/root/snmp-test"

# ap-wireless-01 advertises 172.30.10.1/24 on a `br-` prefixed interface — the
# #663 fixture, where an access point's NAT guest network was misclassified as a
# Docker bridge. It's the only agent serving its own ipAddrTable, which means it
# is also the only one that breaks silently: if snmpd fails to displace its
# built-in IP module, the `pass` directive loses the duplicate registration and
# the agent quietly falls back to reporting only the scanned subnet. Check it
# explicitly so a scan is never run against a fixture that isn't there.
verify_guest_subnet_fixture() {
    local host="${HOSTS[5]}" community="${COMMUNITIES[5]}"
    local if_index="4" guest_ip="172.30.10.1" if_name="br-guest"

    local got_index got_name
    got_index=$("$SNMPGET" -v2c -c "$community" -t 2 -r 1 -Ovq \
        "$host" ".1.3.6.1.2.1.4.20.1.2.${guest_ip}" 2>/dev/null || echo "FAILED")
    got_name=$("$SNMPGET" -v2c -c "$community" -t 2 -r 1 -Ovq \
        "$host" ".1.3.6.1.2.1.31.1.1.1.1.${if_index}" 2>/dev/null | tr -d '"' || echo "FAILED")

    if [ "$(echo "$got_index" | tr -d ' ')" = "$if_index" ] &&
        [ "$(echo "$got_name" | tr -d ' ')" = "$if_name" ]; then
        printf "  ${GREEN}✓${NC} %-18s  %-20s  %s/24 on %s (#663 fixture)\n" \
            "$host" "guest-subnet" "$guest_ip" "$if_name"
        return 0
    fi

    printf "  ${RED}✗${NC} %-18s  %-20s  ipAdEntIfIndex=%s ifName=%s\n" \
        "$host" "guest-subnet" "$got_index" "$got_name"
    printf "      expected ipAdEntIfIndex=%s and ifName=%s\n" "$if_index" "$if_name"
    printf "      check for a duplicate registration:\n"
    printf "      ssh root@%s 'journalctl -u snmpd-ap-wireless-01 | grep -i duplicate'\n" "${HOSTS[0]}"
    return 1
}

# switch-cisco-01 serves a different bridge forwarding database in its `vlan-20` context than in
# the default one — the GH #686 fixture, where a Catalyst's per-VLAN FDB was unreachable because
# the credential's context name was never put on the wire.
#
# The check is the comparison, not either count on its own. A context-unaware agent answers both
# requests from the same table and they come back equal, which is exactly the failure being
# guarded against; asserting only "the vlan-20 walk returns nine" would pass on an agent that
# ignores `-n` entirely and happens to hold nine rows. `proxy -Cn` is also the one directive here
# that silently degrades: if the back-end agent is down the proxy answers nothing, the context walk
# returns zero rows, and the front agent still looks perfectly healthy.
#
# Both v3 (context name) and v2c (Cisco's `community@vlan` indexing) reach the same back end, so
# one device covers both halves of the report.
verify_vlan_context_fixture() {
    local host="${HOSTS[21]}" fdb=".1.3.6.1.2.1.17.4.3.1.1"

    local v3_default v3_context v2c_context
    v3_default=$("$SNMPWALK" -v3 -l authPriv -u "$V3_CTX_USER" -a SHA-256 -A "$V3_CTX_AUTH_PASS" \
        -x AES -X "$V3_CTX_PRIV_PASS" -t 2 -r 1 "$host" "$fdb" 2>/dev/null | grep -c . || echo 0)
    v3_context=$("$SNMPWALK" -v3 -l authPriv -u "$V3_CTX_USER" -a SHA-256 -A "$V3_CTX_AUTH_PASS" \
        -x AES -X "$V3_CTX_PRIV_PASS" -n vlan-20 -t 2 -r 1 "$host" "$fdb" 2>/dev/null | grep -c . || echo 0)
    v2c_context=$("$SNMPWALK" -v2c -c "netdefault@20" -t 2 -r 1 "$host" "$fdb" 2>/dev/null | grep -c . || echo 0)

    if [ "$v3_default" = "1" ] && [ "$v3_context" = "9" ] && [ "$v2c_context" = "9" ]; then
        printf "  ${GREEN}✓${NC} %-18s  %-20s  default=1 vlan-20=9 (v3 and community@vlan, #686)\n" \
            "$host" "vlan-context"
        return 0
    fi

    printf "  ${RED}✗${NC} %-18s  %-20s  default=%s v3ctx=%s v2cctx=%s\n" \
        "$host" "vlan-context" "$v3_default" "$v3_context" "$v2c_context"
    printf "      expected default=1 v3ctx=9 v2cctx=9\n"
    printf "      a context walk of 0 usually means the proxied back end is down:\n"
    printf "      ssh root@%s 'systemctl status snmpd-switch-cisco-01-vlan20'\n" "${HOSTS[0]}"
    return 1
}

cmd_verify() {
    require_lab_env
    echo "Verifying SNMP test hosts..."
    echo ""
    local all_ok=true
    for i in "${!HOSTS[@]}"; do
        local host="${HOSTS[$i]}"
        local version="${VERSIONS[$i]}"
        local community="${COMMUNITIES[$i]}"
        local expected="${SYSNAMES[$i]}"

        local result detail
        case "$version" in
            v1)
                result=$("$SNMPGET" -v1 -c "$community" -t 2 -r 1 "$host" sysName.0 2>/dev/null | sed 's/.*= STRING: //' || echo "FAILED")
                detail="v1 community=$community"
                ;;
            v3)
                local user="${V3_USERS[$i]:-$V3_USER}" apass="$V3_AUTH_PASS" ppass="$V3_PRIV_PASS"
                if [ "$user" = "$V3_CTX_USER" ]; then
                    apass="$V3_CTX_AUTH_PASS"
                    ppass="$V3_CTX_PRIV_PASS"
                fi
                result=$("$SNMPGET" -v3 -l authPriv -u "$user" -a SHA-256 -A "$apass" -x AES -X "$ppass" -t 2 -r 1 "$host" sysName.0 2>/dev/null | sed 's/.*= STRING: //' || echo "FAILED")
                detail="v3 user=$user"
                ;;
            *)
                result=$("$SNMPGET" -v2c -c "$community" -t 2 -r 1 "$host" sysName.0 2>/dev/null | sed 's/.*= STRING: //' || echo "FAILED")
                detail="v2c community=$community"
                ;;
        esac

        if echo "$result" | grep -q "$expected"; then
            printf "  ${GREEN}✓${NC} %-18s  %-20s  %s\n" "$host" "$expected" "$detail"
        else
            printf "  ${RED}✗${NC} %-18s  expected=%-20s  got=%s\n" "$host" "$expected" "$result"
            all_ok=false
        fi
    done

    echo ""
    verify_guest_subnet_fixture || all_ok=false
    verify_vlan_context_fixture || all_ok=false

    echo ""
    if $all_ok; then
        printf "${GREEN}All %d SNMP test hosts are reachable.${NC}\n" "${#HOSTS[@]}"
        echo ""
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo "  LXC hosts on 192.168.4.0/22"
        echo ""
        printf "  %-18s %-22s %-6s %s\n" "IP" "Host" "Ver" "Credential"
        printf "  %-18s %-22s %-6s %s\n" "────────────────" "────────────────────" "─────" "────────────"
        for i in "${!HOSTS[@]}"; do
            local cred="${COMMUNITIES[$i]}"
            # The per-host USM identity, not a single global one: switch-cisco-01 is on its own
            # user so that only one seeded credential can ever win against it.
            [ "${VERSIONS[$i]}" = "v3" ] && cred="user=${V3_USERS[$i]:-$V3_USER}"
            printf "  %-18s %-22s %-6s %s\n" "${HOSTS[$i]}" "${SYSNAMES[$i]}" "${VERSIONS[$i]}" "$cred"
        done
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    else
        printf "${YELLOW}Some hosts are unreachable. Is the LXC running?${NC}\n"
        echo "  Check with: ssh root@${HOSTS[0]} 'systemctl list-units snmpd-*'"
    fi
}

cmd_status() {
    require_lab_env
    echo "SNMP Test Environment Status"
    echo "=============================="
    echo ""
    echo "Checking reachability (ICMP)..."
    for i in "${!HOSTS[@]}"; do
        local host="${HOSTS[$i]}"
        local name="${SYSNAMES[$i]}"
        if ping -c 1 -W 1 "$host" &>/dev/null; then
            printf "  ${GREEN}✓${NC} %-18s  %s\n" "$host" "$name"
        else
            printf "  ${RED}✗${NC} %-18s  %s  (unreachable)\n" "$host" "$name"
        fi
    done
}

# Push this tools/snmp tree to the VM and (re)build every agent. Idempotent: it
# always clears the remote copy first, because scp -r into an *existing*
# directory nests the tree one level deeper (/root/snmp-test/snmp/...) while
# setup.sh keeps running the stale copy — a silent failure that looks like a
# broken fixture. Deploy does not verify; run `snmp-verify` afterwards (the
# `make snmp-deploy` target chains them).
# Render the devices from their typed definitions into lxc/generated/. Nothing under there is
# committed: the deployment generates it and ships what it generated, so there is no second copy
# of a device that can drift from the struct that defines it.
generate_fixtures() {
    local out="$SCRIPT_DIR/lxc/generated"
    rm -rf "$out"
    (cd "$SCRIPT_DIR/../../backend" &&
        cargo run --quiet --bin generate-snmp-fixtures --features snmp-sim -- "$out")
}

cmd_deploy() {
    if [ ! -f "$SSH_KEY" ]; then
        printf "${RED}✗${NC} SSH key not found: %s\n" "$SSH_KEY"
        echo "  The VM accepts publickey auth only. Point SNMP_SSH_KEY at the key,"
        echo "  or place it at the default path above."
        exit 1
    fi
    local ssh_opts=(-i "$SSH_KEY" -o ConnectTimeout=10)
    local VM_HOST
    VM_HOST="$(vm_host)"

    echo "Generating device definitions..."
    generate_fixtures
    require_lab_env

    echo "Deploying SNMP test environment to root@${VM_HOST}..."
    echo "  → clearing ${REMOTE_DIR} (required — scp -r nests into an existing dir)"
    ssh "${ssh_opts[@]}" "root@${VM_HOST}" "rm -rf ${REMOTE_DIR}"
    echo "  → copying tools/snmp → ${VM_HOST}:${REMOTE_DIR}"
    scp "${ssh_opts[@]}" -q -r "$SCRIPT_DIR" "root@${VM_HOST}:${REMOTE_DIR}"
    echo "  → running lxc/setup.sh on the VM (rebuilds every agent)"
    ssh "${ssh_opts[@]}" "root@${VM_HOST}" "bash ${REMOTE_DIR}/lxc/setup.sh"
    echo ""
    printf "${GREEN}Deploy complete.${NC} Verify with: make snmp-verify\n"
}

case "${1:-}" in
    fixtures)
        generate_fixtures
        ;;
    deploy)
        cmd_deploy
        ;;
    verify)
        cmd_verify
        ;;
    status)
        cmd_status
        ;;
    *)
        echo "Usage: $0 {fixtures|deploy|verify|status}"
        echo ""
        echo "  fixtures — Generate lxc/generated/ from the typed device definitions"
        echo "  deploy — Generate, copy tools/snmp to the VM, and rebuild every agent (needs SSH key)"
        echo "  verify — Query each SNMP host and check sysName"
        echo "  status — Ping each host to check reachability"
        exit 1
        ;;
esac
