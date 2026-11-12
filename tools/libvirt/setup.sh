#!/usr/bin/env bash
# ══════════════════════════════════════════════════════════════════════
# Builds the libvirt lab host. Runs as root on the KVM host VM (scanopy-libvirt-host); provision.sh
# copies this directory there and runs it. Idempotent: every step checks before it changes
# anything, and each domain is defined from the same template every run.
#
#   setup.sh network   bridge the VM's NIC into br0 and add br-iso, then reboot
#   setup.sh lab       packages, users, Docker, images and the eight domains
#   setup.sh reset     put every domain back in its baseline state
#
# See LIBVIRT-TEST-ENV.md for the domain table and what each one exercises.
# ══════════════════════════════════════════════════════════════════════
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lab.env
. "$HERE/lab.env"

IMAGES=/var/lib/libvirt/images
ALPINE_URL="https://dl-cdn.alpinelinux.org/alpine/v3.20/releases/cloud/nocloud_alpine-3.20.9-x86_64-bios-cloudinit-r0.qcow2"
ALPINE="$IMAGES/alpine-3.20-nocloud.qcow2"

log() { printf '→ %s\n' "$*"; }

# A fresh VM's first boot runs apt from cloud-init; apt here would find the lock held.
wait_for_cloud_init() { cloud-init status --wait >/dev/null 2>&1 || true; }

# ── network: br0 takes over the VM's NIC, keeping its address so SSH comes back ──────────────
network() {
    if ip link show br0 >/dev/null 2>&1; then
        log "br0 already up"
        return
    fi
    local nic mac
    nic=$(ip -o route get 1.1.1.1 | sed -n 's/.* dev \([^ ]*\).*/\1/p')
    mac=$(cat "/sys/class/net/$nic/address")
    log "bridging $nic into br0 at $HOST_IP/$PREFIX"
    wait_for_cloud_init
    # cloud-init would rewrite the NIC's config on the next boot.
    echo 'network: {config: disabled}' > /etc/cloud/cloud.cfg.d/99-disable-network-config.cfg
    rm -f /etc/netplan/50-cloud-init.yaml
    # The NIC is matched by MAC and keeps its name: cloud-init's file, removed above, was what
    # renamed it from its predictable name. br-iso has no port and no address, so a guest on it is
    # reached by nothing outside it.
    cat > /etc/netplan/60-libvirt-lab.yaml <<EOF
network:
  version: 2
  renderer: networkd
  ethernets:
    $nic:
      match:
        macaddress: "$mac"
      set-name: $nic
      dhcp4: false
  bridges:
    br0:
      interfaces: [$nic]
      addresses: [$HOST_IP/$PREFIX]
      routes:
        - to: default
          via: $GATEWAY
      nameservers:
        addresses: [$GATEWAY]
      parameters:
        stp: false
        forward-delay: 0
    br-iso:
      interfaces: []
      dhcp4: false
      parameters:
        stp: false
EOF
    chmod 600 /etc/netplan/60-libvirt-lab.yaml
    log "rebooting into the bridged network"
    systemctl reboot
}

# ── lab: everything else ───────────────────────────────────────────────────────────────────────
packages() {
    log "packages"
    wait_for_cloud_init
    apt-get update -qq
    DEBIAN_FRONTEND=noninteractive apt-get install -y -qq \
        libvirt-daemon-system libvirt-clients qemu-system-x86 qemu-utils \
        cloud-image-utils docker.io jq >/dev/null
    systemctl enable --now libvirtd docker >/dev/null
}

users() {
    log "users scanopy (libvirt group) and scanopy-ro (read-only socket only)"
    local user
    for user in scanopy scanopy-ro; do
        id "$user" >/dev/null 2>&1 || useradd -m -s /bin/bash "$user"
        install -d -m 700 -o "$user" -g "$user" "/home/$user/.ssh"
        install -m 600 -o "$user" -g "$user" "$HERE/libvirt-lab.pub" "/home/$user/.ssh/authorized_keys"
    done
    usermod -aG libvirt scanopy
}

networks() {
    log "libvirt networks"
    virsh -q net-autostart default >/dev/null
    virsh -q net-start default >/dev/null 2>&1 || true
}

docker_lab() {
    log "Docker on the KVM host: a bridge container and a macvlan container at $HOST_MACVLAN_IP"
    docker network inspect lab-lan >/dev/null 2>&1 ||
        docker network create -d macvlan --subnet "$LAN_CIDR" --gateway "$GATEWAY" \
            -o parent=br0 lab-lan >/dev/null
    docker inspect lab-web >/dev/null 2>&1 ||
        docker run -d --restart unless-stopped --name lab-web -p 8080:80 nginx:alpine >/dev/null
    docker inspect lab-mv >/dev/null 2>&1 ||
        docker run -d --restart unless-stopped --name lab-mv --network lab-lan \
            --ip "$HOST_MACVLAN_IP" nginx:alpine >/dev/null
}

