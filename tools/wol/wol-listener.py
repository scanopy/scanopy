#!/usr/bin/env python3
"""Wake-on-LAN listener for the Scanopy lab's Proxmox node.

Proxmox has no way to wake a guest VM from a magic packet: `pvenode wakeonlan` sends a packet to
wake a *node*, and a stopped guest has no NIC listening. This stands in for the guest's NIC. It
listens for magic packets on UDP, logs each one, finds the VM whose network card carries the
target MAC, and starts or resumes it.

Written from the AMD Magic Packet format, independently of Scanopy's sender, so a daemon run
against it tests whether the two agree on the wire format:

    6 bytes   0xFF x 6                     synchronization stream
    96 bytes  target MAC x 16
    6 bytes   SecureOn password            optional

A bare packet is 102 bytes, one with SecureOn 108. Real NICs scan the whole frame for the sync
stream followed by 16 repetitions, so a magic sequence embedded anywhere in a larger payload is
accepted too.

Standard library only. Runs as root on the Proxmox node (port 9 is privileged, and `qm` needs
root). Logs go to stdout, which systemd sends to the journal: `journalctl -u wol-listener`.
"""

from __future__ import annotations

import argparse
import json
import logging
import re
import selectors
import socket
import subprocess
import sys
import time

SYNC = b"\xff" * 6
MAC_REPEATS = 16
SECUREON_LEN = 6
MAC_RE = re.compile(r"=([0-9A-Fa-f]{2}(?::[0-9A-Fa-f]{2}){5})")
NET_LINE_RE = re.compile(r"^net\d+:\s*(.*)$")

log = logging.getLogger("wol-listener")


def format_mac(raw: bytes) -> str:
    return ":".join(f"{b:02x}" for b in raw)


def find_magic(payload: bytes) -> list[tuple[str, bytes | None]]:
    """Every magic sequence in the payload, as (mac, secureon_password_or_None).

    The sync stream is searched for at every offset, so a MAC that itself starts with 0xFF
    bytes (a longer run of 0xFF before the repetitions) is still found. A SecureOn password is
    reported only when exactly six bytes follow the sixteenth repetition, which is the 108-byte
    form; trailing bytes in a larger payload are not a password.
    """
    found = []
    i = payload.find(SYNC)
    while i != -1:
        start = i + len(SYNC)
        mac = payload[start : start + 6]
        end = start + 6 * MAC_REPEATS
        if len(mac) == 6 and mac != SYNC and payload[start:end] == mac * MAC_REPEATS:
            tail = payload[end:]
            secureon = tail if len(tail) == SECUREON_LEN else None
            found.append((format_mac(mac), secureon))
            i = payload.find(SYNC, end)
        else:
            i = payload.find(SYNC, i + 1)
    return found


def packet_form(size: int, offset_ok: bool) -> str:
    if offset_ok and size == 102:
        return "standard"
    if offset_ok and size == 108:
        return "secureon"
    return "embedded"


# ── MAC to VMID ──────────────────────────────────────────────────────


def run(cmd: list[str]) -> str:
    return subprocess.run(cmd, check=True, capture_output=True, text=True, timeout=30).stdout


def pool_vmids(pool: str) -> list[int]:
    """QEMU VM ids in a Proxmox pool.

    Tries the API path first (two shapes, older and newer PVE), then the pool line in
    /etc/pve/user.cfg, which every PVE version keeps as `pool:<id>:<comment>:<vmids>:<storage>`.
    """
    for cmd in (
        ["pvesh", "get", f"/pools/{pool}", "--output-format", "json"],
        ["pvesh", "get", "/pools", "--poolid", pool, "--output-format", "json"],
    ):
        try:
            data = json.loads(run(cmd))
        except (OSError, subprocess.SubprocessError, ValueError):
            continue
        entries = data if isinstance(data, list) else [data]
        vmids = [
            int(m["vmid"])
            for entry in entries
            for m in entry.get("members", [])
            if m.get("type") == "qemu" and "vmid" in m
        ]
        if vmids:
            return vmids
    try:
        with open("/etc/pve/user.cfg", encoding="utf-8") as f:
            for line in f:
                parts = line.strip().split(":")
                if len(parts) >= 4 and parts[0] == "pool" and parts[1] == pool:
                    return [int(v) for v in parts[3].split(",") if v.strip().isdigit()]
    except OSError:
        pass
    return []


def vm_macs(vmid: int) -> list[str]:
    try:
        config = run(["qm", "config", str(vmid)])
    except (OSError, subprocess.SubprocessError) as e:
        log.warning("qm config %s failed: %s", vmid, e)
        return []
    macs = []
    for line in config.splitlines():
        m = NET_LINE_RE.match(line)
        if m:
            macs.extend(mac.lower() for mac in MAC_RE.findall(m.group(1)))
    return macs


def load_map_file(path: str) -> dict[str, int]:
    """`<mac> <vmid>` per line; `#` starts a comment."""
    mapping = {}
    with open(path, encoding="utf-8") as f:
        for lineno, line in enumerate(f, 1):
            line = line.split("#", 1)[0].strip()
            if not line:
                continue
            parts = line.split()
            if len(parts) != 2 or not parts[1].isdigit():
                raise ValueError(f"{path}:{lineno}: expected '<mac> <vmid>', got {line!r}")
            mapping[parts[0].lower().replace("-", ":")] = int(parts[1])
    return mapping


