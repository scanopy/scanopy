#!/bin/bash
set -euo pipefail

# ══════════════════════════════════════════════════════════════════════
# Wake-on-LAN Test Environment: provision the Proxmox side. Runs on the Mac.
#
# Usage: tools/wol/provision.sh [all|target|daemon-vm|listener]
#
#   target     clone TEMPLATE_VMID into pool $LAB_POOL as $WOL_TARGET_NAME, attach net0 to
#              LAB_BRIDGE, start it once. Skips the clone if the VM already exists.
#   daemon-vm  the same for scanopy-wol-daemon, the lab VM a second daemon runs in for the
#              directed-broadcast path (WOL-TEST-ENV.md, path 2)
#   listener   install wol-listener.py and its unit on the node over SSH and (re)start it
#   all        target, then listener (the default)
#
# Settings: see proxmox-api.sh, plus:
#   WOL_TARGET_IPCONFIG  cloud-init ipconfig0 for the target (default ip=dhcp)
#   WOL_DAEMON_IPCONFIG  cloud-init ipconfig0 for the daemon VM (default ip=dhcp)
#   WOL_STORAGE          storage for the full clone's disks (default: the template's)
# ══════════════════════════════════════════════════════════════════════

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=proxmox-api.sh
. "$SCRIPT_DIR/proxmox-api.sh"

REMOTE_STAGING="/root/scanopy-wol"

# clone_vm NAME IPCONFIG: idempotent clone + configure + first start. Prints the VMID. Runs in a
# command substitution, so callers run require_api first: PROXMOX_NODE set in here is lost.
clone_vm() {
    local name="$1" ipconfig="$2"
    require_api
    require_vars TEMPLATE_VMID LAB_BRIDGE

    local vmid
    vmid=$(find_vmid "$name")
    if [ -n "$vmid" ]; then
        echo "  → $name already exists as VM $vmid in pool $LAB_POOL; not cloning" >&2
    else
        vmid=$(pve GET /cluster/nextid | jq -r '.data')
        echo "  → cloning template $TEMPLATE_VMID to VM $vmid ($name) in pool $LAB_POOL" >&2
        local args=("newid=$vmid" "name=$name" "pool=$LAB_POOL" "full=1")
        [ -n "${WOL_STORAGE:-}" ] && args+=("storage=$WOL_STORAGE")
        local upid
        upid=$(pve POST "/nodes/${PROXMOX_NODE}/qemu/${TEMPLATE_VMID}/clone" "${args[@]}" | jq -r '.data')
        pve_wait_task "$upid"
    fi

    # Keep the MAC the clone was given: rewrite only the bridge in net0. Setting net0 without
    # the MAC would mint a new one, and Scanopy would see a new host.
    local net0
    net0=$(pve GET "/nodes/${PROXMOX_NODE}/qemu/${vmid}/config" | jq -r '.data.net0 // ""')
    [ -n "$net0" ] || die "VM $vmid has no net0; the template needs one network device"
    if [[ "$net0" == *bridge=* ]]; then
        net0=$(sed -E "s/bridge=[^,]*/bridge=${LAB_BRIDGE}/" <<< "$net0")
    else
        net0="${net0},bridge=${LAB_BRIDGE}"
    fi
    echo "  → net0=$net0 ipconfig0=$ipconfig" >&2
    pve PUT "/nodes/${PROXMOX_NODE}/qemu/${vmid}/config" "net0=$net0" "ipconfig0=$ipconfig" >/dev/null

    if [ "$(vm_state "$vmid" | awk '{print $1}')" != "running" ]; then
        echo "  → starting VM $vmid" >&2
        pve_wait_task "$(pve POST "/nodes/${PROXMOX_NODE}/qemu/${vmid}/status/start" | jq -r '.data')"
    fi
    echo "$vmid"
}

cmd_target() {
    echo "Provisioning WoL target VM..."
    require_api
    local vmid
    vmid=$(clone_vm "$WOL_TARGET_NAME" "${WOL_TARGET_IPCONFIG:-ip=dhcp}")
    printf "${GREEN}✓${NC} %s is VM %s, MAC %s, state %s\n" \
        "$WOL_TARGET_NAME" "$vmid" "$(vm_mac "$vmid")" "$(vm_state "$vmid")"
}

cmd_daemon_vm() {
    echo "Provisioning WoL daemon VM..."
    require_api
    local vmid
    vmid=$(clone_vm scanopy-wol-daemon "${WOL_DAEMON_IPCONFIG:-ip=dhcp}")
    printf "${GREEN}✓${NC} scanopy-wol-daemon is VM %s, MAC %s, state %s\n" \
        "$vmid" "$(vm_mac "$vmid")" "$(vm_state "$vmid")"
    echo "  Install a Scanopy daemon in it the usual way; see WOL-TEST-ENV.md, path 2."
}

# Clears the staging directory first, for the same reason the SNMP deploy does: scp into an
# existing directory can leave a stale copy that is then what gets installed.
cmd_listener() {
    require_vars PROXMOX_HOST
    echo "Installing wol-listener on root@${PROXMOX_HOST}..."
    node_ssh "rm -rf ${REMOTE_STAGING} && mkdir -p ${REMOTE_STAGING}"
    node_scp "$SCRIPT_DIR/wol-listener.py" "$SCRIPT_DIR/wol-listener.service" \
        "root@${PROXMOX_HOST}:${REMOTE_STAGING}/"
    node_ssh bash -s -- "$REMOTE_STAGING" <<'REMOTE'
set -euo pipefail
staging="$1"
python3 -m py_compile "$staging/wol-listener.py"
install -d -m 755 /usr/local/lib/scanopy-lab
install -m 755 "$staging/wol-listener.py" /usr/local/lib/scanopy-lab/wol-listener.py
install -m 644 "$staging/wol-listener.service" /etc/systemd/system/wol-listener.service
systemctl daemon-reload
systemctl enable --quiet wol-listener.service
systemctl restart wol-listener.service
sleep 2
systemctl is-active wol-listener.service
journalctl -u wol-listener.service -n 5 --no-pager -o cat
REMOTE
    printf "${GREEN}✓${NC} wol-listener installed. Logs: tools/wol/wol-test-env.sh listener-log\n"
}

case "${1:-all}" in
    all)
        cmd_target
        cmd_listener
        ;;
    target) cmd_target ;;
    daemon-vm) cmd_daemon_vm ;;
    listener) cmd_listener ;;
    *)
        echo "Usage: $0 [all|target|daemon-vm|listener]"
        exit 1
        ;;
esac