images() {
    if [ ! -f "$ALPINE" ]; then
        log "downloading the Alpine cloud image"
        curl -fsSL -o "$ALPINE.part" "$ALPINE_URL"
        mv "$ALPINE.part" "$ALPINE"
    fi
}

# define NAME MAC_SUFFIX MEMORY NIC... : write the domain's overlay disk, its cloud-init seed and its
# definition. A NIC is `bridge:<br>` or `network:<net>`; the first gets MAC_SUFFIX, the next +1.
define() {
    local name="$1" suffix="$2" memory="$3"
    shift 3
    local disk="$IMAGES/$name.qcow2" seed="$IMAGES/$name-seed.iso"
    [ -f "$disk" ] || qemu-img create -q -f qcow2 -F qcow2 -b "$ALPINE" "$disk" 2G
    cloud-localds -N "$HERE/guests/$name.network.yaml" "$seed" \
        "$HERE/guests/$name.user.yaml" "$HERE/guests/meta-data.yaml"

    local nics="" i=0 nic
    for nic in "$@"; do
        local mac
        mac=$(printf '52:54:00:7a:00:%02x' $((suffix + i)))
        case "$nic" in
            bridge:*) nics+="<interface type='bridge'><source bridge='${nic#bridge:}'/><mac address='$mac'/><model type='virtio'/></interface>" ;;
            network:*) nics+="<interface type='network'><source network='${nic#network:}'/><mac address='$mac'/><model type='virtio'/></interface>" ;;
        esac
        i=$((i + 1))
    done
    # kvm-nochannel has no agent channel, so libvirt says the agent is not configured.
    local channel="<channel type='unix'><target type='virtio' name='org.qemu.guest_agent.0'/></channel>"
    [ "$name" = kvm-nochannel ] && channel=""
    # A redefinition keeps the domain's UUID, so its vm_id stays the same across resets.
    local uuid=""
    uuid=$(virsh -q domuuid "$name" 2>/dev/null) && uuid="<uuid>$uuid</uuid>"

    cat > "/tmp/$name.xml" <<EOF
<domain type='kvm'>
  <name>$name</name>
  $uuid
  <memory unit='MiB'>$memory</memory>
  <vcpu>1</vcpu>
  <os><type arch='x86_64' machine='pc'>hvm</type><boot dev='hd'/></os>
  <features><acpi/></features>
  <cpu mode='host-passthrough'/>
  <devices>
    <disk type='file' device='disk'><driver name='qemu' type='qcow2'/><source file='$disk'/><target dev='vda' bus='virtio'/></disk>
    <disk type='file' device='cdrom'><driver name='qemu' type='raw'/><source file='$seed'/><target dev='sda' bus='sata'/><readonly/></disk>
    $nics
    $channel
    <serial type='pty'/><console type='pty'/>
  </devices>
</domain>
EOF
    virsh -q define "/tmp/$name.xml" >/dev/null
    rm -f "/tmp/$name.xml"
}

domains() {
    log "domains"
    define kvm-noagent   3  128 bridge:br0
    define kvm-nochannel 4  128 bridge:br0
    define kvm-agent     5  160 bridge:br0 network:default
    define kvm-paused    7  128 bridge:br0
    define kvm-stopped   8  128 bridge:br0
    define kvm-docker    9  384 bridge:br0
    define kvm-nat       10 128 network:default
    define kvm-isolated  11 128 bridge:br-iso
}

# The baseline: everything running except kvm-stopped (never started) and kvm-paused (suspended).
baseline() {
    log "baseline states"
    local name
    for name in kvm-noagent kvm-nochannel kvm-agent kvm-docker kvm-nat kvm-isolated kvm-paused; do
        [ "$(virsh -q domstate "$name")" = "running" ] || [ "$(virsh -q domstate "$name")" = "paused" ] ||
            virsh -q start "$name" >/dev/null
    done
    [ "$(virsh -q domstate kvm-stopped)" = "shut off" ] || virsh -q destroy kvm-stopped >/dev/null
    [ "$(virsh -q domstate kvm-paused)" = "paused" ] || {
        # Let it boot first, so it pauses as a running guest would.
        sleep 20
        virsh -q suspend kvm-paused >/dev/null
    }
}

case "${1:-}" in
    network) network ;;
    lab)
        packages
        users
        networks
        docker_lab
        images
        domains
        baseline
        log "done"
        virsh list --all
        ;;
    reset)
        domains
        baseline
        virsh list --all
        ;;
    *)
        echo "usage: $0 network|lab|reset" >&2
        exit 2
        ;;
esac