class VmIndex:
    """MAC to VMID, from a static map file or from the pool's VM configs.

    The pool index is rebuilt on a miss, at most once per `refresh_interval` seconds, so a VM
    added to the pool after startup is found without restarting the listener.
    """

    def __init__(self, pool: str | None, map_file: str | None, refresh_interval: float = 10.0):
        self.pool = pool
        self.map_file = map_file
        self.refresh_interval = refresh_interval
        self.by_mac: dict[str, int] = {}
        self.last_refresh = 0.0
        self.refresh()

    def refresh(self) -> None:
        self.last_refresh = time.monotonic()
        if self.map_file:
            self.by_mac = load_map_file(self.map_file)
        else:
            mapping = {}
            for vmid in pool_vmids(self.pool):
                for mac in vm_macs(vmid):
                    mapping[mac] = vmid
            self.by_mac = mapping
        log.info(
            "index: %d MAC(s) from %s: %s",
            len(self.by_mac),
            self.map_file or f"pool {self.pool}",
            ", ".join(f"{mac}->{vmid}" for mac, vmid in sorted(self.by_mac.items())) or "none",
        )

    def lookup(self, mac: str) -> int | None:
        if mac not in self.by_mac and time.monotonic() - self.last_refresh >= self.refresh_interval:
            self.refresh()
        return self.by_mac.get(mac)


# ── Waking a VM ──────────────────────────────────────────────────────


def vm_status(vmid: int) -> dict[str, str]:
    out = run(["qm", "status", str(vmid), "--verbose"])
    status = {}
    for line in out.splitlines():
        key, sep, value = line.partition(":")
        if sep and not line.startswith(" "):
            status[key.strip()] = value.strip()
    return status


def wake(vmid: int, dry_run: bool) -> None:
    """Start a stopped VM, which also resumes one hibernated with `qm suspend --todisk`, or
    resume one paused in memory with `qm suspend`. A running VM is left alone."""
    try:
        status = vm_status(vmid)
    except (OSError, subprocess.SubprocessError) as e:
        log.error("vm %s: qm status failed: %s", vmid, e)
        return
    state = status.get("status", "unknown")
    qmp = status.get("qmpstatus", "")
    if state == "stopped":
        cmd = ["qm", "start", str(vmid)]
    elif state == "running" and qmp in ("paused", "suspended"):
        cmd = ["qm", "resume", str(vmid)]
    else:
        log.info("vm %s: already %s%s, nothing to do", vmid, state, f" ({qmp})" if qmp else "")
        return
    if dry_run:
        log.info("vm %s: %s; dry run, would run: %s", vmid, state, " ".join(cmd))
        return
    log.info("vm %s: %s; running: %s", vmid, state, " ".join(cmd))
    try:
        run(cmd)
        log.info("vm %s: %s succeeded", vmid, cmd[1])
    except subprocess.CalledProcessError as e:
        log.error("vm %s: %s failed: %s", vmid, cmd[1], (e.stderr or "").strip())
    except (OSError, subprocess.SubprocessError) as e:
        log.error("vm %s: %s failed: %s", vmid, cmd[1], e)


# ── Main loop ────────────────────────────────────────────────────────


def parse_args(argv: list[str]) -> argparse.Namespace:
    p = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    p.add_argument("--bind", default="0.0.0.0", help="address to listen on (default 0.0.0.0)")
    p.add_argument(
        "--port",
        type=int,
        action="append",
        help="UDP port to listen on; repeat for several (default 9)",
    )
    source = p.add_mutually_exclusive_group()
    source.add_argument("--pool", default="scanopy-lab", help="Proxmox pool to map MACs from")
    source.add_argument("--map-file", help="static '<mac> <vmid>' map instead of the pool")
    p.add_argument("--dry-run", action="store_true", help="log what would be started, start nothing")
    args = p.parse_args(argv)
    args.port = args.port or [9]
    return args


def main(argv: list[str]) -> int:
    logging.basicConfig(
        level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s", stream=sys.stdout
    )
    args = parse_args(argv)
    index = VmIndex(None if args.map_file else args.pool, args.map_file)

    sel = selectors.DefaultSelector()
    for port in args.port:
        sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        sock.bind((args.bind, port))
        sel.register(sock, selectors.EVENT_READ)
        log.info("listening on udp %s:%d%s", args.bind, port, " (dry run)" if args.dry_run else "")

    while True:
        for key, _ in sel.select():
            sock = key.fileobj
            payload, (src_ip, src_port) = sock.recvfrom(65535)
            local_port = sock.getsockname()[1]
            hits = find_magic(payload)
            if not hits:
                log.info(
                    "packet from %s:%d to port %d, %d bytes: no magic sequence",
                    src_ip, src_port, local_port, len(payload),
                )
                continue
            at_start = payload.startswith(SYNC)
            for mac, secureon in hits:
                vmid = index.lookup(mac)
                log.info(
                    "packet from %s:%d to port %d, %d bytes, form=%s, mac=%s, secureon=%s, vm=%s",
                    src_ip, src_port, local_port, len(payload),
                    packet_form(len(payload), at_start and len(hits) == 1),
                    mac,
                    format_mac(secureon) if secureon else "no",
                    vmid if vmid is not None else "unknown",
                )
                if vmid is None:
                    continue
                # No time-based repeat suppression: packets are handled one at a time and `wake` reads
                # the VM's state first, so the daemon's second and third packets find it running and do
                # nothing, while a packet after a later shutdown still starts it.
                wake(vmid, args.dry_run)


if __name__ == "__main__":
    try:
        sys.exit(main(sys.argv[1:]))
    except KeyboardInterrupt:
        sys.exit(0)
