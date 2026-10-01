#!/bin/bash
set -euo pipefail

# ══════════════════════════════════════════════════════════════════════
# SSH Test Environment: deploy, verify and manage the SSH credential lab.
#
# Runs on the Mac. The targets live on the SNMP lab VM (root@192.168.4.21) at
# 192.168.7.160-167, one dedicated sshd per address. See SSH-TEST-ENV.md.
#
# Usage: tools/ssh/ssh-test-env.sh deploy|verify|status|rotate-hostkey <ip>
#
# Override via env:
#   SSH_LAB_VM_HOST   VM management address (default 192.168.4.21)
#   SSH_LAB_VM_KEY    key for root@VM (default $SNMP_SSH_KEY, then ~/.ssh/snmp-test-vm)
#   SSH_LAB_STATE     local state: client key, password, known_hosts
#                     (default ~/.config/scanopy-lab/ssh)
#   SSH_LAB_VERIFY_WAIT  seconds the timeout case must still be running (default 8)
# ══════════════════════════════════════════════════════════════════════

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# The script contract as the backend defines it (`SshScriptField`), emitted by `make generate-fixtures`.
CONTRACT="$SCRIPT_DIR/../../ui/src/lib/data/ssh-script-fields.json"
# shellcheck source=targets.env
. "$SCRIPT_DIR/targets.env"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

VM_HOST="${SSH_LAB_VM_HOST:-192.168.4.21}"
VM_KEY="${SSH_LAB_VM_KEY:-${SNMP_SSH_KEY:-$HOME/.ssh/snmp-test-vm}}"
STATE_DIR="${SSH_LAB_STATE:-$HOME/.config/scanopy-lab/ssh}"
CLIENT_KEY="$STATE_DIR/id_ed25519"
PASSWORD_FILE="$STATE_DIR/password"
KNOWN_HOSTS="$STATE_DIR/known_hosts"
REMOTE_DIR="/root/ssh-test"
VERIFY_WAIT="${SSH_LAB_VERIFY_WAIT:-8}"
STDOUT_CAP=65536

require_vm_key() {
    if [ ! -f "$VM_KEY" ]; then
        printf "${RED}✗${NC} VM SSH key not found: %s\n" "$VM_KEY"
        echo "  The VM accepts publickey auth only. Set SSH_LAB_VM_KEY or SNMP_SSH_KEY."
        exit 1
    fi
}

vm_ssh() {
    ssh -i "$VM_KEY" -o ConnectTimeout=10 "root@${VM_HOST}" "$@"
}

# The lab's own client key and password live outside the repo and survive deploys, so the
# credentials created in Scanopy keep working after a redeploy.
ensure_client_secrets() {
    mkdir -p "$STATE_DIR"
    chmod 700 "$STATE_DIR"
    if [ ! -f "$CLIENT_KEY" ]; then
        ssh-keygen -q -t ed25519 -N '' -C "scanopy-ssh-lab-client" -f "$CLIENT_KEY"
        echo "  → generated lab client key $CLIENT_KEY"
    fi
    if [ ! -s "$PASSWORD_FILE" ]; then
        (umask 077 && head -c 18 /dev/urandom | base64 | tr -d '/+=\n' > "$PASSWORD_FILE")
        echo "  → generated lab password $PASSWORD_FILE"
    fi
}

target_index() {
    local want="$1" i
    for i in "${!SSH_HOSTS[@]}"; do
        if [ "${SSH_HOSTS[$i]}" = "$want" ]; then
            echo "$i"
            return 0
        fi
    done
    return 1
}

