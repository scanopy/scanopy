#!/bin/bash
set -euo pipefail

# SNMP Test Environment — manages simulated snmpd instances on a Proxmox LXC
# Subnet: 192.168.4.0/22 (hosts in the reserved 192.168.7.192-254 block — see
# SNMP-TEST-ENV.md's "Addressing" section for why that range is the lab's)
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

# Deploy target: the Proxmox VM's durable management address (see lxc/setup.sh's "durable
# management address" step) — a static secondary address on eth0, independent of every lab
# macvlan address, so the way in stays reachable even when the lab itself is completely down.
# It used to default to the first lab host's address instead (which doubled as its management
# IP), which is exactly how a broken lab also broke the deploy path. The VM accepts publickey
# auth only, so the key is required. Override either with SNMP_VM_HOST / SNMP_SSH_KEY.
vm_host() {
    if [ -n "${SNMP_VM_HOST:-}" ]; then
        echo "$SNMP_VM_HOST"
        return
    fi
    echo "192.168.4.21"
}
SSH_KEY="${SNMP_SSH_KEY:-$HOME/.ssh/snmp-test-vm}"
REMOTE_DIR="/root/snmp-test"

# Look a device up by sysName and echo its index into the parallel HOSTS/COMMUNITIES/VERSIONS
# arrays. The fixture checks below used to hard-code those indices, which held right up until
# GH #709 and #668 inserted three devices in the middle of the lab: every index past `.244`
# shifted by three, and the vlan-context check silently began walking `switch-stuck-01` instead
# of `switch-cisco-01` and reporting the empty result as a lab failure. Index positions are not
# stable across a growing lab; names are. Exits non-zero if the name is absent so a renamed
# device fails loudly here rather than being verified against whatever moved into its slot.
device_index() {
    local want="$1" i
    for i in "${!SYSNAMES[@]}"; do
        if [ "${SYSNAMES[$i]}" = "$want" ]; then
            echo "$i"
            return 0
        fi
    done
    printf "${RED}✗${NC} no device named %s in the generated lab — check sim/devices/\n" "$want" >&2
    return 1
}

# ap-wireless-01 advertises 172.30.10.1/24 on a `br-` prefixed interface — the
# #663 fixture, where an access point's NAT guest network was misclassified as a
# Docker bridge. Every agent now serves its own ipAddrTable, but this is the only
# one serving an address *beyond* its own, so it is the one where a lost `pass`
# registration is invisible: it would quietly fall back to the built-in IP module
# and drop the guest subnet while still answering. verify_no_leaked_addresses
# catches the general case; this checks the extra address is actually there.
verify_guest_subnet_fixture() {
    local idx
    idx=$(device_index "ap-wireless-01") || return 1
    local host="${HOSTS[$idx]}" community="${COMMUNITIES[$idx]}"
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
    printf "      ssh root@%s 'journalctl -u snmpd-ap-wireless-01 | grep -i duplicate'\n" "$(vm_host)"
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
    local idx
    idx=$(device_index "switch-cisco-01") || return 1
    local host="${HOSTS[$idx]}" fdb=".1.3.6.1.2.1.17.4.3.1.1"

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
    printf "      ssh root@%s 'systemctl status snmpd-switch-cisco-01-vlan20'\n" "$(vm_host)"
    return 1
}

