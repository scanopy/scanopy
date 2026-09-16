# PROFINET DCP Test Environment

A real fixture and a real daemon scan against it — not an in-process mock standing in for both
sides. §12 of the OT-discovery scoping report says plainly that no such environment exists yet;
this is it.

**Runs locally, on this Mac** — the same machine the installed daemon-under-test runs on, on the
same interface it actually scans. DCP is raw Ethernet and does not route, so that's what makes a
real scan able to reach the sim at all.

## History: why this isn't on the SNMP lab VM

The first version of this shared the SNMP lab's remote Proxmox VM (`tools/snmp/`'s host), the
same way the SNMP fixtures do — two `macvlan` children of the VM's `eth0`, one for the sim, one
for a verification client, confirmed answering real DCP Identify traffic between them
(2026-09-08). That precedent doesn't carry over to DCP the way it does to SNMP: SNMP is routable
UDP, so a lab host anywhere with IP connectivity works. **DCP is raw Ethernet, addressed to a
multicast MAC, and does not cross an L3 boundary.** A daemon running on this Mac, on its own LAN,
can never see a sim on a remote VM's segment no matter how the two are IP-numbered — confirmed
the hard way: a real scan run here found nothing, because the daemon's own subnet (`en0`,
`192.168.4.0/22`) and the VM's (`192.168.7.0/24` on a physically separate network reached over a
routed link) are not the same L2 segment, whatever the address ranges suggest on paper. Moved
local rather than adding a second host on that segment, since the daemon-under-test already runs
here.

## What this is

`dcp-sim.py` is a standalone PROFINET DCP Identify responder — a fake device. It does **not**
import or share code with `backend/src/daemon/discovery/service/network/dcp/`, on purpose: a sim
built from the client's own encoder/decoder can only confirm the client agrees with itself.
Written independently from the same wire-format reference (Wireshark's `packet-pn-dcp.c`
dissector — see `dcp/packet.rs`'s own module doc), so a daemon scan against it is a real test of
whether the two independent implementations agree on the wire format, the way a real `snmpd`
agent is for SNMP.

`dcp-verify.py` sends one Identify request and prints whatever answers — the same role
`snmpget`/`snmpwalk` play for the SNMP lab: a protocol-level check independent of both the daemon
and the sim's own responder logic, to confirm the fixture actually answers before trusting a full
daemon scan against it.

Both talk raw Ethernet through `bpf_raw.py` — macOS's `/dev/bpf*` character device, driven
directly via `ioctl`/`read`/`write` (stdlib + `fcntl` only, no scapy). macOS has no Linux
`AF_PACKET`, so this is the real equivalent: the same mechanism
`backend/vendor/pnet_datalink/src/bpf.rs` uses for the daemon itself. `bpf_raw.py`'s ioctl
request codes and the `bpf_hdr`/`ifreq` struct layouts it hand-packs are cross-checked against a
small compiled C program against this machine's actual SDK headers
(`net/bpf.h`), not just against the vendored Rust source — both agree exactly.

## Running it

```
tools/dcp/dcp-test-env.sh start    # launches dcp-sim.py in the background on $DCP_IFACE (default en0)
tools/dcp/dcp-test-env.sh verify   # sends one real Identify request, prints whatever answers
tools/dcp/dcp-test-env.sh status
tools/dcp/dcp-test-env.sh stop
```

Set `DCP_IFACE` if the installed daemon scans on something other than `en0`. Both `start` and
`verify` need `sudo` — `/dev/bpf*` is root-owned (`crw-------`). `start` runs `dcp-sim.py` under
`sudo` in the background, has it write its own pid to `/tmp/dcp-sim.pid` once it's actually bound
and listening (not derived from shell-level `$!` after `sudo cmd &`, which isn't reliable across
sudo configurations), and logs to `/tmp/dcp-sim.log`.

## What's confirmed

**BPF delivers cross-process on one shared interface.** This was the open question when the
local port first landed: does macOS deliver a frame written by one process's `/dev/bpf*` fd to a
*different* process's `/dev/bpf*` fd bound to the same interface, which `dcp-sim.py` (answering)
and a real daemon scan (asking) both depend on when sharing `en0`. Confirmed live with
`sudo tcpdump -i en0 -e 'ether proto 0x8892'` running as a third, independent process while
`dcp-verify.py` sent a request: `tcpdump` captured both the outgoing Request and the (at the
time, remote-VM) reply, so cross-process delivery on one interface is real, not just documented
BSD architecture taken on faith.

**Two bugs that made it look broken anyway, both fixed:**
- `dcp-sim.py`/`dcp-verify.py` filtered "our own frame looped back" by comparing the frame's
  source MAC to their own — correct on the old macvlan setup, where the sim and the verify
  client each had a distinct MAC, but wrong once both share `en0`'s one hardware MAC: it silently
  discarded a legitimate peer's frame too. Fixed by dropping the MAC check entirely — the
  existing frame-type check (Request vs. Response) already excludes a self-echo without it,
  matching how the real daemon's own `dcp/identify.rs::collect()` does this (content/xid-based,
  no MAC comparison anywhere).