# ── deploy ────────────────────────────────────────────────────────────
#
# Always clears the remote copy first: scp -r into an existing directory nests the tree one
# level deeper (/root/ssh-test/ssh/...) while setup.sh keeps running the stale copy, a silent
# failure that looks like a broken target. Deploy does not verify; `make ssh-lab-deploy` chains
# verify after it.
cmd_deploy() {
    require_vm_key
    ensure_client_secrets

    echo "Deploying SSH test environment to root@${VM_HOST}..."
    echo "  → clearing ${REMOTE_DIR} (required: scp -r nests into an existing dir)"
    vm_ssh "rm -rf ${REMOTE_DIR}"
    echo "  → copying tools/ssh → ${VM_HOST}:${REMOTE_DIR}"
    scp -i "$VM_KEY" -o ConnectTimeout=10 -q -r "$SCRIPT_DIR" "root@${VM_HOST}:${REMOTE_DIR}"
    scp -i "$VM_KEY" -o ConnectTimeout=10 -q "${CLIENT_KEY}.pub" "root@${VM_HOST}:${REMOTE_DIR}/lab.pub"
    echo "  → running setup.sh on the VM"
    # The password travels on stdin so it never appears in a process list on either machine.
    vm_ssh "SSH_LAB_PASSWORD=\"\$(cat)\" SSH_LAB_PUBKEY_FILE=${REMOTE_DIR}/lab.pub bash ${REMOTE_DIR}/setup.sh" \
        < "$PASSWORD_FILE"
    echo ""
    printf "${GREEN}Deploy complete.${NC} Verify with: make ssh-lab-verify\n"
}

# ── verify ────────────────────────────────────────────────────────────

# Runs the target's script over plain ssh, the way the daemon will, with a local deadline.
# Sets RUN_RC (124 when the deadline hit), RUN_OUT and RUN_ERR (file paths).
RUN_RC=0
run_script() {
    local ip="$1" auth="$2" script="$3" deadline="$4"
    local opts=(-o "UserKnownHostsFile=$KNOWN_HOSTS" -o StrictHostKeyChecking=accept-new
        -o ConnectTimeout=5 -o LogLevel=ERROR)
    if [ "$auth" = "password" ]; then
        opts+=(-o PubkeyAuthentication=no -o PreferredAuthentications=password
            -o NumberOfPasswordPrompts=1)
    else
        opts+=(-i "$CLIENT_KEY" -o IdentitiesOnly=yes -o BatchMode=yes
            -o PasswordAuthentication=no)
    fi

    (
        if [ "$auth" = "password" ]; then
            export SSH_ASKPASS="$ASKPASS" SSH_ASKPASS_REQUIRE=force
        fi
        exec ssh "${opts[@]}" "${SSH_LAB_USER}@${ip}" 'sh -s' < "$script" > "$RUN_OUT" 2> "$RUN_ERR"
    ) &
    local pid=$! waited=0
    while kill -0 "$pid" 2>/dev/null; do
        if [ "$waited" -ge "$deadline" ]; then
            kill "$pid" 2>/dev/null || true
            wait "$pid" 2>/dev/null || true
            RUN_RC=124
            return
        fi
        sleep 1
        waited=$((waited + 1))
    done
    RUN_RC=0
    wait "$pid" || RUN_RC=$?
}

pass() { printf "  ${GREEN}✓${NC} %-15s %-15s %s\n" "$1" "$2" "$3"; }
fail() {
    printf "  ${RED}✗${NC} %-15s %-15s %s\n" "$1" "$2" "$3"
    VERIFY_OK=false
}

# An ssh-level failure (exit 255) means the target itself is broken, whatever case it runs.
report_ssh_failure() {
    local ip="$1" case_name="$2"
    if grep -q "REMOTE HOST IDENTIFICATION HAS CHANGED" "$RUN_ERR"; then
        fail "$ip" "$case_name" "host key changed since last verify (ssh-keygen -R $ip -f $KNOWN_HOSTS)"
    else
        fail "$ip" "$case_name" "ssh failed: $(tr '\n' ' ' < "$RUN_ERR")"
    fi
}

