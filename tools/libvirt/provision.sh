#!/usr/bin/env bash
# ══════════════════════════════════════════════════════════════════════
# Builds the libvirt lab: a KVM host VM on the Proxmox lab node, with nested guests. Runs on the
# Mac. Idempotent: an existing VM is reused, and setup.sh only changes what is missing.
#
#   provision.sh          create the VM if needed, then run setup.sh network and setup.sh lab
#
# Settings: tools/libvirt/lab.env, tools/wol/proxmox-api.sh (the node and its SSH key), and
#   LIBVIRT_SSH_KEY   key the lab's scanopy and scanopy-ro users accept (default ~/.ssh/libvirt-lab;
#                     created when missing)
# ══════════════════════════════════════════════════════════════════════
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=../wol/proxmox-api.sh
. "$HERE/../wol/proxmox-api.sh"
# shellcheck source=lab.env
. "$HERE/lab.env"

VM_NAME=scanopy-libvirt-host
IMAGE=/var/lib/vz/template/iso/debian-12-genericcloud-amd64.qcow2
STORAGE="${LIBVIRT_STORAGE:-local-lvm}"
LIBVIRT_SSH_KEY="${LIBVIRT_SSH_KEY:-$HOME/.ssh/libvirt-lab}"

require_vars PROXMOX_HOST LAB_BRIDGE

vm_ssh() {
    ssh -i "$PROXMOX_SSH_KEY" -o ConnectTimeout=5 -o StrictHostKeyChecking=accept-new \
        -o UserKnownHostsFile="$HOME/.ssh/known_hosts_scanopy_lab" "debian@$HOST_IP" "$@"
}

wait_for_ssh() {
    local i
    for i in $(seq 1 90); do
        vm_ssh true 2>/dev/null && return 0
        sleep 5
    done
    die "the VM did not answer SSH at $HOST_IP"
}

[ -f "$LIBVIRT_SSH_KEY" ] || ssh-keygen -q -t ed25519 -N '' -C scanopy-libvirt-lab -f "$LIBVIRT_SSH_KEY"

vmid=$(node_ssh "qm list | awk '\$2 == \"$VM_NAME\" {print \$1}'")
if [ -z "$vmid" ]; then
    vmid=$(node_ssh "pvesh get /cluster/nextid")
    printf "${GREEN}→${NC} creating VM %s (%s)\n" "$vmid" "$VM_NAME"
    node_scp "${PROXMOX_SSH_KEY}.pub" "root@${PROXMOX_HOST}:/root/scanopy-libvirt-host.pub"
    # cpu=host passes VMX through for the nested guests. No firewall on net0, so Proxmox puts no
    # MAC filter on the guests' own MACs.
    node_ssh "set -e
        qm create $vmid --name $VM_NAME --pool $LAB_POOL --memory 3584 --cores 2 --cpu host \
            --net0 virtio,bridge=$LAB_BRIDGE --scsihw virtio-scsi-pci --serial0 socket --vga serial0
        qm importdisk $vmid $IMAGE $STORAGE >/dev/null
        qm set $vmid --scsi0 $STORAGE:vm-$vmid-disk-0 --boot order=scsi0 >/dev/null
        qm resize $vmid scsi0 24G
        qm set $vmid --ide2 $STORAGE:cloudinit --ciuser debian \
            --sshkeys /root/scanopy-libvirt-host.pub \
            --ipconfig0 ip=$HOST_IP/$PREFIX,gw=$GATEWAY --nameserver $GATEWAY >/dev/null
        qm start $vmid"
fi
printf "${GREEN}→${NC} VM %s, waiting for SSH at %s\n" "$vmid" "$HOST_IP"
wait_for_ssh

push() {
    vm_ssh "rm -rf /tmp/scanopy-libvirt && mkdir -p /tmp/scanopy-libvirt"
    scp -q -i "$PROXMOX_SSH_KEY" -o UserKnownHostsFile="$HOME/.ssh/known_hosts_scanopy_lab" \
        -r "$HERE/setup.sh" "$HERE/lab.env" "$HERE/guests" "${LIBVIRT_SSH_KEY}.pub" \
        "debian@$HOST_IP:/tmp/scanopy-libvirt/"
    if [ "$(basename "$LIBVIRT_SSH_KEY")" != libvirt-lab ]; then
        vm_ssh "mv /tmp/scanopy-libvirt/$(basename "$LIBVIRT_SSH_KEY").pub /tmp/scanopy-libvirt/libvirt-lab.pub"
    fi
}

push
if ! vm_ssh "ip link show br0" >/dev/null 2>&1; then
    vm_ssh "sudo bash /tmp/scanopy-libvirt/setup.sh network" || true
    sleep 10
    wait_for_ssh
    push
fi
vm_ssh "sudo bash /tmp/scanopy-libvirt/setup.sh lab"
printf "${GREEN}✓${NC} libvirt lab ready at %s (users scanopy, scanopy-ro; key %s)\n" "$HOST_IP" "$LIBVIRT_SSH_KEY"