- `bpf_raw.py` didn't set `BIOCPROMISC`, reasoning from the real daemon's bpf.rs having no
  promiscuous ioctl at all — the wrong comparison. The daemon only *sends* DCP multicast
  (promiscuous doesn't affect transmit) and only *receives* the sim's *unicast* reply (always
  delivered regardless of promiscuous). The sim is different: it has to *receive* an arbitrary
  multicast Request nobody addressed to it, which a non-promiscuous BPF listener never sees —
  confirmed by the same `tcpdump` capture above showing the Request that a non-promiscuous sim
  didn't. The identical bug class the original Linux/macvlan version hit, for the same reason
  (see the history section above) — just missed again on the port to a different OS.

## Running a real daemon scan against it

1. `tools/dcp/dcp-test-env.sh start` (confirm with `verify` first).
2. Run discovery from the installed daemon on this Mac, same as any other scan.
3. Confirm in the DB: a host with no IP addresses, one interface carrying the sim's reported MAC
   (`00:1a:2b:dc:90:01` by default — see `dcp-sim.py --device-mac`, not `en0`'s own MAC), sourced
   `AttributeSource::ProfinetDcp`, named `scanopy-dcp-sim` (from the Identify Response's Name of
   Station block).

The sim's reported MAC is deliberately not `en0`'s own hardware MAC: `en0`'s real address has the
locally-administered bit set (so did the original macvlan sim's auto-generated one), and the
backend refuses to mint a host from a MAC that isn't a real vendor-assigned unicast address —
confirmed live, the first local run answered correctly but the daemon logged exactly that
rejection for `en0`'s own MAC. `dcp-sim.py` fabricates a vendor-style address instead (the same
Cisco OUI `tools/snmp/`'s own fixtures use), so the full mint path can actually be exercised.

This is the last piece of end-to-end proof the unit tests (`dcp/packet.rs`, `dcp/identify.rs`)
can't provide on their own — they're honest about testing this daemon's understanding of the wire
format against itself; `dcp-verify.py` proves it against an independent implementation; only a
real daemon run proves the *whole* path (submission, minting, provenance, L2 visibility) end to
end.

## What this still doesn't prove

`dcp-sim.py` is a reference implementation written for this purpose, not a real PROFINET device —
it settles whether the daemon's parser/builder agree with an independent reading of the same spec
material, not whether a real Siemens/Rockwell/Beckhoff device behaves identically. In particular
it does not resolve the unicast-vs-multicast question `dcp/packet.rs`'s module doc flags as
unverified — `dcp-sim.py` replies unicast because that's what the daemon's current receive filter
assumes, not because it's confirmed as the real-world answer. For that, the options are the same
as before: IEC 61158-6-10 itself, or a genuine third-party stack. RT-Labs'
[p-net](https://github.com/rtlabs-com/p-net) (BSD-3, used for real vendor pre-certification)
remains the candidate if that level of confidence is ever needed — its `pn_dev` sample app
already answers DCP Identify — but building and deploying it is a larger lift than this simple
responder and hasn't been done here.