# Inventory output must name the target and carry exactly one interface: the macvlan holding its
# own address. More means the SetEnv identity did not reach the session and every target would
# report the whole VM. The keys must all be ones the script contract accepts, or the scan reports
# them as not applied.
check_inventory() {
    local ip="$1" case_name="$2"
    local host n_ifaces unknown
    if ! jq -e . "$RUN_OUT" >/dev/null 2>&1; then
        fail "$ip" "$case_name" "output is not valid JSON"
        return
    fi
    host=$(jq -r '.hostname // ""' "$RUN_OUT")
    n_ifaces=$(jq '[.interfaces[]?] | length' "$RUN_OUT")
    unknown=$(jq -r --slurpfile contract "$CONTRACT" '
        ($contract[0] | map(select(.scope == "Host") | .key) + ["interfaces"]) as $host_keys
        | ($contract[0] | map(select(.scope == "Interface") | .key | sub("^interfaces\\[\\]\\."; ""))) as $iface_keys
        | [(keys - $host_keys), ([.interfaces[]? | keys[]] - $iface_keys)] | flatten | unique | join(",")' "$RUN_OUT")
    if [ "$host" != "ssh-${case_name}" ]; then
        fail "$ip" "$case_name" "hostname '$host', expected 'ssh-${case_name}' (SetEnv missing?)"
    elif [ "$n_ifaces" != "1" ]; then
        fail "$ip" "$case_name" "reports $n_ifaces interfaces; expected only the one holding $ip"
    elif [ -n "$unknown" ]; then
        fail "$ip" "$case_name" "prints keys outside the script contract: $unknown"
    else
        pass "$ip" "$case_name" "valid JSON, hostname=$host, $(jq -c '[.interfaces[0].name, .interfaces[0].mac]' "$RUN_OUT")"
    fi
}

cmd_verify() {
    if [ ! -f "$CLIENT_KEY" ] || [ ! -s "$PASSWORD_FILE" ]; then
        printf "${RED}✗${NC} No lab client key or password in %s. Run 'make ssh-lab-deploy' first.\n" "$STATE_DIR"
        exit 1
    fi
    command -v jq >/dev/null || { echo "jq is required (brew install jq)"; exit 1; }
    [ -f "$CONTRACT" ] || { echo "Missing $CONTRACT; run 'make generate-fixtures'"; exit 1; }

    local tmp
    tmp=$(mktemp -d)
    # shellcheck disable=SC2064
    trap "rm -rf '$tmp'" EXIT
    RUN_OUT="$tmp/out"
    RUN_ERR="$tmp/err"
    ASKPASS="$tmp/askpass"
    printf '#!/bin/sh\ncat %q\n' "$PASSWORD_FILE" > "$ASKPASS"
    chmod 700 "$ASKPASS"

    echo "Verifying SSH lab targets (running each case's script as ${SSH_LAB_USER})..."
    echo ""
    VERIFY_OK=true
    local i ip case_name auth script size
    for i in "${!SSH_HOSTS[@]}"; do
        ip="${SSH_HOSTS[$i]}"
        case_name="${SSH_CASES[$i]}"
        auth="${SSH_AUTH[$i]}"
        script="$SCRIPT_DIR/scripts/${SSH_SCRIPTS[$i]}"

        if [ "$case_name" = "timeout" ]; then
            run_script "$ip" "$auth" "$script" "$VERIFY_WAIT"
            if [ "$RUN_RC" = "124" ]; then
                pass "$ip" "$case_name" "still running after ${VERIFY_WAIT}s, as intended"
            elif [ "$RUN_RC" = "255" ]; then
                report_ssh_failure "$ip" "$case_name"
            else
                fail "$ip" "$case_name" "finished early with exit $RUN_RC"
            fi
            continue
        fi

        run_script "$ip" "$auth" "$script" 30
        if [ "$RUN_RC" = "255" ]; then
            report_ssh_failure "$ip" "$case_name"
            continue
        fi

        case "$case_name" in
            inventory-ok | password | hostkey-rotate)
                if [ "$RUN_RC" != "0" ]; then
                    fail "$ip" "$case_name" "script exited $RUN_RC: $(tr '\n' ' ' < "$RUN_ERR")"
                else
                    check_inventory "$ip" "$case_name"
                fi
                ;;
            exit-nonzero)
                if [ "$RUN_RC" = "3" ]; then
                    pass "$ip" "$case_name" "exit 3, stderr: $(tr '\n' ' ' < "$RUN_ERR")"
                else
                    fail "$ip" "$case_name" "exit $RUN_RC, expected 3"
                fi
                ;;
            bad-json)
                if [ "$RUN_RC" != "0" ]; then
                    fail "$ip" "$case_name" "exit $RUN_RC, expected 0"
                elif jq -e . "$RUN_OUT" >/dev/null 2>&1; then
                    fail "$ip" "$case_name" "output parsed as JSON; it must not"
                else
                    pass "$ip" "$case_name" "exit 0, output does not parse"
                fi
                ;;
            oversized)
                size=$(wc -c < "$RUN_OUT" | tr -d ' ')
                if [ "$RUN_RC" != "0" ]; then
                    fail "$ip" "$case_name" "exit $RUN_RC, expected 0"
                elif [ "$size" -le "$STDOUT_CAP" ]; then
                    fail "$ip" "$case_name" "$size bytes, expected more than $STDOUT_CAP"
                elif ! jq -e . "$RUN_OUT" >/dev/null 2>&1; then
                    fail "$ip" "$case_name" "$size bytes but not valid JSON; the cap would not be the only failure"
                else
                    pass "$ip" "$case_name" "$size bytes of valid JSON (cap is $STDOUT_CAP)"
                fi
                ;;
            unknown-keys)
                if [ "$RUN_RC" != "0" ]; then
                    fail "$ip" "$case_name" "exit $RUN_RC, expected 0"
                elif ! jq -e '.hostname and .favorite_color' "$RUN_OUT" >/dev/null 2>&1; then
                    fail "$ip" "$case_name" "expected valid JSON with hostname and favorite_color"
                else
                    pass "$ip" "$case_name" "valid JSON, unknown keys: $(jq -c 'keys - ["hostname","sys_descr","model","interfaces"]' "$RUN_OUT")"
                fi
                ;;
            *)
                fail "$ip" "$case_name" "verify has no expectation for this case"
                ;;
        esac
    done

    # The password target must refuse the key, or an SshKey credential would pass there too and
    # the case would not isolate password auth.
    local pw_idx
    if pw_idx=$(target_index_of_case password); then
        ip="${SSH_HOSTS[$pw_idx]}"
        run_script "$ip" key "$SCRIPT_DIR/scripts/inventory-ok.sh" 15
        if [ "$RUN_RC" = "255" ] && grep -q "Permission denied" "$RUN_ERR"; then
            pass "$ip" "password" "refuses publickey auth"
        else
            fail "$ip" "password" "accepted or errored on publickey auth (exit $RUN_RC)"
        fi
    fi

    echo ""
    if $VERIFY_OK; then
        printf "${GREEN}All %d SSH lab targets behave as their case expects.${NC}\n" "${#SSH_HOSTS[@]}"
        echo "  User ${SSH_LAB_USER}, key ${CLIENT_KEY}, password in ${PASSWORD_FILE}"
    else
        printf "${YELLOW}Some targets failed.${NC} Check: make ssh-lab-status\n"
        exit 1
    fi
}

