#!/bin/bash
set -euo pipefail

# ══════════════════════════════════════════════════════════════════════
# PROFINET DCP Test Environment — local macOS sim, start/stop/verify/status.
#
# Runs directly on this Mac, on the same interface (and so the same L2 segment) the
# daemon-under-test itself scans on. DCP is raw Ethernet and does not route — a sim on a remote
# VM is unreachable from a real scan run here regardless of IP connectivity, which is why this
# no longer deploys anywhere (see DCP-TEST-ENV.md for the history: it originally shared the SNMP
# lab's remote VM, the same way SNMP does, but SNMP is routable UDP and DCP isn't). No VM, no
# macvlan, no SSH — dcp-sim.py talks raw Ethernet directly via bpf_raw.py (macOS /dev/bpf*).
#
# Usage: tools/dcp/dcp-test-env.sh start|stop|verify|status
#
# Override via env: DCP_IFACE (default en0 — set this to whatever interface the installed
# daemon actually scans on), DCP_DEVICE_NAME (default scanopy-dcp-sim).
# ══════════════════════════════════════════════════════════════════════

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
IFACE="${DCP_IFACE:-en0}"
DEVICE_NAME="${DCP_DEVICE_NAME:-scanopy-dcp-sim}"
PID_FILE="/tmp/dcp-sim.pid"
LOG_FILE="/tmp/dcp-sim.log"

is_running() {
    # dcp-sim.py runs as root (sudo, for /dev/bpf* access), so a plain kill -0 from this
    # unprivileged script always fails with "Operation not permitted" even when it's alive —
    # signal-0 delivery is permission-checked too. Needs sudo here for the same reason cmd_stop
    # already used sudo kill.
    [ -f "$PID_FILE" ] && sudo kill -0 "$(cat "$PID_FILE")" 2>/dev/null
}

cmd_start() {
    if is_running; then
        echo "already running (pid $(cat "$PID_FILE")) — see $LOG_FILE"
        exit 0
    fi
    rm -f "$PID_FILE"
    echo "→ starting dcp-sim.py on ${IFACE} as '${DEVICE_NAME}' (needs sudo for /dev/bpf*)"
    sudo python3 "$SCRIPT_DIR/dcp-sim.py" "$IFACE" --name "$DEVICE_NAME" --pidfile "$PID_FILE" \
        > "$LOG_FILE" 2>&1 &
    disown
    for _ in $(seq 1 20); do
        [ -f "$PID_FILE" ] && break
        sleep 0.25
    done
    if is_running; then
        echo "started — pid $(cat "$PID_FILE"), log at $LOG_FILE"
    else
        echo "failed to start — see $LOG_FILE" >&2
        exit 1
    fi
}

cmd_stop() {
    if ! is_running; then
        echo "not running"
        rm -f "$PID_FILE"
        exit 0
    fi
    sudo kill "$(cat "$PID_FILE")"
    rm -f "$PID_FILE"
    echo "stopped"
}

cmd_verify() {
    echo "→ running the real protocol-level check from ${IFACE}"
    sudo python3 "$SCRIPT_DIR/dcp-verify.py" "$IFACE"
}

cmd_status() {
    if is_running; then
        echo "✓ running (pid $(cat "$PID_FILE"))"
    else
        echo "✗ not running"
        exit 1
    fi
}

case "${1:-}" in
    start) cmd_start ;;
    stop) cmd_stop ;;
    verify) cmd_verify ;;
    status) cmd_status ;;
    *)
        echo "Usage: $0 {start|stop|verify|status}"
        echo ""
        echo "  start  — launch dcp-sim.py in the background on \$DCP_IFACE (default en0)"
        echo "  stop   — stop it"
        echo "  verify — send one real Identify request from \$DCP_IFACE and confirm it answers"
        echo "  status — check whether it's currently running"
        exit 1
        ;;
esac
