#!/usr/bin/env bash
# ══════════════════════════════════════════════════════════════════════
# Drives the libvirt lab from the Mac. See LIBVIRT-TEST-ENV.md.
#
#   libvirt-test-env.sh status            every domain's state, NICs and addresses by source
#   libvirt-test-env.sh scenario <name>   start-stopped | undefine | rename | reset
#
# Capturing parse fixtures goes through the daemon's own code instead:
#   make libvirt-lab-capture
# ══════════════════════════════════════════════════════════════════════
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lab.env
. "$HERE/lab.env"
LIBVIRT_SSH_KEY="${LIBVIRT_SSH_KEY:-$HOME/.ssh/libvirt-lab}"
PROXMOX_SSH_KEY="${PROXMOX_SSH_KEY:-$HOME/.ssh/proxmox-lab}"

as_scanopy() {
    ssh -i "$LIBVIRT_SSH_KEY" -o ConnectTimeout=5 \
        -o UserKnownHostsFile="$HOME/.ssh/known_hosts_scanopy_lab" "scanopy@$HOST_IP" "$@"
}
as_root() {
    ssh -i "$PROXMOX_SSH_KEY" -o ConnectTimeout=5 \
        -o UserKnownHostsFile="$HOME/.ssh/known_hosts_scanopy_lab" "debian@$HOST_IP" sudo "$@"
}

status() {
    as_scanopy 'v() { virsh -q -c qemu:///system "$@"; }
        v list --all
        for u in $(v list --all --uuid); do
            echo; echo "== $(v domname "$u") ($(v domstate "$u"))"
            v domiflist "$u" --inactive
            for s in lease arp agent; do
                echo "-- $s"; v domifaddr "$u" --source $s 2>&1 | sed "s/^/   /"
            done
        done'
}

scenario() {
    case "${1:-}" in
        # The reporter's stopped VMs come up: the MAC-only host gains its address.
        start-stopped) as_root virsh -q start kvm-stopped ;;
        # A domain removed from libvirt: its host is deleted after the next scan.
        undefine)
            as_root virsh -q destroy kvm-paused 2>/dev/null || true
            as_root virsh -q undefine kvm-paused
            ;;
        # A renamed domain keeps its UUID, so it stays the same host.
        rename)
            as_root virsh -q shutdown kvm-noagent
            sleep 15
            as_root virsh -q domrename kvm-noagent kvm-renamed
            as_root virsh -q start kvm-renamed
            ;;
        reset)
            as_root virsh -q destroy kvm-renamed 2>/dev/null || true
            as_root virsh -q undefine kvm-renamed 2>/dev/null || true
            as_root bash /tmp/scanopy-libvirt/setup.sh reset
            ;;
        *)
            echo "usage: $0 scenario start-stopped|undefine|rename|reset" >&2
            exit 2
            ;;
    esac
}

case "${1:-}" in
    status) status ;;
    scenario) scenario "${2:-}" ;;
    *)
        echo "usage: $0 status | scenario <name>" >&2
        exit 2
        ;;
esac