target_index_of_case() {
    local want="$1" i
    for i in "${!SSH_CASES[@]}"; do
        if [ "${SSH_CASES[$i]}" = "$want" ]; then
            echo "$i"
            return 0
        fi
    done
    return 1
}

# ── rotate-hostkey ────────────────────────────────────────────────────
cmd_rotate_hostkey() {
    local ip="${1:-}"
    if [ -z "$ip" ] || ! target_index "$ip" >/dev/null; then
        echo "Usage: $0 rotate-hostkey <ip>   (one of: ${SSH_HOSTS[*]})"
        exit 1
    fi
    require_vm_key
    echo "Rotating the host key of ${ip}..."
    vm_ssh bash -s -- "$ip" <<'REMOTE'
set -euo pipefail
ip="$1"
key="/etc/ssh-test/targets/${ip}/ssh_host_ed25519_key"
old=$(ssh-keygen -lf "${key}.pub" | awk '{print $2}')
rm -f "$key" "${key}.pub"
ssh-keygen -q -t ed25519 -N '' -C "scanopy-ssh-lab-${ip}" -f "$key"
systemctl restart "sshd-test@${ip}"
new=$(ssh-keygen -lf "${key}.pub" | awk '{print $2}')
echo "  old ${old}"
echo "  new ${new}"
REMOTE
    # Only this script's own pin. Scanopy's pin is the thing under test and is left alone.
    ssh-keygen -R "$ip" -f "$KNOWN_HOSTS" >/dev/null 2>&1 || true
    echo ""
    echo "The next Scanopy scan of ${ip} must refuse the changed key and apply nothing."
}

