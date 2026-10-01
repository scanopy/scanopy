#!/bin/bash
# ══════════════════════════════════════════════════════════════════════
# Reconcile the SSH lab's macvlan links, addresses and port-22 redirect.
#
# Installed to /usr/local/bin/ssh-lab-network-up.sh by setup.sh and run by
# ssh-lab-network.service at every boot and on every deploy.
#
# ── Coexists with the SNMP lab on the same VM ─────────────────────────
# tools/snmp/lxc/lab-network-up.sh owns every link named mv-snmp* and deletes
# any mv-snmp* link it has no device for. This script owns mv-ssh* and touches
# nothing else, so neither can delete the other's links. Neither lab uses a
# netplan file for its links: netplan cannot create a macvlan, and the SNMP
# setup deletes /etc/netplan/60-snmp-test.yaml on every deploy.
#
# ── eth0 is out of bounds ─────────────────────────────────────────────
# $IFACE is read as the macvlan parent and never written. The VM's management
# address and DHCP lease live on it.
#
# ── Reconciled in place, not rebuilt ──────────────────────────────────
# `ip link add ... type macvlan` mints a random MAC. Rebuilding the links on
# every deploy would give every target a new MAC, and Scanopy would record a
# new host for each one.
# ══════════════════════════════════════════════════════════════════════
set -euo pipefail

ENV_FILE="${SSH_LAB_NETWORK_ENV:-/etc/ssh-test/lab-network.env}"
if [ ! -r "$ENV_FILE" ]; then
    echo "ERROR: $ENV_FILE is missing. Deploy with 'make ssh-lab-deploy'." >&2
    exit 1
fi
# IFACE, CIDR, BACKEND_PORT and HOSTS, written by setup.sh from targets.env.
# shellcheck source=/dev/null
. "$ENV_FILE"

# ── 1. Drop mv-ssh* links the target list does not account for ────────
while read -r link; do
    idx=${link#mv-ssh}
    keep=no
    case $idx in
        '' | *[!0-9]*) ;;
        *)
            if [ "$link" = "mv-ssh$((10#$idx))" ] && [ "$((10#$idx))" -lt "${#HOSTS[@]}" ]; then
                keep=yes
            fi
            ;;
    esac
    [ "$keep" = yes ] && continue
    ip link del "$link"
    echo "  Removed $link (no target at that position)"
done < <(ip -o link show | awk -F': ' '{print $2}' | sed 's/@.*//' | grep '^mv-ssh' || true)

# ── 2. Drop addresses that are not the link's own ─────────────────────
for i in "${!HOSTS[@]}"; do
    mvname="mv-ssh${i}"
    ip link show "$mvname" &>/dev/null || continue
    while read -r addr; do
        [ "$addr" = "${HOSTS[$i]}/$CIDR" ] && continue
        ip addr del "$addr" dev "$mvname"
        echo "  Removed $addr from $mvname"
    done < <(ip -4 -o addr show dev "$mvname" | awk '{print $4}')
done

# ── 3. Create and address what is missing ─────────────────────────────
for i in "${!HOSTS[@]}"; do
    ip="${HOSTS[$i]}"
    mvname="mv-ssh${i}"
    if ! ip link show "$mvname" &>/dev/null; then
        ip link add "$mvname" link "$IFACE" type macvlan mode bridge
        echo "  Created $mvname"
    fi
    ip link set "$mvname" up
    # Without these the parent answers ARP for every lab address with its own MAC, and every
    # target lands on one host record. The SNMP lab sets the conf/all knobs too; setting them per
    # link here keeps this unit correct on a VM where that file was never applied.
    sysctl -qw "net.ipv4.conf.${mvname}.arp_ignore=1" "net.ipv4.conf.${mvname}.arp_announce=2"
    if ! ip -4 -o addr show dev "$mvname" | awk '{print $4}' | grep -qx "$ip/$CIDR"; then
        ip addr add "$ip/$CIDR" dev "$mvname"
        echo "  Addressed $mvname ($ip)"
    fi
done

# ── 4. Redirect port 22 on each lab address to that target's sshd ─────
# The VM's main sshd listens on 0.0.0.0:22. Linux refuses to bind <lab-ip>:22 while a wildcard
# listener holds the port, so each target sshd listens on <lab-ip>:$BACKEND_PORT and this DNAT
# rule sends <lab-ip>:22 there. The rule matches only the lab addresses, so 192.168.4.21:22 still
# reaches the main sshd untouched. The table is ours alone: declaring, deleting and re-declaring
# it in one `nft -f` run replaces it atomically without touching any other table.
rules=""
for ip in "${HOSTS[@]}"; do
    rules+="        ip daddr ${ip} tcp dport 22 dnat to ${ip}:${BACKEND_PORT}"$'\n'
done
nft -f - <<EOF
table ip scanopy_ssh_lab {}
delete table ip scanopy_ssh_lab
table ip scanopy_ssh_lab {
    chain prerouting {
        type nat hook prerouting priority dstnat; policy accept;
${rules}    }
}
EOF
echo "  Port 22 redirect installed for ${#HOSTS[@]} address(es)"
