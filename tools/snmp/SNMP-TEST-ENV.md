# SNMP Test Environment

> **Where the devices live.** Every device is a typed Rust value in
> `backend/src/daemon/discovery/integration/snmp/sim/devices/`, and `lxc/setup.sh` is now only the
> host harness — interfaces, the three `pass` handlers, systemd units. `make snmp-deploy` generates
> the data files and agent configs from those definitions and ships what it generated, so there is
> no committed artifact that can drift. To add or change a device, see **Adding a device** below.

22 simulated network devices running on a Proxmox VM, each on port 161. Most speak SNMPv2c; `.236`/`.237` are version-locked to exercise the SNMPv1 and SNMPv3 paths (#557); `.238`/`.239` are Extreme switches that exercise the LLDP local-port remap (Issue 2, July 2026); `.240`–`.242` reproduce the L2-topology failures from #664, #649 and #614; `.243` serves a deliberately malformed neighbour record; `.244` serves the port-id shapes from #668 and repeats one MAC across every port; `.245` serves that report's last device, whose neighbour table is indexed one sub-id short; `.246`/`.247` cover #674 and the Westermo local-port report; `.248`/`.249` are the two failure shapes the partial-failure reporting exists for; `.250` is the Dell OS10 switch from #685, whose breakout-port names and 568+ local-port namespace decide which interface a neighbour lands on; `.251` is the Cisco from #686 and the only device here that serves different data per SNMPv3 context.

| IP | Host | Version | Credential | Device |
|---|---|---|---|---|
| 192.168.7.230 | switch-core-01 | v2c | community `netdefault` | Cisco C2960 |
| 192.168.7.231 | switch-access-01 | v2c | community `netdefault` | Cisco C3750 |
| 192.168.7.232 | router-gw-01 | v2c | community `secret42` | Juniper MX204 |
| 192.168.7.233 | firewall-01 | v2c | community `secret42` | FortiGate 60F |
| 192.168.7.234 | printer-lobby | v2c | community `public` | HP LaserJet M428 |
| 192.168.7.235 | ap-wireless-01 | v2c | community `netdefault` | Ubiquiti UniFi AP |
| 192.168.7.236 | legacy-switch-01 | **v1 only** | community `legacyv1` | Cisco C2950 |
| 192.168.7.237 | secure-switch-01 | **v3 only** | user `scanopyv3` (see below) | Huawei S5000 |
| 192.168.7.238 | switch-exos-01 | v2c | community `netdefault` | Extreme X435 (EXOS) |
| 192.168.7.239 | switch-voss-01 | v2c | community `netdefault` | Extreme VSP-7400 (VOSS) |
| 192.168.7.240 | switch-netgear-01 | v2c | community `netdefault` | Netgear GS724Tv3 |
| 192.168.7.241 | switch-aruba-01 | v2c | community `netdefault` | HP/Aruba ProCurve 2910al |
| 192.168.7.242 | switch (Omada) | v2c | community `public` | TP-Link Omada TL-SG3216 |
| 192.168.7.243 | switch-flaky-01 | v2c | community `netdefault` | Malformed-LLDP profile (see below) |
| 192.168.7.244 | switch-dlink-01 | v2c | community `netdefault` | D-Link DGS-1210-48 (see below) |
| 192.168.7.245 | switch-tplink-01 | v2c | community `netdefault` | TP-Link TL-SX3016F (see below) |
| 192.168.7.246 | switch-unsorted-01 | v2c | community `netdefault` | Out-of-order ARP table (see below) |
| 192.168.7.247 | switch-macport-01 | v2c | community `netdefault` | Westermo WeOS, from the customer's walk (see below) |
| 192.168.7.248 | switch-mute-01 | v2c | community `netdefault` | Answers the credential, serves nothing (see below) |
| 192.168.7.249 | switch-stuck-01 | v2c | community `netdefault` | ARP table never advances (see below) |
| 192.168.7.250 | switch-dell-01 | v2c | community `netdefault` | Dell PowerSwitch S4112T-ON, OS10 breakout ports (see below) |
| 192.168.7.251 | switch-cisco-01 | **v3 only** | user `scanopyctx`, context `vlan-20` | Cisco Catalyst 3850, per-VLAN bridge context (see below) |
| 192.168.7.252 | switch-ocnos-01 | v2c | community `netdefault` | UfiSpace S9600-32X, IP Infusion OcNOS, LLDP-V2-MIB only (see below) |

**LLDP local-port remap (`.238`/`.239`).** ExtremeXOS reports its `lldpRemTable` local-port index as an `lldpLocPortNum` (1..N) that is a **separate namespace from `ifIndex`** (switch-exos-01 uses ifIndex 1001+, ifName `1:N`), so neighbours only resolve if the daemon walks `lldpLocPortTable` (`1.0.8802.1.1.2.1.3.7`) and suffix-matches `lldpLocPortId` against `ifName`. Before the Issue 2 fix, switch-exos-01 yields **zero** LLDP neighbours. Extreme VOSS (switch-voss-01) reports local-port == ifIndex with `lldpLocPortId` matching `ifName` exactly, so it stays correct on both old and new code — the regression guard for the fix.

**L2 neighbour resolution (`.240`/`.241`).** These two are cabled to each other in the fixture data — `switch-netgear-01 g1 ↔ switch-aruba-01 port 41` and `g2 ↔ A5` — and between them cover both halves of a physical link:

- **Chassis MAC that is on no port (#664).** switch-netgear-01's LLDP chassis id is `00:1a:2b:3c:4d:63`, while its ports report `…:65/:66/:67` and it bears no IP with that MAC. switch-aruba-01's neighbour entries advertise that chassis MAC, so the remote host is identifiable **only** through the `chassis_id` recorded from switch-netgear-01's own LLDP local identity. Matching MACs against interfaces and IPs alone yields `hosts_resolved=0` and an empty L2 Physical view.
- **Locally-assigned port ids (#649).** switch-netgear-01's neighbour entries use port-ID subtype 7 with values `41` (which is switch-aruba-01's `ifDescr`) and `197` (which matches only its `ifIndex` — that port is labelled `A5`). Both shapes occur on real Aruba/HP gear. Treating subtype 7 as unresolvable stops resolution at the host, and a host-only neighbour draws **no edge at all**, so the switch is missing from L2 Physical entirely.

Both links should render in L2 Physical, and the server's `LLDP/CDP link resolution complete` line should report `ports_resolved` covering all four neighbour records (two per device).

**High-ifIndex interface persistence (`.242`).** The Omada TL-SG3216 puts its 16 physical ports at ifIndex 49153–49168, reports **no** ifXTable `ifName` for any of them, and returns the same chassis `ifPhysAddress` on every port; only ifIndex 1 (`Vlan-interface1`) carries a name and an IP. All 17 must persist as distinct interfaces. Note its `sysName` is the literal `switch`, matching the reporter's device.

It advertises exactly **one** LLDP neighbour, switch-dlink-01 on local port 5, and that is the whole of its LLDP data — deliberately, so the interface-persistence path it exists for stays readable (17 interfaces, regardless of what LLDP does). That one neighbour is the reciprocal half of the `.244` pair described below: two devices that each repeat one address across every port, each naming the other exactly once. Its `lldpLocPortNum` runs 1–16 against ifIndex 49153–49168, so the local-port remap is load bearing here — `lldpLocPortId` is subtype 5 carrying the `ifDescr`, the only identifier these nameless ports have, and without the remap the neighbour lands on an index no interface holds and is discarded whole.

> **MAC octet padding.** Every fixture wrote MACs abbreviated (`0:1a:2b:0:10:0`) until 2026-07-27, and the daemon's string-parsing fallback rejected that form outright — so no LLDP data persisted for *any* sim device and no host ever got a `chassis_id`. Silently: an unparseable chassis id discards the whole neighbour record, which is indistinguishable from a switch that advertises none. The daemon now accepts both forms, and the fixtures are padded **except switch-exos-01's own chassis id**, deliberately left abbreviated as the standing guard for that tolerance (ExtremeXOS is one of the two vendors known to send this identifier as a string rather than octets).

**Malformed neighbour records (`.243`).** A truncated `lldpRemChassisId` column and a device that simply serves no chassis ID are indistinguishable to the daemon — both yield a neighbour carrying a port ID and a system name but no chassis ID. The first is a transient nobody can schedule; the second is static and reproduces on every scan, which is what makes this path testable at all.

Taken at face value that record is destructive: the chassis ID is a mandatory TLV (IEEE 802.1AB), so it is malformed, but writing it through overwrites a good chassis ID with NULL — and a row without one is excluded from L2 resolution entirely, freezing the link at whatever it last resolved to with no way back. That is what stranded `router-gw-01` in July 2026.

`switch-flaky-01` links to `switch-core-01`'s `Gi0/3` (the one port on that switch with no other neighbour) and ships five LLDP variants. The agent serves whichever is copied over `-lldp-active.txt`, and the `pass` handler re-reads its file per request, so swapping takes effect immediately with **no snmpd restart**:

```bash
# on the VM — serve the chassis-less record
cp /etc/snmp-test/data/switch-flaky-01-lldp-nochassis.txt \
   /etc/snmp-test/data/switch-flaky-01-lldp-active.txt

# ...and restore the well-formed one
cp /etc/snmp-test/data/switch-flaky-01-lldp-complete.txt \
   /etc/snmp-test/data/switch-flaky-01-lldp-active.txt
```

The other variants separate the causes that used to arrive as one number (#668). All of them discard the record; the daemon's warning now says which happened — and, decisively, whether a rescan is worth the operator's time — and the counters are what these files exercise:

| Variant file | Serves | Counter it drives | Reported cause |
|---|---|---|---|
| `-lldp-nochassis.txt` | neither `.4` nor `.5`, for the only neighbour | `ghost_rows`, `kept=0` | `GhostRows` — device contributes nothing |
| `-lldp-ghost.txt` | local port 2 in `.6`–`.10` only, port 1 complete | `ghost_rows`, `kept=1` | `GhostRows` — device is present with holes |
| `-lldp-nosubtype.txt` | `.5` only | `missing_subtype` | `IncompleteRecords` |
| `-lldp-badsubtype.txt` | `.4` as a string, `.5` fine | `unexpected_subtype_type="OctetString"` | `UnexpectedType` |

Note what the first two have in common: a chassis column that lists **none** of a row's positions is indistinguishable from one that never had them, so both read as ghost rows. They differ in `kept`, which is what decides whether the warning says the device contributes no physical links at all or only that some are missing — a different place on the map, so they must not read the same.

`-lldp-badsubtype.txt` matters most: it reads as a *complete* walk — no truncation signal anywhere — so before the per-cause counters the only evidence was the record silently going missing.

`-lldp-ghost.txt` is the sparse-chassis-column shape. Until August 2026 it had no fixture at all and was reachable only in unit tests, so the classification that separates it from a cut-short read had never been checked against a real agent.

**No variant serves `WalkCutShort`**, and that is not an omission: it needs a column to stop *mid-walk*, which a static data file cannot stage. It is the one cause where a rescan is the remedy rather than a waste, so it is covered by a unit test that fails the transport partway (`a_cut_short_chassis_column_is_not_reported_as_a_firmware_defect`). The sim env produces it by accident anyway — `pass` forks per request and the agents fall behind under load, which is the noise the header of `lxc/setup.sh` warns about.

Re-running `lxc/setup.sh` also resets it, which is the simplest way to undo a test that left the device broken.

**Port ids that name-only matching cannot resolve (`.244`).** Modelled on the D-Link DGS-1210-48 from #668. Its own ifTable uses the D-Link shape — `ifDescr` = `D-Link DGS-1210-48 Rev.GX/7.20.003 Port N`, `ifName` = `Slot0/N`, `ifIndex` = N — and its four neighbour records each need a different route to their far end:

- **Subtype 5 carrying a bare port number.** `lldpRemPortId` = `2` with `lldpRemPortIdSubtype` = 5 (`interfaceName`), while switch-core-01's ifIndex 2 is named `GigabitEthernet0/2`/`Gi0/2`. Subtype 5 used to get a name lookup and nothing else, so this resolved to the host and stopped — and a host-only neighbour draws no edge. It now falls through to `ifIndex`, the same ladder subtypes 2/6/7 already had.
- **A port id that matches nothing, and a port description that does.** `lldpRemPortId` = `ethernet1/0/44` is neither a name on that device nor a number, so the id is a dead end; `lldpRemPortDesc` = `GigabitEthernet0/1` is byte-identical to switch-core-01's `ifDescr` for ifIndex 1. That field was stored and never matched on.
- **A MAC port id that identifies exactly one port** — local port 3, pointing at switch-macport-01. `lldpRemPortIdSubtype` = 3 with `00:07:7c:20:01:e3`, that device's `ifPhysAddress` for `eth3` and for nothing else, so it must resolve to that port. This is the positive half of the pair whose negative half is switch-tplink-01 local port 4: same subtype, same tier, opposite verdict, because the far end there repeats one MAC across every port. Both matter — a guard that rejected this one too would look correct while quietly costing every vendor that addresses its ports individually. Its chassis id is `00:07:7c:20:01:e0`, which is on none of that device's *physical* ports — only on its six VLAN interfaces, which the uniqueness test excludes — so the port lookup cannot borrow the answer; its `lldpRemPortDesc` is `Ring port to peer`, matching nothing over there, so a broken MAC tier fails loudly instead of being rescued by the description tier. Both identifiers are sent as `octet` — six raw bytes, the only end-to-end coverage of `parse_mac_id`'s raw-octet branch, since every other LLDP fixture here uses the ASCII form.
- **A far end no identifier can name a port on** — local port 4, pointing at switch-omada-01, and the reciprocal-tier case. Both devices repeat one chassis address across every interface and each advertises that address as its port id, so on both sides the MAC names the device and nothing narrower. What resolves the link is that **each names the other on exactly one port**: those two local ports are attached by `ifIndex` on their own devices, so the pair binds without either far-end port ever being identified, and L2 Physical draws a solid `PhysicalLink` instead of a dashed device-level one. Its negative counterpart is switch-tplink-01 local ports 3 and 4, which name switch-dlink-01 *twice* — a LAG, genuinely ambiguous, which must stay device-level. Keep both at their current counts: a second link between `.244` and `.242`, or the removal of one of tplink's two, silently deletes one half of the pair.

Both should resolve to `Neighbor::Interface` and draw edges in L2 Physical. The records deliberately share `Gi0/1`/`Gi0/2` with links other sim devices also claim — the profile exercises port-id resolution, which runs per interface row, not a physically consistent lab.

It also carries the third report from the same issue: **every one of its ports reports the same `ifPhysAddress`**, the chassis base MAC `00:ad:24:af:4e:00`, which is also its `lldpLocChassisId`. The real DGS-1210-48 does this, RFC 2863 does not require per-port addresses, and the reporter — seeing one MAC repeated down the whole interface list — read it as Scanopy mis-attributing them. It is not: the ifTable walk keys each `ifPhysAddress` off its own row's OID sub-id and cannot copy one row's value onto another. What it did break is identity: a MAC that names three ports names none of them, and the lookups that treated one as a port identifier picked whichever row the database returned first.

> **Send a MAC with `octet`, never `string`.** `string` transmits the value as text, so
> `00:ad:24:af:4e:00` arrives as 17 ASCII bytes where a `PhysAddress` is six raw octets. The daemon
> correctly refuses to read that as an address, the interface stores no MAC, and the fixture quietly
> tests nothing while the L2 view still looks healthy. `octet` takes space-separated hex
> (`octet 00 ad 24 af 4e 00`) and emits what a real agent sends. This has caught three fixtures so
> far. Check any MAC-valued column you add:
>
> ```bash
> snmpwalk -v2c -c netdefault -Ox 192.168.7.244 1.3.6.1.2.1.2.2.1.6   # six octets, not seventeen
> ```

An earlier revision of this profile gave each port its own address (`…:4e:01`–`03`) — the case that never needed guarding — and nothing else here depended on them being distinct.

**Two devices that fail on purpose (`.248`/`.249`).** These exist because the partial-failure
reporting had nothing to report against: before they were added, no scan of this environment had
ever produced an incomplete-walk warning for any group, so that entire path went unexercised while
looking healthy.

- **`.248 switch-mute-01`** answers the credential and then serves nothing — no interfaces, no
  neighbours, no addresses, no forwarding data. That is the shape a host takes when SNMP
  "succeeds" and yields nothing, which used to read to an operator as a clean scan. It must
  produce the warning saying the device answered SNMP and returned nothing at all. Note the seven
  `pass -p 1` lines in its config: `ifTable`/`ifXTable` are suppressed by the `-I` flag every unit
  carries, but `ipAddrTable` and `ipNetToMediaTable` cannot be, so without those overrides it
  would report the VM's own addresses and ARP cache and would not be mute.
- **`.249 switch-stuck-01`** answers every request for its ARP table with the same row, whatever
  was asked. This is the non-advancing agent the walk's retry-then-stop guard was written for
  (originally a Ubiquiti bridge FDB): left unguarded it would have the daemon re-request the same
  page until the entry cap or the integration timeout. It has an ordinary `ifTable` so that it is
  a *shortfall* case rather than a mute one, and it must produce a warning naming the ARP table
  with a desynchronised reason — not a device that quietly reports no ARP entries.

**A table served out of ascending OID order (`.246`).** Modelled on the Hikvision DS-3T1512HP from #674. The switch stores its ARP table unsorted and iterates it positionally, so GETNEXT hands back whatever row physically follows the one asked for — answering `…10.20.30.44` with `…10.20.30.1`. `snmpwalk` stops at `OID not increasing`; `snmpbulkwalk -Cc` reads all 45 rows. The data is retrievable, and only a client insisting every step ascend refuses it.

This is the one device served by `snmp-pass-handler-unsorted.sh` rather than the usual handler: the normal one answers GETNEXT with the first line *numerically* greater than the request, so a shuffled file would just end the walk early and could never reproduce the defect.

Two properties of the fixture matter and should survive any edit. There are 45 rows per column, so the walk needs more than one GETBULK page (the daemon asks 20 at a time); and the rows are ordered evens-then-odds so that the **second page ends lower than the first** — which is the exact moment a strictly-ascending walk gives up. Its `ifTable` is deliberately ordinary, so an empty ARP table here is visibly a property of that table rather than of the whole host.

```bash
snmpwalk    -v2c -c netdefault 192.168.7.246 1.3.6.1.2.1.4.22.1.1   # stops: OID not increasing
snmpbulkwalk -Cc -v2c -c netdefault 192.168.7.246 1.3.6.1.2.1.4.22.1.1   # all 45 rows
```

A scan of this host must produce 45 ARP entries. Before the #674 fix it collects 40 and reports the walk as desynchronised; the ARP entry is a join across four columns, so a column that comes up short discards every row the others read — which is how the reporter's switch logged `count=0` while answering hundreds of rows.

**A Westermo WeOS switch, from the customer's own walk (`.247`).** Reconciled against the real device (`172.17.6.193`) in August 2026. It previously modelled a switch whose `lldpLocPortNum` was a separate namespace from `ifIndex`; the customer's LLDP walk disproved that premise, and a fixture that cannot fail the way the reported device does is not a guard.

What the real device actually reports, and why each column is here:

- **`ifIndex` 10–19 for `eth10` down to `eth1`**, plus `lo` (ifType 24) and six VLAN interfaces (ifType 53). The port number is neither the index nor an offset from it: index 11 is `eth9`, 16 is `eth4`, 19 is `eth1`.
- **`ifDescr` carries the media type in front of the name** — `100-T eth9`, `1000-LX eth1` — so a neighbour advertising the bare port name matches `ifDescr` nowhere on this family. `ifName` *and* `ifAlias` both hold the bare name; this is the only fixture here that serves `ifAlias`, and it is what makes `eth9` resolvable at all.
- **`ifPhysAddress` is unique per physical port** (`…e1`–`…ea`) while all six VLAN interfaces repeat the chassis address `…e0`, which is on no physical port. A MAC lookup that counts virtual rows finds six matches and declines — costing a port that no physical interface ever contested.
- **`lldpLocPortTable` is keyed 10–19, which are this device's `ifIndex` values**: the local-port table is the identity mapping, and each port advertises subtype 3 with its own unique `ifPhysAddress`, so the unique-MAC tier confirms it. Nothing here needs the remap, which is exactly why v0.17.10's remap fix changed nothing for this customer. The reverse-numbering case the remap exists for is covered by unit test rather than by pretending this device is it.

Its three neighbours are the real ones, and each reaches its far end by a different route:

| Local port | Chassis | Port id | Notes |
|---|---|---|---|
| 11 (`eth9`) | subtype **7** (local) `C230408` | subtype 3 MAC `e8:80:88:be:30:e7` | no `portDesc`, **no `sysName`** |
| 19 (`eth1`) | subtype 4 MAC `f0:64:26:b3:84:00` | subtype 5 `1/19` | Extreme 5520 FabricEngine, sysName `VSAFC11` |
| 16 (`eth4`) | subtype 4 MAC `78:8c:77:e5:92:7d` | subtype 3, same MAC | Lexmark printer |

The port-11 row is the one nothing else covers. Its chassis id names no MAC, no address and no interface, so the far end can only be found through `hosts.chassis_id` — recorded from that device's own `lldpLocChassisId`. If those two paths ever canonicalise differently the neighbour is unfindable, and no counter distinguishes that from a device nobody scanned.

Its chassis id is served as six raw octets while switch-dlink-01 names this same device with the ASCII text form, so one scan proves both encodings reach one identity.

Note the contrast with `.244`/`.245`, which report **one shared MAC across every interface**: MAC-based matching must decline there rather than collapsing every neighbour onto one port. Those two also advertise each other, which is what exercises the reciprocal-LLDP tier — neither can name the other's port, so the link is only port-precise because each names the other exactly once.

**A neighbour table indexed without `lldpRemTimeMark` (`.245`).** Modelled on the TP-Link TL-SX3016F from #668, from the reporter's own `snmpwalk`. The MIB indexes `lldpRemEntry` as `lldpRemTimeMark.lldpRemLocalPortNum.lldpRemIndex`; this firmware omits the time mark and indexes on the remaining two, so every neighbour row arrives one sub-id shorter than on every other device here:

```
.1.0.8802.1.1.2.1.4.1.1.4.1.1 = INTEGER: 4                  # local port 1, remIndex 1
.1.0.8802.1.1.2.1.4.1.1.5.1.1 = STRING: "00:1A:2B:00:10:00"
```

This is the shape that made the device vanish without evidence. A parser requiring three sub-ids built no record, so nothing reached the discard counters, the walk still reported itself complete, and an empty result from a sixteen-port switch was then treated as the device authoritatively reporting no neighbours — clearing the links the server already held. It was the only failure in this query that raised **no warning of any kind**, which is why the reporter's completed scan named every other problem device and not this one. Verify with:

```bash
snmpwalk -v2c -c netdefault 192.168.7.245 1.0.8802.1.1.2.1.4.1.1.4   # two-element index
```

Two further quirks from the same device are kept deliberately, because they decide whether a row that now survives can actually resolve: chassis ids are subtype 4 carrying an **uppercase ASCII MAC** rather than six raw octets, and ports are `ifDescr` `ten-gigabitEthernet 1/0/N` with **no `ifName`** (there is no ifXTable `pass` in its config), alongside a `Vlan-interface1`. Its `lldpLocPortNum` equals `ifIndex`, so the local-port remap is the identity mapping and cannot mask the index parse under test.

Each of its five neighbours resolves through exactly one intended path, matched on a value the far end actually reports:

| Local port | Far end | Host matched by | Port matched by |
|---|---|---|---|
| `1/0/1` | switch-core-01 | its own `lldpLocChassisId` `00:1a:2b:00:10:00` | `ifName` `Gi0/3` |
| `1/0/2` | *nothing* | — | — |
| `1/0/3` | switch-dlink-01 | chassis `00:ad:24:af:4e:00`, now on that switch's ports as well as its `hosts.chassis_id` | `ifName` `Slot0/3` |
| `1/0/4` | switch-dlink-01 | same chassis MAC | **nothing, deliberately** — see below |
| `1/0/5` | switch-netgear-01 | `hosts.chassis_id` only — `00:1a:2b:3c:4d:63` is on no port and no IP (the #664 shape) | `ifIndex` 3 (`g3`), since `3` matches no name and the port desc deliberately matches nothing |

So a clean scan gives three edges in L2 Physical — two port-to-port, one device-level from `1/0/4`, and, from `1/0/2`, none.

**`1/0/4` is the far side of the shared-MAC case**, and the only subtype-3 port id in the lab. It advertises `lldpRemPortIdSubtype` = 3 (`macAddress`) with `00:AD:24:AF:4E:00` — the address switch-dlink-01 reports on all three of its ports. The chassis id resolves the host; the port id must then resolve *nothing*, because a MAC belonging to three ports identifies none of them. Expect `port_ambiguous=1` on the `LLDP/CDP link resolution complete` line, a named entry on the companion warning, and one amber `NeighborLink` — **not** a teal `PhysicalLink` to whichever of `Slot0/1`–`Slot0/3` came back first, which is what it drew before #668. Its `lldpRemPortDesc` is `Uplink to core`, matching no `ifName` or `ifDescr` on that switch: the port-description tier still runs after an ambiguous port id (a description that *does* match should win), so anything matchable there would resolve the port and hide the case.

Check `port_ambiguous`, not the edge colour. If switch-dlink-01's physAddress is ever sent as `string` again it stores no MACs, the lookup returns `port_not_found` instead, and the identical amber edge appears for an entirely different reason.

Note that `1/0/3` and `1/0/4` name the same far-end device on purpose. The pair is the A/B: identical host resolution, one port id that identifies a port and one that cannot.

**`1/0/2` is unresolvable on purpose.** It advertises a desk phone whose MAC and sysName belong to no device in this lab, so every host tier fails and it is the environment's only source of a non-zero `host_not_found`. That counter is otherwise permanently 0 here, which left the server-side summary that names unmatched far ends with no way to fire. Endpoints exactly like this are what `host_not_found` legitimately consists of on a real network (#668).

> Every far-end value above is checked against what the lab actually reports. An earlier revision used a made-up chassis MAC for switch-netgear-01 and a port (`Gi0/4`) that switch-core-01 does not have; both still appeared to work — one fell through to the sysName tier, the other stopped at a device-level edge — so the profile passed without exercising what it documents. When adding a neighbour here, confirm the far end's `hosts.chassis_id`, `if_name` and `if_index` in the scanned data first.

**A device that serves only the LLDP-V2-MIB (`.252`).** Modelled on a UfiSpace S9600-32X running IP Infusion OcNOS 7.0.1, from an `snmpwalk` of the real switch (#688), identifiers rewritten for the lab. Its LLDP lives under the 802.1AB-2009 root `1.3.111.2.802.1.1.13` and nowhere else: a walk of the classic `1.0.8802.1.1.2.1.4.1` finds nothing, so before the fallback the device contributed no L2 edges at all. Three things differ from every other device here, and each is what the regression test checks:

```
.1.3.111.2.802.1.1.13.1.4.1.1.5.0.10009.1.6 = INTEGER: 4      # timeMark.ifIndex.destMacIndex.remIndex
.1.3.111.2.802.1.1.13.1.4.1.1.6.0.10009.1.6 = Hex-STRING: 00 1A 2B 40 E9 CA
```

- The remote columns sit **one above** their classic numbers (`lldpV2RemLocalIfIndex` is inserted as column 2), so chassis subtype is `.5`, chassis id `.6`, and so on.
- The row index has **four** sub-ids, the third a row pointer into `lldpV2DestAddressTable` (always 1 here). The classic end-relative parse reads `(1, remIndex)` off that and collapses every neighbour onto port 1.
- The local identifier is a **real ifIndex** — 3, 10009, 10073 — not an `lldpLocPortNum`, and there is no classic `lldpLocPortTable` to remap through. The daemon must place the neighbours on those interfaces directly.

The interface table is the device's own shape: `eth0` at ifIndex 3 and thirty-two 100G ports `ce0`–`ce31` at 10001, 10005, … 10125, with nothing in between, so the neighbour indices are only meaningful against a table with the same gaps. Verify with:

```bash
snmpwalk -v2c -c netdefault 192.168.7.252 1.0.8802.1.1.2.1.4.1.1.4    # nothing under the classic root
snmpwalk -v2c -c netdefault 192.168.7.252 1.3.111.2.802.1.1.13.1.4.1.1.6   # three neighbours, four-part index
```

> **The NUL half of #668 is not reproducible here.** The same D-Links NUL-terminate their port ids (`lldpRemPortId` arrives as `31 00`, i.e. `"1\0"`), which used to fail the write of the entire host. net-snmp's `pass` protocol is line-based — the handler prints OID, type and value as three lines — so an embedded `0x00` cannot survive the transport and no data file can express it. That half is covered by unit tests instead: `value_to_string`, `LldpPortId::from_snmp`, and `PgText`/`PgJson` in `server/shared/storage/pg_value.rs`.

**Dell OS10 breakout ports (`.250`).** A Dell PowerSwitch S4112T-ON running OS10 10.4.3.4, from #685, where a switch that discovers cleanly in every other respect showed **no physical connections at all**. Two properties of this device decide whether a neighbour reaches an interface, and nothing else in the lab has either.

**The interface names carry both anchor characters.** Port 14 is broken out, so OS10 names its lanes `ethernet1/1/14:1`, `:2` and `:3` — a `/` *and* a `:` in one name — alongside `ethernet1/1/1`…`1/1/13` and `mgmt1/1/1`. The local-port suffix tier matches an id that ends at a `:` or `/` boundary, and on this device the bare id `1` ends at one in three places at once (`mgmt1/1/1`, `ethernet1/1/1`, `ethernet1/1/14:1`). Taking the first match bound neighbours to a plausible-looking wrong port with no warning; the tier now requires the boundary to name one interface, and an exact name in `lldpLocPortDesc` outranks a matching id fragment.

**`lldpLocPortNum` is a separate namespace from `ifIndex`, and not a small one.** The management port is 4 and the front panel runs 555–570, against ifIndex values in the millions:

```
4   -> mgmt1/1/1
568 -> ethernet1/1/14:1 -> EVILCORP
569 -> ethernet1/1/14:2 -> VIRTUALPC
570 -> ethernet1/1/14:3 -> TAMMIERENEW
```

That is the mapping the reporter published, and it is what a scan of `.250` has to reproduce exactly — a neighbour on any other port is the bug, not a near miss.

The remote rows also carry **large, widely spaced `lldpRemTimeMark`s** (31577700, 93300700, 123380800, 127153800). Every other device here uses `0` or a small mark, so this is the only fixture that walks a first index sub-id of that size, and it is why the local ports arrive in an order unrelated to the ports themselves. Verify with:

```bash
snmpwalk -v2c -c netdefault 192.168.7.250 1.0.8802.1.1.2.1.4.1.1.5   # four neighbours, four time marks
snmpwalk -v2c -c netdefault 192.168.7.250 1.0.8802.1.1.2.1.3.7.1.3   # the 4/555-570 port namespace
```

Three of the four neighbours are end hosts advertising chassis subtype 7 (locally assigned) carrying a hostname rather than a MAC, which is what the reporter's walk shows; the fourth, on `mgmt1/1/1`, is subtype 4 and sends six raw octets with no sysName and no port description at all.

> **The walk falling short is not reproducible here**, and it is the other half of #685: the neighbour walk was being cut short by a timeout or an answer with no varbinds on it, which marks the whole neighbour set non-authoritative and discards it. Staging that needs an agent that fails to answer one request and then answers the next, and `pass` answers single-threaded — a handler that stalls past the daemon's 5s timeout blocks every later request behind it, so the late replies arrive one request out of step for the rest of the walk and the fixture would fail whether or not the fix is in. Covered by unit test instead, in `queries.rs::walk_tests`, the way `WalkCutShort` already is.

**Per-VLAN bridge context (`.251`).** A Catalyst 3850 running IOS-XE, from #686, where a switch with a full MAC-address table reported exactly one entry however it was queried. IOS-XE partitions its forwarding database per VLAN and keeps almost nothing in the default context, so a scan that cannot name a context reads the wrong table — and is told nothing is wrong, because a walk that ends cleanly on a one-row table is a complete walk.

This is the only device here whose answer depends on **which context you ask in**:

```bash
# default context — one entry, the reporter's symptom
snmpwalk -v3 -l authPriv -u scanopyctx -a SHA-256 -A ctxauthpass12345 -x AES -X ctxprivpass12345 \
  192.168.7.251 1.3.6.1.2.1.17.4.3.1.1
# vlan-20 context — nine
snmpwalk -v3 -l authPriv -u scanopyctx -a SHA-256 -A ctxauthpass12345 -x AES -X ctxprivpass12345 \
  -n vlan-20 192.168.7.251 1.3.6.1.2.1.17.4.3.1.1
# the same nine, through Cisco's v2c community indexing
snmpwalk -v2c -c 'netdefault@20' 192.168.7.251 1.3.6.1.2.1.17.4.3.1.1
```

`make snmp-verify` runs all three and compares them. The comparison is the check, not any one count: an agent that ignores `-n` answers both from the same table, which is the failure being guarded against, and asserting only "the context walk returns nine" would pass on one that ignores contexts entirely and happens to hold nine rows.

**It is two agents.** `pass` takes no context argument — it registers into the default context and nothing else — so a handler cannot be scoped to a context directly. `proxy -Cn vlan-20` in front of a second snmpd on `127.0.0.1:16151` is the only way stock net-snmp serves different data per context. That back end has no macvlan, no entry in `HOSTS` and no place in `SYSNAMES`; its unit is written by hand and ordered `Before=` the front agent, since a proxy to a dead port answers nothing while the front agent still looks healthy. A context walk returning **0** rather than 9 is that failure — check `systemctl status snmpd-switch-cisco-01-vlan20` first.

**It is on its own USM user (`scanopyctx`), and that is load-bearing.** Every seeded credential is Broadcast-scoped to every network, and only one SNMP credential per host ever executes — the last mapping that authenticates wins. If both the context-bearing credential and the plain `scanopyv3` one answered here, a scan would report nine FDB entries or one depending on mapping order. For the same reason it serves no seeded community: `netdefault@20` is reachable from the command line and is deliberately not a credential, so no v2c mapping can win against this device.

**ifTable, ifXTable and the system MIB stay in the default context**, as they do on the real switch. That is why the daemon scopes only its bridge and VLAN walks to the credential's context rather than the whole session — a context-wide session would find no interfaces at all here, which is the regression that shape would have introduced.

## Self-reported counts — what a device claims vs what it serves

The daemon compares a device's own published figures against what a collection actually read, so a
scan can say *"the device told us to expect 23 and we read 1"* rather than only *"we did not
finish"*. Three of those figures come from this lab:

| Figure | OID | Where it comes from |
|---|---|---|
| `ifNumber` | `.1.3.6.1.2.1.2.1.0` | derived from each fixture's own `ifIndex` rows |
| `dot1dBaseNumPorts` | `.1.3.6.1.2.1.17.1.2.0` | derived from each fixture's own `dot1dBasePortIfIndex` rows |
| `sysServices` datalink bit | `.1.3.6.1.2.1.1.7.0` | the `sysservices` line already in each config |

**The first two are derived from the device's own rows**, by `IfTable::declared_count` and
`BridgeTable::declared_port_count`. Editing an ifTable is enough; the scalars follow, and there is
no second figure to forget. `switch-dell-01` is the one device that overrides `ifNumber`, in one
place and with the reason on the field.

`dot1dBaseNumPorts` follows whatever bridge table it belongs to, so switch-cisco-01 gets one in
each of its contexts — a device whose two contexts disagreed about how many ports it has would be a
fixture bug nobody could see.

**None of those sets can be assembled any more.** A data file no config serves, a config naming a
file nobody wrote, a device with no config, or an ifTable served without its `ifNumber`
registration each used to provision quietly and then answer from net-snmp's built-in MIBs — a
device behaving oddly rather than a lab that was never assembled. Registrations are now derived
from the tables a device holds, so there is nothing left to disagree; the unit tests named in
*Adding a device* hold the line.

`ifNumber` is registered with `pass -p 1` because `mibII/interfaces` owns the scalar and would
otherwise answer it from the VM's own kernel state — the container's interface count, against a
fixture's 24 rows, on every device. The generator derives that priority from the subtree
(`needs_priority`), as it does for `ipAddrTable` and `ipNetToMediaTable`, which the IP module owns
for the same reason. Confirm what owns a subtree with `snmpd -Dregister_mib -C -c <conf>`.

**switch-dell-01 declares 52 interfaces and serves 23, deliberately.** Every other device here
agrees with itself, which demonstrates the check staying quiet but cannot demonstrate it firing —
and a guard nobody has watched fire is a guard nobody knows works. This is the GH #685 device, whose
report is a switch that discovers cleanly in every other respect, so the contradiction belongs on
it. A scan of `.250` must produce a warning naming **both** numbers and must still record all 23
interfaces: a device that misreports itself is still a device to scan.

> **switch-mute-01 (`.248`) also reports a bridge contradiction, and should.** It sets the datalink
> bit and answers nothing at all, so it has no bridge MIB to serve. That is what the device is for;
> the line is correct, and it is the only standing one in the lab.

## Bridge forwarding tables (`dot1dTpFdb` / `dot1qTpFdb`) — #686

Four devices serve a forwarding database. Before August 2026 none did, which is why **GH #686 — a
bridge-FDB defect — had no fixture that could reproduce it**.

| Device | Rows | What it covers |
|---|---:|---|
| switch-core-01 (`.230`) | 8 | mixed statuses, plus a `dot1qTpFdb` overlay on the same MACs |
| switch-dell-01 (`.250`) | 9 | a full table on a device whose ifIndexes are OS10 breakout lanes |
| legacy-switch-01 (`.236`) | 2 | the same join over **getnext**, which v1 forces |
| switch-cisco-01 (`.251`) | 1 / 9 | the reported device itself — see *Per-VLAN bridge context* above |

`.251` is the fixture #686 is actually about, and the section above is the one to read for it: its
count depends on which SNMP context you ask in, which is the half of the report the other three
cannot express. They cover the read — the three-column join, the status filter, the encoding, the
getnext path — on devices that answer the same way however you ask.

Three properties decide whether these are worth anything:

- **The MAC is the index** — six decimal sub-ids, one per octet — and the address column repeats it
  as six raw bytes via `octet`. That repetition is the only end-to-end coverage of a binary MAC on a
  table the daemon joins across three columns, and it is exactly what a `string` encoding here would
  silently stop testing.
- **Statuses are mixed.** The daemon keeps learned(3) and mgmt(5) and drops self(4), so
  switch-core-01's eight rows yield seven entries. A filter that stopped working shows up as a count
  that is too *high*, not as an empty table.
- **switch-core-01's `dot1qTpFdb` rows repeat MACs its `dot1dTpFdb` already lists**, in VLANs 10 and
  20. The daemon keys both on the MAC alone so one address learned in several VLANs collapses to one
  entry; if that collapse broke, the count would double rather than the table appearing empty.

> **What these three cover, and what `.251` covers.** A passing scan on switch-core-01,
> switch-dell-01 or legacy-switch-01 says the daemon reads a forwarding table correctly: the join
> holds across three columns, the status filter drops self(4), binary MACs survive, and the getnext
> path works. It says nothing about #686, whose switch returned a *complete, correct, one-row*
> table because the scan was asking in the wrong context. That half is `.251`'s, and the check
> there is the comparison between contexts rather than any single count.

## Adding a device

Every device is defined once, as a typed Rust value, in
`backend/src/daemon/discovery/integration/snmp/sim/devices/`. The deployment generates its data
files and its agent config from that definition — there is no second copy to keep in step, and
`lxc/setup.sh` no longer describes any device.

**The workflow for a new regression is: add a struct, write the test that fails, fix the code.**

1. **Ask the reporter for the subtrees below**, with `-On` so OIDs arrive numeric. On anything that
   partitions its bridge tables per VLAN — any Catalyst, and most switches that index a community —
   ask for `1.3.6.1.2.1.17` a second time naming a context, and record both. A single bridge walk
   cannot tell a device with an empty table apart from one that keeps its table somewhere the walk
   did not look, and that ambiguity is the whole of #686.

   ```bash
   snmpwalk -On -v2c -c <community> <ip> 1.3.6.1.2.1.1        # system, incl. sysServices
   snmpwalk -On -v2c -c <community> <ip> 1.3.6.1.2.1.2        # ifNumber + ifTable
   snmpwalk -On -v2c -c <community> <ip> 1.3.6.1.2.1.31.1.1   # ifXTable
   snmpwalk -On -v2c -c <community> <ip> 1.0.8802.1.1.2       # LLDP, local and remote
   snmpwalk -On -v2c -c <community> <ip> 1.3.111.2.802.1.1.13 # LLDP-V2, for anything serving it
   snmpwalk -On -v2c -c <community> <ip> 1.3.6.1.2.1.17       # bridge: ports, FDB, VLANs
   snmpwalk -On -v3 -l authPriv -u <user> -a SHA -A <pass> -x AES -X <pass> \
     -n <context> <ip> 1.3.6.1.2.1.17                         # and again, naming a context
   ```

2. **Write the device.** Copy the nearest existing module in `sim/devices/`, give it a `Purpose`
   naming the issue and what breaks without it, and add it to `all()` in `sim/devices/mod.rs`.
   `Purpose` is required: a device with no established defect must say `Purpose::Control`.

   **Rewrite identifiers consistently** — same value, same replacement, everywhere. MACs, the
   management address and neighbour hostnames all carry customer information, and all of them are
   what resolution keys on. Rewriting them per-occurrence instead of per-value breaks the
   cross-table joins that make the fixture worth having. **Never commit a captured walk.**

3. **Write the test that fails**, beside the device, driving the real collection path:

   ```rust
   let scan = harness::scan("switch-new-01").await;
   assert_eq!(scan.arp.records.len(), 45, "GH #674: a strict walk stops at 40");
   ```

   Assert the **outcome the issue was about**, never that the fixture equals itself. `harness::scan`
   runs the daemon's own queries in the order `execute` uses and hands back what they read.

4. **Fix the code**, and watch the test go green.

5. `make snmp-deploy`, then `make snmp-verify`.

### What you no longer have to remember

These were review instructions in this document. They are now properties of the model, and the
listed unit tests fail if one is broken:

| Was | Now |
|---|---|
| "Keep each data file ascending by OID" | Derived by `DataFile::render`. `every_file_ascends_except_the_one_that_must_not` |
| "Do not write `ifNumber` or `dot1dBaseNumPorts`" | Derived from the rows. `adding_a_port_moves_the_declared_count_with_it` |
| "Send a MAC with `octet`, never `string`" | A MAC is a `MacAddress`; the encoding is `MacEncoding` and must be named. `port_macs_are_octets_unless_the_device_says_otherwise` |
| "A data file no config serves / a config naming a file nobody wrote" | Registrations are derived from the tables held. `every_served_file_has_a_registration_and_vice_versa` |
| "An ifTable served without its `ifNumber` registration" | Derived. `a_device_serving_an_if_table_registers_its_own_count` |
| "Record what the fixture is for" | `Purpose` is a required field |

Two things the type system cannot check, and that still need care:

- **Confirm every far end exists.** Check the neighbour's `chassis_id`, `if_name` and `if_index`
  against the device it points at before adding it. A neighbour that resolves through a fallback
  tier looks identical to one that resolves properly, and only the second is evidence. An earlier
  revision of `.240` used a made-up chassis MAC and a port `switch-core-01` does not have; both
  still appeared to work, so the profile passed without exercising what it documents.
- **`ifPhysAddress` is octets; an LLDP identifier may legitimately be either.** `value_to_mac`
  accepts only six raw bytes, so a port MAC sent as text is silently dropped. `parse_mac_id`
  accepts both, deliberately, because real firmware sends both — `switch-dlink-01` sends raw
  octets, `switch-tplink-01` uppercase ASCII, and `switch-exos-01`'s own chassis id is left
  *abbreviated* as the standing guard on the unpadded form. Do not "tidy" those.

## Expect truncation warnings — the simulator races itself

A scan of this environment normally reports several incomplete SNMP walks. **That is the simulator, not the product under test.**

`snmpd` forks the `pass` handler — a bash script that then forks awk — once per SNMP request. With 22 agents on one VM and ~17 column walks per host, a single scan is hundreds of concurrent forks, and under that load the agents answer some requests with the *wrong* OID: one belonging to a request the daemon made earlier.

Measured 2026-07-27, walking all 12 v2c devices from a single client:

| | truncated |
|---|---|
| serial | **0** of 12 |
| concurrent | **4–5** of 12, a *different* set of devices each run |

Every truncation was a stale response — an in-subtree walk answered with an OID *lower* than the one requested (asking for `lldpRemChassisId` and getting `lldpRemChassisIdSubtype`; asking within ifXTable and being handed an LLDP OID that sorts below the entire subtree). A correct agent walking forward cannot produce that, and it is not the client desyncing: the responses pass request-id and community validation, each session owns its own connected socket and request-id range, and the identical walks are clean run serially.

### The daemon now recovers from it (2026-07-31)

Re-measured after the walk gained a bounded re-ask on a misdirected answer:

| | truncated | re-asks |
|---|---|---|
| concurrent, 2 scans | **0** | **13**, every one `StaleResponse` |

Re-asking is safe because a wrong-OID response is rejected before it reaches the row callback, so nothing from it was collected and the retry cannot duplicate rows. Bounded at two attempts, so an agent answering persistently out of step still reports as truncated rather than spinning the scan.

**The simulator is still doing exactly what it did** — 13 misdirected answers across two scans, on a different set of devices each time. What changed is that the daemon no longer converts them into lost columns. Judge future changes against *0 truncated*, and treat a rise in the re-ask count as the simulator being busier rather than the product regressing.

Note this arrived by way of a null result worth remembering: an earlier retry keyed on `snmp2::Error::RequestIdMismatch` — a transport error where the session rejects the datagram — and moved these numbers not at all, because the simulator never produces that. A customer's switches did. Two environments, two different faults; the first fix was aimed at the wrong one.

The remaining `set_complete=false` results here are a *different* simulator misbehaviour: columns answering for ifIndexes the device never listed (`foreign_rows=1, 7, 19` in one two-scan window). Those rows are discarded and reported separately, so warning count is not a clean proxy for truncation — count `SNMP walk truncated` lines instead.

### Control: a correct agent under load (2026-07-31)

The agents here are deliberately incorrect, so they cannot answer "is the *scanner* losing data?". A control agent settles it: one `snmpd` with **no `pass` directives at all**, serving the built-in MIBs from real kernel state, on its own macvlan behind `tc netem`.

| impairment | rescans | truncations | re-asks | ifTable |
|---|---|---|---|---|
| none | 5 | 0 | 0 | 17/17, complete |
| 200 ms delay, 2% loss | 6 | 0 | 0 | 17/17, complete |
| 400 ms delay, 15% loss | 6 | 0 | 0 | 17/17, complete |

Against an agent that is correct by construction, the scanner loses nothing — including at loss rates well past anything a LAN produces. (At 800 ms / 55% loss only 1 of 3 rescans got past the probe at all, so that arm proves little either way.)

To rebuild it: a macvlan on the parent interface, a conf file with `agentAddress`, `rocommunity` and the `sys*` values and **nothing else**, a systemd unit running `snmpd -f -Lo -C -c <conf>` — note *without* the `-I -ifTable,-ifXTable` the other units carry, since here the built-in implementations are the point — then `tc qdisc replace dev <iface> root netem delay Xms loss Y%`. Remove the qdisc, unit, conf and macvlan afterwards.

**How to read a scan.** Judge a change by whether *data* was lost — interfaces pruned, neighbours wiped, links frozen — not by whether warnings appeared. A warning saying previously discovered values "were kept rather than overwritten" is the daemon handling the chaos correctly.

**Worth keeping.** This free adversarial agent surfaced three real defects in July 2026: a foreign interface appearing on a switch, a chassis ID overwritten with NULL leaving a link permanently unresolvable, and a truncated column reported as authoritative. If the noise ever needs quieting, `pass_persist` replaces the fork-per-request with one long-lived handler per agent — but leave a device or two on `pass` deliberately, or the environment loses the property that found those bugs.

**What a scan exercises (session-reuse + getbulk).** Every device is scanned with a single reused SNMP session across all ~11 queries (one v3 engine discovery instead of ~12), and each table is walked with `getbulk` (v1 falls back to `getnext`). To make the getbulk walks land on real data for the subtrees stock `snmpd` does **not** implement:
- **switch-core-01** additionally serves BRIDGE-MIB / Q-BRIDGE (`dot1dBasePortIfIndex`, `dot1qVlanStaticName` → VLANs "DATA"/"VOICE", `dot1qPvid`), ENTITY-MIB (chassis inventory) and CDP (a `router-gw-01` neighbour) — exercising those getbulk walks end-to-end.
- **legacy-switch-01 (v1-only)** additionally serves a small bridge table, so the **getbulk → getnext fallback** is exercised on a non-ifTable walk, not just ifTable/LLDP.
- **Every device whose `sysServices` sets the datalink bit serves `dot1dBasePortIfIndex`**, generated from its own ethernetCsmacd interfaces (see *Self-reported counts* below). A switch that says it bridges and answers the bridge MIB with `noSuchObject` is a device contradicting itself, which the daemon now reports — so a fixture in that state is a false positive waiting to happen, not a neutral omission.
- `ipAddrTable` and `ipNetToMedia` (ARP) are answered by snmpd's built-in IP module, so those walks run on every device already. **ARP MAC rows are therefore not simulated** — that subtree cannot be displaced (see the `.4.20` note below), so the addresses come from the VM's kernel and no `PhysAddress` fixture can reach them.

> **Bridge FDB rows are simulated, and were not until August 2026.** An earlier revision of this document said `pass` could not carry binary MACs and that FDB *and* ARP rows were therefore out of reach. That is true of ARP, for the registration reason above, and was never true of the FDB: `pass` emits binary through type `octet`, which is how every `ifPhysAddress` and LLDP chassis id here already sends six raw bytes. The consequence of believing otherwise is GH #686 — a bridge-FDB defect reported against a lab with no FDB row in it, on a table nothing here had ever walked with data behind it.
- **ap-wireless-01** is the one exception: it serves its own `ipAddrTable` so it can advertise a second subnet (see below).

**Access-point guest subnet (`.235`) — #663.** The built-in IP module answers `ipAddrTable` from the VM's real kernel state, so every other agent only ever reports addresses inside the scanned `192.168.4.0/22`. `ap-wireless-01` overrides it and serves the table from `ap-wireless-01-ipaddr.txt`, advertising **172.30.10.1/24 on ifIndex 4**, whose `ifName` is **`br-guest`** — the built-in NAT guest network of a real access point.

> Unlike ifTable/ifXTable, this subtree **cannot** be freed by disabling its module: `-I -ipaddr` (or `-ipAddr`) does not stop `mibII/ipaddr` registering. It also registers per *column* (`.4.20.1.1`…`.4.20.1.5`), so a single `pass` at the `.4.20` root always loses on specificity, whatever priority it carries. The override therefore registers one `pass -p 1` per column — matching granularity and beating the default priority of 255. Confirm what owns a subtree with `snmpd -Dregister_mib -C -c <conf>`.

That combination is what issue #663 reported: a `br-` prefixed `ifName` on a remote device used to be classified as a Docker bridge, so the AP's guest subnet rendered as "Docker @ *AP*" in Topology. A scan of `.235` should now discover `172.30.10.0/24` as a **Guest** subnet, with no Docker/container label anywhere.

Because `.235` is the only agent serving its own `ipAddrTable`, it is also the only one that can fail *silently* — if the module displacement doesn't take, the `pass` directive loses the duplicate registration and the agent quietly reports just the scanned subnet. `make snmp-verify` checks this fixture explicitly for that reason; don't run a scan against it until that check passes.

The two version-locked hosts use net-snmp VACM/USM so the other protocol versions are genuinely refused (a plain `rocommunity` answers both v1 and v2c, which wouldn't prove version negotiation):

- **legacy-switch-01 (v1 only):** VACM grants access only via the v1 security model — v2c/v3 are denied.
- **secure-switch-01 (v3 only):** USM user `scanopyv3`, AuthPriv, **SHA-256 / AES-128**, auth password `authpass12345`, priv password `privpass12345`. No `rocommunity`, so v1/v2c are denied.

> **AES-256 note:** the v3 host uses AES-128, which stock Debian/Ubuntu net-snmp supports out of the box. AES-256 (`createUser … AES-256`) requires net-snmp built with Blumenthal AES (`--enable-blumenthal-aes`); change `createUser`/the verify command in `lxc/setup.sh` only if your build supports it.

## Credentials

The devices deliberately span five credentials so a scan exercises credential selection, the v1/v2c/v3 negotiation paths, and the "try the next credential" fallback rather than one community answering everything. Seed all five into the dev database with:

```bash
make snmp-seed-credentials
```

It assigns each one to **every network in the database** (Broadcast scope — the only option that works before a scan, since PerHost assignment needs hosts that don't exist yet), and is idempotent: re-running updates the existing rows rather than accumulating duplicates. If it reports `networks | 0`, create a network first — nothing was seeded.

The credential values live in `backend/scripts/seed-snmp-credentials.sql` and must stay in step with the community strings in `lxc/setup.sh`.

## Setup

Paste the contents of `tools/snmp/lxc/setup.sh` into a root shell on a Debian/Ubuntu VM with primary IP 192.168.7.230/22.

Before pasting, verify:
- Interface is `eth0` (`ip link`) — edit `IFACE=` if different
- Primary IP is 192.168.7.230 — edit `HOSTS=()` if different

## Patch: migrate secondary IPs to macvlan (unique MACs)

If each device shares the host's MAC (secondary IPs on eth0), run on the VM:

```bash
IFACE=eth0; CIDR=22; HOSTS=(192.168.7.230 192.168.7.231 192.168.7.232 192.168.7.233 192.168.7.234 192.168.7.235 192.168.7.236 192.168.7.237 192.168.7.238 192.168.7.239 192.168.7.240 192.168.7.241 192.168.7.242); for i in "${!HOSTS[@]}"; do ip addr del "${HOSTS[$i]}/$CIDR" dev "$IFACE" 2>/dev/null; ip link del "mv-snmp${i}" 2>/dev/null; ip link add "mv-snmp${i}" link "$IFACE" type macvlan mode bridge; ip addr add "${HOSTS[$i]}/$CIDR" dev "mv-snmp${i}"; ip link set "mv-snmp${i}" up; done && sysctl -w net.ipv4.conf.all.arp_ignore=1 net.ipv4.conf.all.arp_announce=2 && for i in "${!HOSTS[@]}"; do sysctl -w net.ipv4.conf.mv-snmp${i}.arp_ignore=1 net.ipv4.conf.mv-snmp${i}.arp_announce=2; done && sysctl -w net.ipv4.conf.${IFACE}.arp_ignore=1 net.ipv4.conf.${IFACE}.arp_announce=2
```

Then flush the ARP cache on the scanning host (`sudo arp -a -d` on macOS).

## Patch: fix duplicate MIB registration

If snmpd logs show `duplicate registration: MIB modules ifTable and pass`, run:

```bash
for f in /etc/systemd/system/snmpd-*.service; do sed -i 's|snmpd -f -Lo -C|snmpd -f -Lo -I -ifTable,-ifXTable -C|' "$f"; done && systemctl daemon-reload && for f in /etc/systemd/system/snmpd-*.service; do systemctl restart "$(basename "$f" .service)"; done
```

## Updating an already-running VM

`lxc/setup.sh` is idempotent — existing macvlan interfaces are left alone, while MIB data files, snmpd configs and systemd units are rewritten and every agent is restarted. So a full re-run is always the update path; there is no separate partial script.

```bash
ssh -i ~/.ssh/snmp-test-vm root@192.168.7.230 'rm -rf /root/snmp-test' \
  && scp -i ~/.ssh/snmp-test-vm -r tools/snmp root@192.168.7.230:/root/snmp-test \
  && ssh -i ~/.ssh/snmp-test-vm root@192.168.7.230 'bash /root/snmp-test/lxc/setup.sh'
```

Hosts that gained nothing are effectively no-ops; anything whose data file, config or unit changed comes back with the new content.

> **The `rm -rf` is required, not tidiness.** `scp -r tools/snmp <host>:/root/snmp-test` only lands at that path the *first* time. Once `/root/snmp-test` exists, scp copies *into* it — the new tree lands at `/root/snmp-test/snmp/` while `bash /root/snmp-test/lxc/setup.sh` re-runs the **stale** copy. Every agent restarts and the run reports success, so this fails silently and looks like a broken fixture rather than a stale deploy. Sanity-check with `grep -c br-guest /root/snmp-test/lxc/setup.sh` before running it.

> **SSH key.** The VM accepts publickey only (password auth is disabled) and there is no `~/.ssh/config` entry, so `-i ~/.ssh/snmp-test-vm` is required or you get `Permission denied (publickey)`. Add a `Host 192.168.7.2*` / `IdentityFile ~/.ssh/snmp-test-vm` block to `~/.ssh/config` to drop the flag.

Afterwards, flush the scanning host's ARP cache (`sudo arp -a -d` on macOS) so any new MACs are learned, then run `make snmp-verify` from your Mac.

> Re-running is required after any change to the MIB data or a systemd unit — including the `ap-wireless-01` guest-subnet fixture (#663), which changes both its `ipAddrTable` data and its `ExecStart` module exclusions.

## Verify

**Verify from an external host (e.g. your Mac), not the VM itself.** The agents bind to macvlan interfaces, and the Linux kernel won't let the VM reach its own macvlan child interfaces — so `snmpget` from the VM to `192.168.7.x` always fails even when everything is healthy. `setup.sh` therefore only checks systemd service health locally and prints a reminder to verify externally.

From your Mac:

```bash
make snmp-verify
```

Or manually — note the per-version flags:

```bash
# v2c
snmpget -v2c -c secret42 -t 2 -r 1 192.168.7.232 sysName.0
# v1 (legacy-switch-01)
snmpget -v1 -c legacyv1 -t 2 -r 1 192.168.7.236 sysName.0
# v3 (secure-switch-01) — SHA-256 / AES-128 AuthPriv
snmpget -v3 -l authPriv -u scanopyv3 -a SHA-256 -A authpass12345 -x AES -X privpass12345 -t 2 -r 1 192.168.7.237 sysName.0
```

To prove the version lock, confirm the wrong version is refused:

```bash
snmpget -v2c -c legacyv1 192.168.7.236 sysName.0   # should time out (v1-only)
snmpget -v2c -c public   192.168.7.237 sysName.0   # should time out (v3-only)
```

## Manage services

```bash
# On the VM
systemctl status snmpd-router-gw-01
journalctl -u snmpd-router-gw-01 --no-pager -n 20
systemctl restart snmpd-router-gw-01
```