# ── status ────────────────────────────────────────────────────────────
cmd_status() {
    require_vm_key
    echo "SSH Test Environment Status (root@${VM_HOST})"
    echo "==========================================="
    vm_ssh bash -s -- "$SSH_LAB_CIDR" "$SSH_LAB_BACKEND_PORT" "${SSH_HOSTS[@]}" <<'REMOTE'
set -u
cidr="$1"; port="$2"; shift 2
ok=true
listening="$(ss -Hltn "sport = :${port}" | awk '{print $4}')"
i=0
for ip in "$@"; do
    mv="mv-ssh${i}"
    i=$((i + 1))
    problem=""
    if ! ip link show "$mv" &>/dev/null; then
        problem="link $mv missing"
    elif ! ip -4 -o addr show dev "$mv" | awk '{print $4}' | grep -qx "${ip}/${cidr}"; then
        problem="$mv lacks ${ip}/${cidr}"
    elif ! systemctl is-active --quiet "sshd-test@${ip}"; then
        problem="sshd-test@${ip} is $(systemctl is-active "sshd-test@${ip}")"
    elif ! grep -qx "${ip}:${port}" <<< "$listening"; then
        problem="nothing listening on ${ip}:${port}"
    fi
    if [ -n "$problem" ]; then
        printf "  \033[0;31m✗\033[0m %-15s %s\n" "$ip" "$problem"
        ok=false
    else
        fp=$(ssh-keygen -lf "/etc/ssh-test/targets/${ip}/ssh_host_ed25519_key.pub" | awk '{print $2}')
        printf "  \033[0;32m✓\033[0m %-15s %-8s %s\n" "$ip" "$mv" "$fp"
    fi
done
if nft list table ip scanopy_ssh_lab &>/dev/null; then
    echo "  ✓ nftables table scanopy_ssh_lab present (port 22 → ${port})"
else
    echo "  ✗ nftables table scanopy_ssh_lab missing: systemctl restart ssh-lab-network"
    ok=false
fi
$ok
REMOTE
}

case "${1:-}" in
    deploy) cmd_deploy ;;
    verify) cmd_verify ;;
    status) cmd_status ;;
    rotate-hostkey) cmd_rotate_hostkey "${2:-}" ;;
    *)
        echo "Usage: $0 {deploy|verify|status|rotate-hostkey <ip>}"
        echo ""
        echo "  deploy          Copy tools/ssh to the VM and (re)build every target (needs VM key)"
        echo "  verify          Run each case's script against its target and check the result"
        echo "  status          Check links, addresses, units and the port-22 redirect on the VM"
        echo "  rotate-hostkey  Replace one target's host key, to test the changed-key refusal"
        exit 1
        ;;
esac
