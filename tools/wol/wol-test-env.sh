#!/bin/bash
set -euo pipefail

# ══════════════════════════════════════════════════════════════════════
# Wake-on-LAN Test Environment: put the target to sleep, check it, wake it. Runs on the Mac.
#
# Usage:
#   tools/wol/wol-test-env.sh sleep [--hibernate]
#   tools/wol/wol-test-env.sh status
#   tools/wol/wol-test-env.sh verify <address> [port]
#   tools/wol/wol-test-env.sh listener-log [-f]
#
# `verify` sends a magic packet from this Mac with its own sender (the python3 snippet below,
# not Scanopy's), so it proves the listener and the addressing work before a daemon is
# involved. <address> is where the packet goes: the node's lab IP for the relay path, or a
# directed broadcast address when sending from inside the lab.
#
# Settings: see proxmox-api.sh, plus:
#   WOL_TARGET_IP        target's lab address; verify also waits for it to answer ping
#   WOL_SECUREON         SecureOn password to append, 12 hex digits (colons allowed)
#   WOL_VERIFY_TIMEOUT   seconds verify waits for the VM to come up (default 120)
# ══════════════════════════════════════════════════════════════════════

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=proxmox-api.sh
. "$SCRIPT_DIR/proxmox-api.sh"

target_vmid() {
    local vmid
    vmid=$(find_vmid "$WOL_TARGET_NAME")
    [ -n "$vmid" ] || die "no VM named $WOL_TARGET_NAME in pool $LAB_POOL; run tools/wol/provision.sh"
    echo "$vmid"
}

# ── sleep ─────────────────────────────────────────────────────────────
#
# Default: ACPI shutdown, forced off after 60 s. --hibernate: suspend to disk, which keeps the
# guest's RAM in a state volume and resumes on the next start, the closest Proxmox gets to S4.
cmd_sleep() {
    require_api
    local vmid upid
    vmid=$(target_vmid)
    if [ "${1:-}" = "--hibernate" ]; then
        echo "Hibernating VM $vmid ($WOL_TARGET_NAME)..."
        upid=$(pve POST "/nodes/${PROXMOX_NODE}/qemu/${vmid}/status/suspend" todisk=1 | jq -r '.data')
    else
        echo "Shutting down VM $vmid ($WOL_TARGET_NAME)..."
        upid=$(pve POST "/nodes/${PROXMOX_NODE}/qemu/${vmid}/status/shutdown" timeout=60 forceStop=1 | jq -r '.data')
    fi
    pve_wait_task "$upid"
    printf "${GREEN}✓${NC} VM %s is now: %s\n" "$vmid" "$(vm_state "$vmid")"
}

# ── status ────────────────────────────────────────────────────────────
cmd_status() {
    require_api
    local vmid state
    vmid=$(target_vmid)
    state=$(vm_state "$vmid")
    echo "WoL Test Environment Status"
    echo "==========================="
    printf "  target   VM %s (%s), MAC %s\n" "$vmid" "$WOL_TARGET_NAME" "$(vm_mac "$vmid")"
    printf "           status qmpstatus lock: %s\n" "$state"
    if node_ssh "systemctl is-active --quiet wol-listener" 2>/dev/null; then
        printf "  ${GREEN}✓${NC} wol-listener active on %s\n" "$PROXMOX_HOST"
    else
        printf "  ${RED}✗${NC} wol-listener not active on %s (tools/wol/provision.sh listener)\n" "$PROXMOX_HOST"
    fi
}

# ── verify ────────────────────────────────────────────────────────────

