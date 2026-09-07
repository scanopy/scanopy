#!/bin/sh
# Capture real `lldpcli -f json` output from the lldpd the daemon image ships.
#
# Runs INSIDE a container built from backend/Dockerfile.daemon, with NET_ADMIN. Two lldpd
# instances sit on the ends of a veth pair and exchange real LLDP; each is pinned to its
# own end with -I and has its own control socket, so they meet as two systems on a wire.
# A veth pair rather than a bridge, because Linux bridges do not forward the 802.1D
# reserved group address LLDP uses unless the host's group_fwd_mask says so — a dead end
# on a hosted runner. Both stay in the container's own network namespace: creating one
# (`ip netns add`) needs mounts, which NET_ADMIN alone does not grant.
#
# Writes into $OUT_DIR (default /out):
#   neighbors-empty.json  what `show neighbors details` emits before any frame arrives
#   neighbors.json        the same, once the peer has been heard
#   lldpd-version.txt     `lldpcli -v`
#
# The captures are then parsed by the repository's own parser (the ignored
# `live_capture_*` tests in daemon/discovery/service/lldpd.rs), so a shape change in a
# new Debian lldpd package breaks on our types, not on a jq path.
set -eu

OUT_DIR="${OUT_DIR:-/out}"
SOCK_A=/run/lldpd-a.socket
SOCK_B=/run/lldpd-b.socket

say() { echo "lldpd-live-capture: $*" >&2; }

mkdir -p "$OUT_DIR"
lldpcli -v > "$OUT_DIR/lldpd-version.txt"
say "lldpd version: $(cat "$OUT_DIR/lldpd-version.txt")"

ip link add cap0 type veth peer name cap1
ip addr add 198.51.100.1/30 dev cap0
ip addr add 198.51.100.2/30 dev cap1
ip link set cap0 up
ip link set cap1 up

# Each instance is pinned to its own interface and socket. -S names them so the captured
# chassis section carries a recognisable, stable system description.
lldpd -u "$SOCK_A" -I cap0 -C cap0 -S "lldpd-live-capture side A"

# Side B is not up yet, so this is the genuine zero-neighbour shape — the one the parser
# must treat as an authoritative empty table.
sleep 1
lldpcli -u "$SOCK_A" -f json show neighbors details > "$OUT_DIR/neighbors-empty.json"
say "captured empty shape: $(cat "$OUT_DIR/neighbors-empty.json")"

lldpd -u "$SOCK_B" -I cap1 -C cap1 -S "lldpd-live-capture side B"

# lldpd advertises on start, so the neighbour normally appears within a second or two;
# 60 s covers a slow runner plus one full re-advertisement interval.
tries=0
while :; do
    lldpcli -u "$SOCK_A" -f json show neighbors details > "$OUT_DIR/neighbors.json"
    if grep -q "cap0" "$OUT_DIR/neighbors.json"; then
        break
    fi
    tries=$((tries + 1))
    if [ "$tries" -ge 60 ]; then
        say "no neighbour after ${tries}s"
        say "side A log check: $(lldpcli -u "$SOCK_A" show statistics summary 2>&1 | tr '\n' ' ')"
        exit 1
    fi
    sleep 1
done

say "captured neighbour table after ${tries}s:"
cat "$OUT_DIR/neighbors.json" >&2
