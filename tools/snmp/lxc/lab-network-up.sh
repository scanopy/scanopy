#!/bin/bash
# ══════════════════════════════════════════════════════════════════════
# Reconcile the lab's macvlan links and addresses to the current device list.
#
# Installed to /usr/local/bin/snmp-lab-network-up.sh by lxc/setup.sh and run by
# snmp-lab-network.service, at every boot and on every deploy.
#
# This converges to the current device list rather than accumulating toward every
# address the lab has ever been asked for. The earlier version only ever added —
# no `ip addr del`, no `ip link del` — so a renumbering left the previous range
# configured alongside the new one, on the same links. One interface answers ARP
# for every address it holds with one MAC, and two addresses behind one MAC is
# what merges distinct devices onto a single host record.
#
# ── eth0 is out of bounds ─────────────────────────────────────────────
# Every mutation below names an interface matching mv-snmp*. $IFACE is read as
# the macvlan parent and never written. The VM's durable management address and
# its DHCP lease live on it, and losing them locks everyone out of the box.
#
# ── Reconciled in place, not rebuilt ──────────────────────────────────
# Deleting and recreating every link each run would be simpler to reason about,
# and the agents would survive it — `Requires=`/`After=snmp-lab-network.service`
# restarts each one when this unit restarts. But `ip link add ... type macvlan`
# mints a fresh random MAC, so a rebuild would give all 31 simulated devices new
# MACs on every deploy. Against a product that matches hosts on MAC that invents
# a lab's worth of new hosts and orphans the old ones each time, which is worse
# than the accumulation it would fix.
#
# ── IPv4 only ─────────────────────────────────────────────────────────
# The links pick up SLAAC global addresses from the segment's router
# advertisements. Those are the kernel's to manage; deleting one just invites it
# straight back on the next RA.
# ══════════════════════════════════════════════════════════════════════
set -euo pipefail

# IFACE, CIDR and HOSTS, written by setup.sh from the generated lab.env so this
# script carries no device knowledge of its own.
ENV_FILE="${LAB_NETWORK_ENV:-/etc/snmp-test/lab-network.env}"
if [ ! -r "$ENV_FILE" ]; then
    echo "ERROR: $ENV_FILE is missing. Deploy with 'make snmp-deploy'." >&2
    exit 1
fi
# shellcheck source=/dev/null
. "$ENV_FILE"

# Device i lives on mv-snmp<i> holding HOSTS[i]/CIDR and nothing else — the same
# positional mapping every lab.env array uses.

# ── 1. Drop links the current device list does not account for ────────
# Deleting the link takes its addresses with it, so this covers a lab that has
# shrunk as well as one left with surplus links from an earlier, longer list.
while read -r link; do
    idx=${link#mv-snmp}
    # Anything that is not exactly mv-snmp<n> for an n the device list reaches is
    # not ours to keep. The re-formed comparison rejects a padded index, which
    # would otherwise pass the range test under a different name.
    case $idx in
        '' | *[!0-9]*) keep=no ;;
        *)
            if [ "$link" = "mv-snmp$((10#$idx))" ] && [ "$((10#$idx))" -lt "${#HOSTS[@]}" ]; then
                keep=yes
            else
                keep=no
            fi
            ;;
    esac
    if [ "$keep" = yes ]; then
        continue
    fi
    ip link del "$link"
    echo "  Removed $link (no device at that position)"
done < <(ip -o link show | awk -F': ' '{print $2}' | sed 's/@.*//' | grep '^mv-snmp' || true)

# ── 2. Drop addresses that are not the link's own ─────────────────────
# Separate from the create pass below so a renumber never tries to add an address
# that is still configured on a different link. An address at the wrong prefix
# length does not match either, and is corrected the same way.
for i in "${!HOSTS[@]}"; do
    mvname="mv-snmp${i}"
    if ! ip link show "$mvname" &>/dev/null; then
        continue
    fi
    while read -r addr; do
        if [ "$addr" = "${HOSTS[$i]}/$CIDR" ]; then
            continue
        fi
        ip addr del "$addr" dev "$mvname"
        echo "  Removed $addr from $mvname"
    done < <(ip -4 -o addr show dev "$mvname" | awk '{print $4}')
done

# ── 3. Create and address what is missing ─────────────────────────────
for i in "${!HOSTS[@]}"; do
    ip="${HOSTS[$i]}"
    mvname="mv-snmp${i}"
    if ! ip link show "$mvname" &>/dev/null; then
        ip link add "$mvname" link "$IFACE" type macvlan mode bridge
        echo "  Created $mvname"
    fi
    ip link set "$mvname" up
    # Scope ARP to the interface that owns the address. At the kernel default the
    # box answers an ARP request for any local address from any interface, so
    # $IFACE replies for every lab address with its own MAC and the whole lab
    # collapses onto one host record.
    #
    # setup.sh's /etc/sysctl.d/60-snmp-lab-arp.conf sets the conf/all knobs, and
    # the kernel takes the max of conf/all and conf/<interface>, so that already
    # covers these links. Setting them per interface here too costs one syscall
    # and makes the unit correct on a box where that file was never applied —
    # the failure it guards against is silent, and only visible as merged hosts
    # in a scan days later.
    sysctl -qw "net.ipv4.conf.${mvname}.arp_ignore=1" "net.ipv4.conf.${mvname}.arp_announce=2"
    if ! ip -4 -o addr show dev "$mvname" | awk '{print $4}' | grep -qx "$ip/$CIDR"; then
        ip addr add "$ip/$CIDR" dev "$mvname"
        echo "  Addressed $mvname ($ip)"
    fi
done
