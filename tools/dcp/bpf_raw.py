"""Minimal raw-Ethernet I/O on macOS via /dev/bpf*, for dcp-sim.py and dcp-verify.py.

macOS has no AF_PACKET (that's Linux-specific — see dcp-sim.py's own module doc for why the
original VM-hosted version used it). The macOS equivalent is a BPF device: open /dev/bpfN, bind it
to an interface with the BIOCSETIF ioctl, then read()/write() raw frames directly. This is the same
mechanism `backend/vendor/pnet_datalink/src/bpf.rs` uses for the real daemon — the ioctl request
codes and struct layouts below are cross-checked against that vendored source (which itself matches
/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/include/net/bpf.h) rather than reconstructed
from scratch, so this script and the real daemon are reading the exact same kernel interface.

Sets BIOCPROMISC. An earlier version of this didn't, reasoning that the real daemon's bpf.rs has
no promiscuous ioctl call anywhere on the macOS backend and it works fine in production — true,
but the wrong comparison: the daemon only ever *sends* DCP multicast (transmit isn't filtered by
promiscuous) and only ever *receives* the sim's *unicast* reply (always delivered to its own MAC
regardless of promiscuous mode). This sim is different — it has to *receive* an arbitrary
multicast Identify Request nobody addressed to it, and without promiscuous mode that's exactly
the traffic a NIC drops before a non-promiscuous BPF listener ever sees it. Confirmed live: a
`tcpdump -i en0 -e 'ether proto 0x8892'` (promiscuous by default) saw the Request that a
non-promiscuous sim never did — the identical bug class the original Linux/macvlan version hit
for the same underlying reason (see DCP-TEST-ENV.md's own history section on that).
"""

import fcntl
import os
import re
import select
import struct
import subprocess

_IF_NAMESIZE = 16
_SIZEOF_IFREQ = 32
_IOC_VOID = 0x20000000
_IOC_IN = 0x80000000
_IOC_OUT = 0x40000000
_IOCPARM_MASK = 0x1FFF


def _io(group: str, num: int) -> int:
    return _IOC_VOID | (ord(group) << 8) | num


def _iow(group: str, num: int, length: int) -> int:
    return _IOC_IN | ((length & _IOCPARM_MASK) << 16) | (ord(group) << 8) | num


def _iowr(group: str, num: int, length: int) -> int:
    return _IOC_IN | _IOC_OUT | ((length & _IOCPARM_MASK) << 16) | (ord(group) << 8) | num


_BIOCSBLEN = _iowr("B", 102, 4)  # set read buffer length — must precede BIOCSETIF
_BIOCPROMISC = _io("B", 105)  # accept frames not addressed to us — needed to see DCP's multicast
_BIOCSETIF = _iow("B", 108, _SIZEOF_IFREQ)  # bind the device to an interface
_BIOCIMMEDIATE = _iow("B", 112, 4)  # return from read as soon as a packet is available
_BIOCSHDRCMPLT = _iow("B", 117, 4)  # we supply the full L2 header ourselves; don't overwrite it

# struct bpf_hdr on 64-bit macOS: BPF_TIMEVAL is `timeval32` under __LP64__ (2x int32), then two
# u_int32 fields, then a u_short. Only the offsets up to bh_hdrlen matter here — bh_hdrlen itself
# tells us where the captured frame starts, so nothing downstream needs the struct's full,
# padded size.
_BPF_HDR_FIELDS = struct.Struct("<iiIIH")  # tv_sec, tv_usec, bh_caplen, bh_datalen, bh_hdrlen


def get_mac(interface: str) -> bytes:
    output = subprocess.check_output(["ifconfig", interface], text=True)
    match = re.search(r"ether ([0-9a-f:]{17})", output)
    if not match:
        raise RuntimeError(f"could not determine the MAC address of {interface}")
    return bytes.fromhex(match.group(1).replace(":", ""))


def open_bpf(interface: str, buffer_size: int = 4096) -> int:
    """Open the first free /dev/bpfN device and bind it to `interface`. Needs root."""
    fd = None
    last_error: OSError | None = None
    for i in range(64):
        try:
            fd = os.open(f"/dev/bpf{i}", os.O_RDWR)
            break
        except OSError as e:
            last_error = e
    if fd is None:
        raise OSError(
            f"no free /dev/bpfN device (need root, and one must be free): {last_error}"
        )

    fcntl.ioctl(fd, _BIOCSBLEN, struct.pack("=I", buffer_size))

    name_field = interface.encode("ascii")[: _IF_NAMESIZE - 1].ljust(_IF_NAMESIZE, b"\x00")
    ifreq = name_field + b"\x00" * (_SIZEOF_IFREQ - _IF_NAMESIZE)
    fcntl.ioctl(fd, _BIOCSETIF, ifreq)

    fcntl.ioctl(fd, _BIOCPROMISC)
    fcntl.ioctl(fd, _BIOCIMMEDIATE, struct.pack("=I", 1))
    fcntl.ioctl(fd, _BIOCSHDRCMPLT, struct.pack("=I", 1))
    return fd


def read_frame(fd: int, timeout_s: float | None) -> bytes | None:
    """Wait up to `timeout_s` (None = block forever) for a packet; return its raw bytes, or None
    on timeout. If more than one packet lands in a single underlying read(), only the first is
    returned — fine for a low-traffic test tool exchanging one Identify request/response at a
    time, not a general-purpose multi-packet BPF reader."""
    poller = select.poll()
    poller.register(fd, select.POLLIN)
    if not poller.poll(None if timeout_s is None else timeout_s * 1000):
        return None
    buf = os.read(fd, 4096)
    if len(buf) < _BPF_HDR_FIELDS.size:
        return None
    _tv_sec, _tv_usec, caplen, _datalen, hdrlen = _BPF_HDR_FIELDS.unpack_from(buf, 0)
    return buf[hdrlen : hdrlen + caplen]


def write_frame(fd: int, frame: bytes) -> None:
    os.write(fd, frame)