# Three packets one second apart, the same cadence as the daemon. SO_BROADCAST so that a
# broadcast address works as well as a unicast one.
send_magic() {
    local mac="$1" address="$2" port="$3" secureon="${4:-}"
    python3 - "$mac" "$address" "$port" "$secureon" <<'PY'
import socket, sys, time
mac, address, port, secureon = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
payload = b"\xff" * 6 + bytes.fromhex(mac.replace(":", "")) * 16
if secureon:
    pw = bytes.fromhex(secureon.replace(":", ""))
    if len(pw) != 6:
        sys.exit("WOL_SECUREON must be 6 bytes (12 hex digits)")
    payload += pw
s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
s.setsockopt(socket.SOL_SOCKET, socket.SO_BROADCAST, 1)
for i in range(3):
    s.sendto(payload, (address, port))
    print(f"  → sent {len(payload)}-byte magic packet for {mac} to {address}:{port} ({i + 1}/3)")
    if i < 2:
        time.sleep(1)
PY
}

cmd_verify() {
    local address="${1:-}" port="${2:-9}"
    [ -n "$address" ] || die "usage: $0 verify <address> [port]   (relay path: the node's lab IP, ${PROXMOX_LAB_IP:-$PROXMOX_HOST})"
    require_api
    local vmid mac state timeout waited=0
    vmid=$(target_vmid)
    mac=$(vm_mac "$vmid")
    [ -n "$mac" ] || die "VM $vmid has no MAC on net0"
    state=$(vm_state "$vmid")
    if [ "${state%% *}" = "running" ] && [[ "$state" != *" paused "* ]] && [[ "$state" != *" suspended "* ]]; then
        die "VM $vmid is already up ($state). Put it to sleep first: $0 sleep"
    fi
    timeout="${WOL_VERIFY_TIMEOUT:-120}"

    echo "Waking VM $vmid ($WOL_TARGET_NAME, $mac), currently: $state"
    send_magic "$mac" "$address" "$port" "${WOL_SECUREON:-}"

    while [ "$waited" -lt "$timeout" ]; do
        state=$(vm_state "$vmid")
        if [ "${state%% *}" = "running" ] && [[ "$state" == *" running "* ]]; then
            break
        fi
        sleep 2
        waited=$((waited + 2))
    done
    if [[ "$state" != "running running "* ]]; then
        printf "${RED}✗${NC} VM %s still %s after %ss. Check: %s listener-log\n" "$vmid" "$state" "$timeout" "$0"
        exit 1
    fi
    printf "  ${GREEN}✓${NC} VM %s running after %ss\n" "$vmid" "$waited"

    if [ -n "${WOL_TARGET_IP:-}" ]; then
        while [ "$waited" -lt "$timeout" ]; do
            if ping -c 1 -t 1 "$WOL_TARGET_IP" >/dev/null 2>&1; then
                printf "  ${GREEN}✓${NC} %s answers ping after %ss\n" "$WOL_TARGET_IP" "$waited"
                return 0
            fi
            sleep 1
            waited=$((waited + 1))
        done
        printf "${YELLOW}!${NC} VM is running but %s did not answer ping within %ss\n" "$WOL_TARGET_IP" "$timeout"
        exit 1
    fi
}

cmd_listener_log() {
    require_vars PROXMOX_HOST
    if [ "${1:-}" = "-f" ]; then
        node_ssh "journalctl -u wol-listener -n 50 -f --no-pager -o short-iso"
    else
        node_ssh "journalctl -u wol-listener -n 50 --no-pager -o short-iso"
    fi
}

case "${1:-}" in
    sleep) cmd_sleep "${2:-}" ;;
    status) cmd_status ;;
    verify) cmd_verify "${2:-}" "${3:-}" ;;
    listener-log) cmd_listener_log "${2:-}" ;;
    *)
        echo "Usage: $0 {sleep [--hibernate]|status|verify <address> [port]|listener-log [-f]}"
        echo ""
        echo "  sleep         Shut the target VM down, or hibernate it to disk with --hibernate"
        echo "  status        Target VM state and MAC, and whether the listener is running"
        echo "  verify        Send a magic packet from this Mac and wait for the VM to come up"
        echo "  listener-log  The listener's last 50 log lines from the node (-f to follow)"
        exit 1
        ;;
esac
