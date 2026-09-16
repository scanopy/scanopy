#!/usr/bin/env python3
"""Send one PROFINET DCP Identify request and print whatever answers.

A protocol-level check independent of the daemon and of dcp-sim.py's own responder logic — the
same role `snmpget`/`snmpwalk` play in `tools/snmp/snmp-test-env.sh`'s `verify`: confirm the
fixture actually answers before trusting a full daemon scan against it. Run on the same host as
dcp-sim.py (raw Ethernet does not route, so this only proves anything on the sim's own L2
segment — see DCP-TEST-ENV.md).

    sudo ./dcp-verify.py en0
"""

import argparse
import secrets
import struct
import sys
import time

import bpf_raw

ETHERTYPE_PROFINET = 0x8892
DCP_IDENTIFY_MULTICAST = bytes.fromhex("010ecf000000")
FRAME_ID_DCP_IDENT_REQ = 0xFEFE
FRAME_ID_DCP_IDENT_RES = 0xFEFF
SERVICE_ID_IDENTIFY = 0x05
SERVICE_TYPE_REQUEST = 0x00
SERVICE_TYPE_RESPONSE_SUCCESS = 0x01
OPTION_DEVICE = 0x02
SUBOPTION_DEVICE_NAME_OF_STATION = 0x02


def build_identify_request(src_mac: bytes, xid: int) -> bytes:
    block = bytes([0xFF, 0xFF, 0, 0])  # All-Selector, no value
    dcp_header = struct.pack(">HBBIHH", FRAME_ID_DCP_IDENT_REQ, SERVICE_ID_IDENTIFY, SERVICE_TYPE_REQUEST, xid, 100, len(block))
    return DCP_IDENTIFY_MULTICAST + src_mac + struct.pack(">H", ETHERTYPE_PROFINET) + dcp_header + block


def parse_name_of_station(block: bytes):
    i = 0
    while i + 4 <= len(block):
        option, suboption, length = block[i], block[i + 1], struct.unpack(">H", block[i + 2 : i + 4])[0]
        value = block[i + 4 : i + 4 + length]
        if option == OPTION_DEVICE and suboption == SUBOPTION_DEVICE_NAME_OF_STATION:
            return value.decode(errors="replace")
        i += 4 + length + (length % 2)
    return None


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("interface")
    parser.add_argument("--timeout", type=float, default=3.0)
    args = parser.parse_args()

    own_mac = bpf_raw.get_mac(args.interface)
    fd = bpf_raw.open_bpf(args.interface)

    xid = secrets.randbits(24) | 0x0F000000
    bpf_raw.write_frame(fd, build_identify_request(own_mac, xid))
    print(f"sent Identify Request from {own_mac.hex(':')} (xid={xid:#x}), waiting {args.timeout}s...")

    deadline = time.monotonic() + args.timeout
    found = 0
    while time.monotonic() < deadline:
        frame = bpf_raw.read_frame(fd, timeout_s=max(0.0, deadline - time.monotonic()))
        if frame is None:
            break
        # No MAC-based self-loopback check here on purpose — see dcp-sim.py's own comment on
        # the same line it used to have. This process and the sim share en0's one hardware MAC
        # as their source address when both run on this Mac, so a MAC comparison would silently
        # discard the sim's legitimate reply along with our own echoed request. Our own request
        # loops back with frame_id == FRAME_ID_DCP_IDENT_REQ, which the frame_id check below
        # already rejects (it only accepts FRAME_ID_DCP_IDENT_RES) — no MAC check needed.
        payload = frame[14:]
        if len(payload) < 12:
            continue
        frame_id, service_id, service_type, got_xid, _reserved, data_length = struct.unpack(">HBBIHH", payload[0:12])
        if frame_id != FRAME_ID_DCP_IDENT_RES or service_id != SERVICE_ID_IDENTIFY:
            continue
        if service_type != SERVICE_TYPE_RESPONSE_SUCCESS or got_xid != xid:
            print(f"  ignored a reply with the wrong xid/type from {frame[6:12].hex(':')}")
            continue
        name = parse_name_of_station(payload[12 : 12 + data_length])
        print(f"  ANSWERED by {frame[6:12].hex(':')}  name_of_station={name!r}")
        found += 1

    if found == 0:
        print("no reply — nothing answered the Identify multicast on this interface", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