# Every agent must report only its own address(es) — its own, plus whatever `EXTRA_ADDRS` names
# as a declared extra one (the `#663` guest-subnet fixture). This is the check that would have
# caught two workers a cycle each: before it existed, 30 of 31 agents fell through to snmpd's
# built-in IP module and answered from the VM's real kernel address table — loopback, the VM's
# management address, and every other agent's macvlan address — so Scanopy's host-merge logic
# (any one shared address is a match) collapsed the whole lab into a couple of host records.
verify_no_leaked_addresses() {
    local all_ok=true leaked_any=false
    local i host version community
    for i in "${!HOSTS[@]}"; do
        host="${HOSTS[$i]}"
        version="${VERSIONS[$i]}"
        community="${COMMUNITIES[$i]}"
        local allowed=("$host")
        if [ -n "${EXTRA_ADDRS[$i]}" ]; then
            local extra
            IFS=',' read -ra extra <<< "${EXTRA_ADDRS[$i]}"
            allowed+=("${extra[@]}")
        fi

        local reported
        case "$version" in
            v1)
                reported=$("$SNMPWALK" -v1 -c "$community" -t 2 -r 1 -Ovq \
                    "$host" 1.3.6.1.2.1.4.20.1.1 2>/dev/null || true)
                ;;
            v3)
                local user="${V3_USERS[$i]:-$V3_USER}" apass="$V3_AUTH_PASS" ppass="$V3_PRIV_PASS"
                if [ "$user" = "$V3_CTX_USER" ]; then
                    apass="$V3_CTX_AUTH_PASS"
                    ppass="$V3_CTX_PRIV_PASS"
                fi
                reported=$("$SNMPWALK" -v3 -l authPriv -u "$user" -a SHA-256 -A "$apass" \
                    -x AES -X "$ppass" -t 2 -r 1 -Ovq "$host" 1.3.6.1.2.1.4.20.1.1 2>/dev/null || true)
                ;;
            *)
                reported=$("$SNMPWALK" -v2c -c "$community" -t 2 -r 1 -Ovq \
                    "$host" 1.3.6.1.2.1.4.20.1.1 2>/dev/null || true)
                ;;
        esac

        # Keep only lines that are actually dotted quads. A device serving no ipAddrTable answers
        # "No Such Instance currently exists at this OID" on *stdout*, which is not an error to
        # snmpwalk and would otherwise be compared against `allowed` and reported as a leaked
        # address — which is exactly what switch-mute-01 did.
        local addr found=()
        while IFS= read -r addr; do
            [ -z "$addr" ] && continue
            [[ "$addr" =~ ^[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}$ ]] || continue
            found+=("$addr")
        done <<< "$reported"

        if [ "${SUPPRESSES_IPADDR[$i]:-0}" = "1" ]; then
            # Mute by design. Silence is the pass condition, and anything at all is the failure —
            # the built-in IP module answering means the suppressing registration was lost.
            if [ ${#found[@]} -gt 0 ]; then
                printf "  ${RED}✗${NC} %-18s  %-20s  serves %d address(es) but suppresses ipAddrTable\n" \
                    "$host" "address-leak" "${#found[@]}"
                all_ok=false
                leaked_any=true
            fi
            continue
        fi

        # Every other device must serve its own address. A silent table here is the same lost
        # registration, and reporting nothing must not read as "nothing leaked". Checked before
        # any `"${found[@]}"` expansion: this runs under `set -u` on bash 3.2, where expanding an
        # empty array is an unbound-variable error rather than an empty list.
        if [ ${#found[@]} -eq 0 ]; then
            printf "  ${RED}✗${NC} %-18s  %-20s  serves no ipAddrTable row for its own address\n" \
                "$host" "address-leak"
            all_ok=false
            leaked_any=true
            continue
        fi

        local has_own=false
        for addr in "${found[@]}"; do
            [ "$addr" = "$host" ] && has_own=true && break
        done
        if ! $has_own; then
            printf "  ${RED}✗${NC} %-18s  %-20s  serves ipAddrTable rows but not its own address\n" \
                "$host" "address-leak"
            all_ok=false
            leaked_any=true
        fi

        for addr in "${found[@]}"; do
            local ok=false a
            for a in "${allowed[@]}"; do
                [ "$addr" = "$a" ] && ok=true && break
            done
            if ! $ok; then
                printf "  ${RED}✗${NC} %-18s  %-20s  reports %s, which is not its own address\n" \
                    "$host" "address-leak" "$addr"
                all_ok=false
                leaked_any=true
            fi
        done
    done

    if ! $leaked_any; then
        printf "  ${GREEN}✓${NC} %-18s  %-20s  no agent reports an address it does not own\n" \
            "(all hosts)" "address-leak"
    fi
    $all_ok
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
    verify_no_leaked_addresses || all_ok=false

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
        echo "  Check with: ssh root@$(vm_host) 'systemctl list-units snmpd-*'"
        echo "  If only newly added devices failed, the VM is behind the repo: make snmp-deploy"
    fi
}

# ICMP-only reachability said nothing useful while the lab was dead for five days — every
# `snmpd-*` unit can be thrashing in `auto-restart` and this would still just print unreachable
# addresses, no different from any other outage. This instead SSHes to the management address
# (see vm_host()) and checks, on the VM itself, exactly the three things that can go wrong: the
# macvlan link missing, its address missing, or the unit having given up (`failed`).
cmd_status() {
    require_lab_env
    if [ ! -f "$SSH_KEY" ]; then
        printf "${RED}✗${NC} SSH key not found: %s\n" "$SSH_KEY"
        exit 1
    fi
    local ssh_opts=(-i "$SSH_KEY" -o ConnectTimeout=10)
    local VM_HOST
    VM_HOST="$(vm_host)"

    echo "SNMP Test Environment Status"
    echo "=============================="
    echo ""
    echo "Checking links, addresses and units on root@${VM_HOST}..."
    echo ""

    local remote_report
    if ! remote_report=$(ssh "${ssh_opts[@]}" "root@${VM_HOST}" bash -s -- "${HOSTS[@]}" <<'REMOTE'
set -u
links_missing=()
addrs_missing=()
i=0
for ip in "$@"; do
    mv="mv-snmp${i}"
    if ! ip link show "$mv" &>/dev/null; then
        links_missing+=("$mv ($ip)")
    elif ! ip -4 addr show "$mv" | grep -q " ${ip}/"; then
        addrs_missing+=("$mv (want $ip)")
    fi
    i=$((i + 1))
done
units_failed=()
while IFS= read -r u; do
    [ -n "$u" ] && units_failed+=("$u")
done < <(systemctl list-units 'snmpd-*' 'snmp-bulk-refuser-*' --all --no-legend --plain --state=failed | awk '{print $1}')

echo "LINKS_MISSING:${#links_missing[@]}"
printf '%s\n' "${links_missing[@]}"
echo "ADDRS_MISSING:${#addrs_missing[@]}"
printf '%s\n' "${addrs_missing[@]}"
echo "UNITS_FAILED:${#units_failed[@]}"
printf '%s\n' "${units_failed[@]}"
REMOTE
    ); then
        printf "${RED}✗${NC} Could not reach root@%s over SSH.\n" "$VM_HOST"
        exit 1
    fi

    local mode="" n_links=0 n_addrs=0 n_failed=0
    local links=() addrs=() failed=()
    while IFS= read -r line; do
        case "$line" in
            LINKS_MISSING:*)
                mode=links
                n_links="${line#LINKS_MISSING:}"
                continue
                ;;
            ADDRS_MISSING:*)
                mode=addrs
                n_addrs="${line#ADDRS_MISSING:}"
                continue
                ;;
            UNITS_FAILED:*)
                mode=failed
                n_failed="${line#UNITS_FAILED:}"
                continue
                ;;
        esac
        case "$mode" in
            links) links+=("$line") ;;
            addrs) addrs+=("$line") ;;
            failed) failed+=("$line") ;;
        esac
    done <<< "$remote_report"

    if [ "$n_links" = "0" ] && [ "$n_addrs" = "0" ] && [ "$n_failed" = "0" ]; then
        printf "${GREEN}healthy${NC} — all %d links up, all addresses assigned, no failed units.\n" "${#HOSTS[@]}"
        return
    fi

    if [ "$n_links" != "0" ]; then
        printf "${RED}links missing${NC} (%s):\n" "$n_links"
        printf '  %s\n' "${links[@]:-}"
    fi
    if [ "$n_addrs" != "0" ]; then
        printf "${RED}addresses missing${NC} (%s):\n" "$n_addrs"
        printf '  %s\n' "${addrs[@]:-}"
    fi
    if [ "$n_failed" != "0" ]; then
        printf "${RED}units failed${NC} (%s):\n" "$n_failed"
        printf '  %s\n' "${failed[@]:-}"
        echo "  Check with: journalctl -u <unit>"
    fi
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
    # Keeps the device table in SNMP-TEST-ENV.md from the same source as lab.env, so it cannot
    # go stale the way the hand-maintained one did.
    (cd "$SCRIPT_DIR/../../backend" &&
        cargo run --quiet --bin generate-snmp-fixtures --features snmp-sim -- \
            --device-table "$SCRIPT_DIR/SNMP-TEST-ENV.md")
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
        echo "  status — Check links/addresses/unit health on the VM (needs SSH key)"
        exit 1
        ;;
esac
