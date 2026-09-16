//! The simulated SNMP devices, defined once as typed values.
//!
//! Every device in `tools/snmp/` lives here and nowhere else. The deployment generates its data
//! files and agent configs from these definitions rather than carrying a second copy, and each
//! device's regression test drives the real collection path against it — so a fixture that stops
//! exercising the defect it exists for fails a test rather than passing quietly.
//!
//! Compiled under `cfg(test)` and behind the `snmp-sim` feature, which the fixture generator
//! enables; the shipped `server` and `daemon` binaries carry none of it.

pub mod allocation;
pub mod devices;
pub mod emit;
pub mod harness;
pub mod lldp;
pub mod mibs;
pub mod tables;
pub mod transport;
pub mod wire;

use std::net::Ipv4Addr;

use transport::{BulkRejection, Handler, Registration, SimAgent};
use wire::{DataFile, Ordering};

use crate::daemon::discovery::integration::snmp::oids::{
    arp, bridge, cdp, entity, if_mib, ip_mib, lldp as lldp_oids, oid_parts,
};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::CredentialType;

/// Why a device is in the lab.
///
/// Required rather than optional: a device with no established defect has to say so. "A fixture
/// whose purpose is not written down is one nobody dares change and nobody can verify" was a
/// review instruction in `SNMP-TEST-ENV.md`; here it is not expressible.
#[derive(Debug, Clone, Copy)]
pub enum Purpose {
    /// Reproduces a specific reported defect. `issue` names the GitHub issue or the report it
    /// came from; `defect` says what goes wrong when the guard is absent.
    Regression {
        issue: &'static str,
        defect: &'static str,
    },
    /// No defect of its own: a far end other devices resolve against, or a healthy baseline that
    /// demonstrates a check staying quiet.
    Control { role: &'static str },
}

/// Every MIB a device serves.
#[derive(Debug, Clone, Default)]
pub struct Tables {
    pub if_table: Option<tables::IfTable>,
    pub lldp: Option<lldp::LldpTable>,
    pub bridge: mibs::BridgeTable,
    pub arp: mibs::ArpTable,
    /// Addresses this device serves *beyond* its own. Its own address is never set here — every
    /// device serves it automatically, at [`Tables::own_address`].
    pub ip_addr: mibs::IpAddrTable,
    /// Where this device's own address rides in `ipAddrTable` — `ifIndex` and netmask. See
    /// [`mibs::OwnAddress`]'s default for why the default `ifIndex` is not the device's first
    /// port.
    pub own_address: mibs::OwnAddress,
    pub entity: mibs::EntityTable,
    pub cdp: mibs::CdpTable,
    /// Alternative LLDP files written alongside the active one and swapped in by hand. Only
    /// `switch-flaky-01` has these; each drives a different discard counter.
    pub lldp_variants: Vec<(&'static str, lldp::LldpTable)>,
    /// A second bridge table served in a named SNMP context, through a proxied back-end agent.
    /// Only `switch-cisco-01`, which is what GH #686 is about.
    pub context_bridge: Option<mibs::BridgeTable>,
}

/// A simulated device: everything the deployment and the tests need, in one value.
#[derive(Debug, Clone)]
pub struct SimDevice {
    /// `sysName`, and the prefix of every file this device serves.
    pub name: &'static str,
    pub ip: Ipv4Addr,
    pub purpose: Purpose,
    pub credential: CredentialType,
    pub system: SystemInfo,
    pub tables: Tables,
    /// Which `pass` handler serves the ARP table. Two devices deliberately misbehave here.
    pub arp_handler: Handler,
    /// Subtrees this device registers against an empty file, to stop net-snmp answering them.
    ///
    /// `ifTable`/`ifXTable` are suppressed by the `-I` flag every unit carries, but `ipAddrTable`
    /// and `ipNetToMediaTable` cannot be — without these overrides a device that is supposed to
    /// serve nothing would report the host's own addresses and ARP cache and would not be mute.
    pub suppresses: Vec<&'static str>,
    /// A GETBULK above a repetition count answered with an error status, by
    /// `snmp-bulk-refuser.py --reject-above` in front of the agent. `None` for every device but
    /// `switch-hikvision-01` (GH #710).
    pub rejects_getbulk: Option<BulkRejection>,
}

/// The file suffixes, kept here so the deployment and the registrations cannot disagree about
/// what a device's files are called.
const IFTABLE: &str = "iftable";
const BRIDGE: &str = "bridge";
const ARP: &str = "arp";
const IPADDR: &str = "ipaddr";
const ENTITY: &str = "entity";
const CDP: &str = "cdp";
pub(crate) const VLAN20: &str = "vlan20";
/// `lldpLocPortTable`, split out for a device that serves it through a handler of its own.
const LOCPORTS: &str = "locports";

/// A scalar's *object* OID: the constant without its trailing instance sub-id.
///
/// The MIB constants name instances (`lldpLocChassisId.0`), which is what a walk asks for, but
/// `pass` registers a subtree. Registering the instance leaves the object invisible to a GETNEXT
/// arriving from above, so the scalar is only reachable by a caller that already knows its exact
/// OID.
fn scalar_object(oid: &str) -> Vec<u64> {
    let mut parts = oid_parts(oid);
    parts.pop();
    parts
}
const EMPTY: &str = "empty";

impl SimDevice {
    fn file(&self, suffix: &str, ordering: Ordering, rows: Vec<wire::Row>) -> DataFile {
        DataFile::new(format!("{}-{}", self.name, suffix), ordering, rows)
    }

    fn ethernet_if_indexes(&self) -> Vec<i32> {
        self.tables
            .if_table
            .as_ref()
            .map(tables::IfTable::ethernet_if_indexes)
            .unwrap_or_default()
    }

    /// Whether this device deliberately serves no `ipAddrTable` at all — `switch-mute-01`, which
    /// already registers the column against an empty file via [`Self::suppresses`]. Own-address
    /// synthesis has to defer to that override rather than fight it for the same registration.
    fn suppresses_ip_addr_table(&self) -> bool {
        self.suppresses
            .contains(&ip_mib::ip_addr_entry::IP_AD_ENT_ADDR)
    }

    /// Subtrees this device refuses a GETBULK for, as `snmp-bulk-refuser.py` is configured.
    ///
    /// Empty for every device but the one that needs it, and empty means no shim: the agent binds
    /// its own address directly and nothing sits in front of it.
    pub fn refuses_getbulk(&self) -> Vec<Vec<u64>> {
        self.tables
            .lldp
            .as_ref()
            .filter(|table| table.neighbours_refuse_getbulk)
            .map(|table| vec![oid_parts(table.mib.remote_root)])
            .unwrap_or_default()
    }

    /// Subtrees this device answers nothing for, as `snmp-bulk-refuser.py --silence` is configured.
    pub fn silenced(&self) -> Vec<Vec<u64>> {
        self.tables
            .lldp
            .as_ref()
            .and_then(|table| table.silent_column.map(|column| column(&table.mib.remote)))
            .map(|oid| vec![oid_parts(oid)])
            .unwrap_or_default()
    }

    /// The `pass` data files this device serves, in a fixed order that the registrations index
    /// into.
    pub fn data_files(&self) -> Vec<DataFile> {
        let mut files = Vec::new();
        if let Some(if_table) = &self.tables.if_table {
            files.push(self.file(IFTABLE, Ordering::Ascending, if_table.wire_rows()));
        }
        if let Some(table) = &self.tables.lldp {
            // A device with swappable variants serves an `-active` slot rather than a fixed file:
            // the handler re-reads its file per request, so copying a variant over it takes effect
            // with no snmpd restart.
            let suffix = if self.tables.lldp_variants.is_empty() {
                table.mib.file_suffix.to_string()
            } else {
                format!("{}-active", table.mib.file_suffix)
            };
            let mut rows = table.wire_rows();
            if table.splits_registrations() {
                // A sub-table served by its own handler needs its own file, because a `pass`
                // script only ever sees the file it was given. `snmp-pass-handler-stuck.sh` reads
                // `head -1`, so pointed at the whole MIB it would answer with a local-system
                // scalar, net-snmp would reject that as outside the registration, and the device
                // would report an *empty* port table rather than a truncated one — a different
                // fault with a different warning.
                let port_table = oid_parts(table.mib.local_port_table);
                let split = rows
                    .iter()
                    .filter(|row| row.oid.starts_with(&port_table))
                    .cloned()
                    .collect();
                rows.retain(|row| !row.oid.starts_with(&port_table));
                files.push(self.file(&format!("{suffix}-{LOCPORTS}"), Ordering::Ascending, split));
            }
            files.push(self.file(&suffix, Ordering::Ascending, rows));
        }
        if !self.tables.bridge.is_empty() {
            let rows = self.tables.bridge.wire_rows(&self.ethernet_if_indexes());
            files.push(self.file(BRIDGE, Ordering::Ascending, rows));
        }
        if !self.tables.arp.is_empty() {
            // The one device whose table is served out of ascending order keeps the order it was
            // written in; everything else is sorted.
            let ordering = match self.arp_handler {
                Handler::Positional => Ordering::Positional,
                _ => Ordering::Ascending,
            };
            files.push(self.file(ARP, ordering, self.tables.arp.wire_rows()));
        }
        if !self.suppresses_ip_addr_table() {
            let mut rows = vec![mibs::IpAddrRow {
                address: self.ip,
                if_index: self.tables.own_address.if_index,
                netmask: self.tables.own_address.netmask,
            }];
            rows.extend(self.tables.ip_addr.rows.iter().cloned());
            let table = mibs::IpAddrTable { rows };
            files.push(self.file(IPADDR, Ordering::Ascending, table.wire_rows()));
        }
        if !self.tables.entity.is_empty() {
            files.push(self.file(ENTITY, Ordering::Ascending, self.tables.entity.wire_rows()));
        }
        if !self.tables.cdp.is_empty() {
            files.push(self.file(CDP, Ordering::Ascending, self.tables.cdp.wire_rows()));
        }
        if !self.suppresses.is_empty() {
            files.push(self.file(EMPTY, Ordering::Ascending, Vec::new()));
        }
        files
    }

    /// The file the context back end serves.
    ///
    /// `pass` takes no context argument — it registers into the default context and nothing else —
    /// so a handler cannot be scoped to a context directly. A second snmpd on loopback holds this
    /// table and the front agent reaches it with `proxy -Cn vlan-20`. It is genuinely two agents,
    /// which is why this is not one of [`Self::data_files`].
    pub fn context_files(&self) -> Vec<DataFile> {
        self.tables
            .context_bridge
            .iter()
            .map(|context| {
                self.file(
                    VLAN20,
                    Ordering::Ascending,
                    context.wire_rows(&self.ethernet_if_indexes()),
                )
            })
            .collect()
    }

    /// The back end's own registration: the whole bridge MIB, from its own file.
    pub fn context_registrations(&self) -> Vec<Registration> {
        self.context_files()
            .iter()
            .enumerate()
            .map(|(file, _)| Registration {
                subtree: oid_parts(bridge::BRIDGE_MIB),
                file,
                handler: Handler::Normal,
            })
            .collect()
    }

    /// The swappable LLDP variants, written beside the active file but served by nobody until
    /// copied over it. Deliberately not in [`Self::data_files`]: they are not part of the
    /// device's normal answer.
    pub fn variant_files(&self) -> Vec<DataFile> {
        self.tables
            .lldp_variants
            .iter()
            .map(|(name, table)| {
                DataFile::new(
                    format!("{}-{}-{}", self.name, table.mib.file_suffix, name),
                    Ordering::Ascending,
                    table.wire_rows(),
                )
            })
            .collect()
    }

    /// The `pass` registrations this device's snmpd config carries.
    ///
    /// Derived from the tables the device actually holds, which is what retires the deploy
    /// script's section 5c: a config cannot name a data file nobody wrote, a data file cannot go
    /// unserved, and a device serving an ifTable cannot forget to register `ifNumber`, because
    /// none of those is written down separately any more.
    pub fn registrations(&self) -> Vec<Registration> {
        let files = self.data_files();
        let index = |suffix: &str| {
            let wanted = format!("{}-{}", self.name, suffix);
            files.iter().position(|f| f.name == wanted)
        };
        let mut registrations = Vec::new();

        if let Some(file) = index(IFTABLE) {
            // `mibII/interfaces` owns this scalar, so without its own registration the device
            // answers its interface count from the host's kernel state — a contradiction reported
            // on every scan for a fault it does not have.
            registrations.push(Registration {
                subtree: oid_parts(if_mib::IF_NUMBER_OBJECT),
                file,
                handler: Handler::Normal,
            });
            registrations.push(Registration {
                subtree: oid_parts(if_mib::IF_TABLE),
                file,
                handler: Handler::Normal,
            });
            if self
                .tables
                .if_table
                .as_ref()
                .is_some_and(tables::IfTable::serves_if_x_table)
            {
                registrations.push(Registration {
                    subtree: oid_parts(if_mib::if_x_table::IF_X_TABLE),
                    file,
                    handler: Handler::Normal,
                });
            }
        }
        // The LLDP subtree is the table's own — a device serving a MIB other than the classic
        // one registers that root instead, and nothing else here has to know.
        let lldp_mib = self.tables.lldp.as_ref().map(|t| t.mib);
        let lldp_file = match (&lldp_mib, self.tables.lldp_variants.is_empty()) {
            (Some(mib), true) => mib.file_suffix.to_string(),
            (Some(mib), false) => format!("{}-active", mib.file_suffix),
            (None, _) => String::new(),
        };
        let lldp_splits = self
            .tables
            .lldp
            .as_ref()
            .is_some_and(lldp::LldpTable::splits_registrations);
        for (suffix, subtree) in [
            (
                // Skipped where the sub-tables are registered separately below: this line covers
                // all of them, and covering them is exactly what stops them being served.
                if lldp_splits { "" } else { lldp_file.as_str() },
                lldp_mib.map(|m| m.root).unwrap_or(lldp_oids::LLDP_MIB),
            ),
            (BRIDGE, bridge::BRIDGE_MIB),
            (ENTITY, entity::ENTITY_MIB),
            (CDP, cdp::CDP_MIB),
        ] {
            if let Some(file) = index(suffix) {
                registrations.push(Registration {
                    subtree: oid_parts(subtree),
                    file,
                    handler: Handler::Normal,
                });
            }
        }
        // A device that serves one LLDP sub-table differently carves the MIB into disjoint pieces
        // instead of registering the root: the local-system scalars one by one, then the port
        // table, then the remote tables. Emitted only when a handler is non-default, so every
        // other device's config is the single root line it always was.
        //
        // Disjoint is not tidiness. net-snmp's `pass` serves only the broadest of a set of
        // overlapping lines and silently ignores the nested ones — a root line plus `…1.3.7` plus
        // `…1.4` answered everything from the root, so neither sub-table handler ever ran and the
        // rows that had moved into the port table's own file were served by nobody. The device
        // reported `No Such Instance` for a table it was holding, and the model, which resolved
        // the nesting, said it was fine. `no_registration_is_nested_inside_another` is what stops
        // that coming back.
        if let (Some(table), Some(file)) = (self.tables.lldp.as_ref(), index(&lldp_file))
            && table.splits_registrations()
        {
            let port_file = index(&format!("{lldp_file}-{LOCPORTS}")).unwrap_or(file);
            let local = &table.mib.local;
            for scalar in [
                local.chassis_id_subtype,
                local.chassis_id,
                local.sys_name,
                local.sys_desc,
            ] {
                registrations.push(Registration {
                    subtree: scalar_object(scalar),
                    file,
                    handler: Handler::Normal,
                });
            }
            registrations.push(Registration {
                subtree: oid_parts(table.mib.local_port_table),
                file: port_file,
                handler: table.local_port_handler,
            });
            registrations.push(Registration {
                subtree: oid_parts(table.mib.remote_root),
                file,
                handler: Handler::Normal,
            });
        }
        if let Some(file) = index(IPADDR) {
            // Column by column, not as one subtree. net-snmp's own IP module owns `ipAddrTable`,
            // and a whole-table `pass` loses the duplicate registration: the agent then quietly
            // answers from the VM's addresses instead of the fixture's, which is a device that
            // looks healthy and is not the device under test (GH #663).
            for column in [
                ip_mib::ip_addr_entry::IP_AD_ENT_ADDR,
                ip_mib::ip_addr_entry::IP_AD_ENT_IF_INDEX,
                ip_mib::ip_addr_entry::IP_AD_ENT_NET_MASK,
            ] {
                registrations.push(Registration {
                    subtree: oid_parts(column),
                    file,
                    handler: Handler::Normal,
                });
            }
        }
        if let Some(file) = index(EMPTY) {
            for subtree in &self.suppresses {
                registrations.push(Registration {
                    subtree: oid_parts(subtree),
                    file,
                    handler: Handler::Normal,
                });
            }
        }
        if let Some(file) = index(ARP) {
            // Registered column by column rather than as one subtree, because the two misbehaving
            // handlers answer per column and a whole-table registration would let one column's
            // answer stand in for another's.
            for column in [
                arp::entry::IP_NET_TO_MEDIA_IF_INDEX,
                arp::entry::IP_NET_TO_MEDIA_PHYS_ADDRESS,
                arp::entry::IP_NET_TO_MEDIA_NET_ADDRESS,
                arp::entry::IP_NET_TO_MEDIA_TYPE,
            ] {
                registrations.push(Registration {
                    subtree: oid_parts(column),
                    file,
                    handler: self.arp_handler,
                });
            }
        }
        registrations
    }

    /// Registrations for the LLDP subtree alone, against a single file.
    ///
    /// For serving one of the swappable variants: the deployed device does this by copying a
    /// variant over `-lldp-active.txt`, and a test does it by handing the variant's rows to an
    /// agent that serves only that subtree.
    pub fn registrations_for_lldp_only(&self) -> Vec<Registration> {
        let root = self
            .tables
            .lldp
            .as_ref()
            .map(|t| t.mib.root)
            .unwrap_or(lldp_oids::LLDP_MIB);
        vec![Registration {
            subtree: oid_parts(root),
            file: 0,
            handler: Handler::Normal,
        }]
    }

    /// This device answering SNMP, for driving the real collection path against.
    ///
    /// Whether it offers getbulk follows from its credential rather than being set per device:
    /// SNMPv1 has no getbulk, so the v1 agent forces every column through getnext.
    pub fn agent(&self) -> SimAgent {
        let agent = SimAgent::new(&self.data_files(), self.registrations())
            .refusing_getbulk(self.refuses_getbulk())
            .silent_on(self.silenced())
            .rejecting_getbulk_above(self.rejects_getbulk);
        match self.credential {
            CredentialType::SnmpV1 { .. } => agent.without_getbulk(),
            _ => agent,
        }
    }

    /// The context back end answering SNMP, for the one device that has one.
    pub fn context_agent(&self) -> Option<SimAgent> {
        let files = self.context_files();
        if files.is_empty() {
            return None;
        }
        Some(SimAgent::new(&files, self.context_registrations()))
    }

    /// How many LLDP neighbours this device is *expected* to lose to a local port that names no
    /// interface, because losing them is the defect it reproduces.
    ///
    /// Zero for every device but one, and the default is what makes the lab-wide check strict: a
    /// new fixture that drops a neighbour fails rather than being quietly excused. The exemption
    /// is a count rather than a flag so the fixture still has to lose *exactly* what it claims —
    /// if `switch-shortports-01` ever stops truncating its `lldpLocPortTable`, the check notices.
    pub fn expected_unplaceable_neighbours(&self) -> usize {
        match self.name {
            // GH #668's switch4. Its `lldpLocPortTable` read stops part way, which is the whole
            // defect: the ports its two neighbours sit on are in the half that never arrives, so
            // both reach no interface. A device that placed them would not reproduce the report.
            "switch-shortports-01" => 2,
            _ => 0,
        }
    }

    /// `ifNumber.0` as this device publishes it, or `None` where it serves no ifTable.
    pub fn declared_if_number(&self) -> Option<i32> {
        self.tables
            .if_table
            .as_ref()
            .map(tables::IfTable::declared_count)
    }
}

/// Every device in the lab, addressed and with every peer reference resolved.
pub fn lab() -> Vec<SimDevice> {
    let mut devices = devices::all();
    allocation::assign_addresses(&mut devices);
    allocation::resolve_peer_addresses(&mut devices);
    devices
}

/// One device by name, for a test that wants to name what it is driving.
pub fn device(name: &str) -> SimDevice {
    lab()
        .into_iter()
        .find(|d| d.name == name)
        .unwrap_or_else(|| panic!("no simulated device named {name}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each device has its own address and its own name, and lives in the lab's subnet.
    ///
    /// Properties rather than a count. Adding a device is the documented workflow, so a test that
    /// pinned the size would fail on every legitimate addition and teach people to edit it without
    /// reading it — while still not catching the mistakes that matter. These do: two devices on one
    /// address collide on the VM's macvlan and only one answers, and an address outside the lab's
    /// range is provisioned and then never scanned.
    #[test]
    fn every_device_has_its_own_address_and_name() {
        let lab = lab();
        assert!(lab.len() >= 22, "the lab lost devices: {}", lab.len());

        let mut addresses: Vec<Ipv4Addr> = lab.iter().map(|d| d.ip).collect();
        addresses.sort();
        addresses.dedup();
        assert_eq!(addresses.len(), lab.len(), "two devices share an address");

        let mut names: Vec<&str> = lab.iter().map(|d| d.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), lab.len(), "two devices share a name");

        for device in &lab {
            let octets = device.ip.octets();
            assert!(
                octets[..3] == [192, 168, 7] && octets[3] >= allocation::FIRST_OCTET,
                "{} is at {}, outside the lab's reserved range",
                device.name,
                device.ip
            );
        }
    }

    /// Every agent reports its own address, plus whatever it declares as an extra one — and
    /// nothing else. No loopback, no VM management address, no other device's.
    ///
    /// Both directions matter and neither is redundant with the other: asserting only "nothing
    /// unexpected" would still pass a device whose own-address synthesis silently stopped
    /// running (an *empty* table has nothing unexpected in it either), and asserting only "the
    /// own address is there" would miss the actual leak this guards against — a device that
    /// serves its own address correctly but *also* falls through to the VM's kernel table.
    ///
    /// Lab-wide rather than per device, because the leak this guards is not a per-fixture defect:
    /// it is what every device gets by default unless its own address is explicitly synthesised
    /// into `ipAddrTable`, which is exactly the property this asserts. Covers the guest-subnet
    /// device (own + extra), the Windows fixture (own only, at a non-default ifIndex) and the
    /// mute device (deliberately suppressed, so an empty table is the one correct answer) with no
    /// special-casing — each is just a different shape of "exactly the allowed set".
    #[tokio::test]
    async fn every_device_serves_only_its_own_addresses() {
        for device in lab() {
            let own = std::net::IpAddr::V4(device.ip);
            let extra: Vec<std::net::IpAddr> = device
                .tables
                .ip_addr
                .rows
                .iter()
                .map(|row| std::net::IpAddr::V4(row.address))
                .collect();
            let allowed: std::collections::HashSet<std::net::IpAddr> =
                std::iter::once(own).chain(extra.iter().copied()).collect();

            let scan = harness::collect(&device).await;
            for addr in scan.ip_addr_table.keys() {
                assert!(
                    allowed.contains(addr),
                    "{} reports {addr}, which is neither its own address nor a declared extra one",
                    device.name
                );
            }
            if device.suppresses_ip_addr_table() {
                assert!(
                    scan.ip_addr_table.is_empty(),
                    "{} deliberately suppresses ipAddrTable but still reported one",
                    device.name
                );
            } else {
                assert!(
                    scan.ip_addr_table.contains_key(&own),
                    "{} did not report its own address at all",
                    device.name
                );
                for addr in &extra {
                    assert!(
                        scan.ip_addr_table.contains_key(addr),
                        "{} declared {addr} as an extra address but did not report it",
                        device.name
                    );
                }
            }
        }
    }

    /// No device registers a subtree inside another one it registers.
    ///
    /// net-snmp's `pass` serves only the broadest of a set of overlapping lines and silently
    /// ignores the nested ones — no error, no log, the config simply does less than it says. A
    /// device that registered its LLDP MIB root alongside `…1.3.7` and `…1.4` therefore served
    /// every LLDP request from the root line, and because the split had moved the port-table rows
    /// into their own file, that table came back `No Such Instance` from a device that was holding
    /// it. Two scans and a wire capture to find, and none of it visible from the definitions.
    ///
    /// A property over every device rather than an assertion about the one that got it wrong: the
    /// mistake is available to any device that wants to serve part of a MIB differently, and the
    /// answer is always to carve the MIB up rather than nest inside it.
    #[test]
    fn no_registration_is_nested_inside_another() {
        for device in lab() {
            let registrations = device.registrations();
            for (i, outer) in registrations.iter().enumerate() {
                for (j, inner) in registrations.iter().enumerate() {
                    if i == j {
                        continue;
                    }
                    assert!(
                        !(inner.subtree.len() > outer.subtree.len()
                            && inner.subtree.starts_with(&outer.subtree)),
                        "{} registers {:?} inside {:?}; net-snmp will serve only the outer one and \
                         the inner handler and file will never run",
                        device.name,
                        inner.subtree,
                        outer.subtree
                    );
                }
            }
        }
    }

    /// Every file a device serves is served by a registration, and every registration names a
    /// file the device wrote. This was four `grep`s run at deploy time; here it cannot be false,
    /// and the test says so rather than the deploy script discovering it.
    #[test]
    fn every_served_file_has_a_registration_and_vice_versa() {
        for device in lab() {
            for (files, registrations) in [
                (device.data_files(), device.registrations()),
                (device.context_files(), device.context_registrations()),
            ] {
                for registration in &registrations {
                    assert!(
                        registration.file < files.len(),
                        "{}: a registration names no file",
                        device.name
                    );
                }
                for (index, file) in files.iter().enumerate() {
                    assert!(
                        registrations.iter().any(|r| r.file == index),
                        "{}: {} is written but nothing serves it",
                        device.name,
                        file.name
                    );
                }
            }
        }
    }

    /// Every device serving an ifTable registers `ifNumber`. Otherwise the scalar sits in the data
    /// file unserved while `mibII/interfaces` answers from the VM's own kernel state, and the
    /// device reports a count contradiction on every scan for a fault it does not have.
    #[test]
    fn a_device_serving_an_if_table_registers_its_own_count() {
        let if_number = oid_parts(if_mib::IF_NUMBER_OBJECT);
        for device in lab() {
            if device.tables.if_table.is_none() {
                continue;
            }
            assert!(
                device
                    .registrations()
                    .iter()
                    .any(|r| r.subtree == if_number),
                "{} serves an ifTable but does not register ifNumber",
                device.name
            );
        }
    }

    /// Every data file is in strictly ascending OID order, except the one device that is unsorted
    /// on purpose. A mis-sort silently truncates a walk, and this is the property that used to be
    /// a review instruction.
    #[test]
    fn every_file_ascends_except_the_one_that_must_not() {
        for device in lab() {
            for file in device
                .data_files()
                .iter()
                .chain(device.variant_files().iter())
                .chain(device.context_files().iter())
            {
                let rows = file.rows();
                let ascending = rows.windows(2).all(|pair| pair[0].oid < pair[1].oid);
                match file.ordering {
                    Ordering::Ascending => {
                        assert!(ascending, "{} is not in ascending OID order", file.name)
                    }
                    Ordering::Positional => assert!(
                        !ascending,
                        "{} is served positionally but happens to be sorted, so it cannot \
                         reproduce the defect it exists for",
                        file.name
                    ),
                }
            }
        }
    }

    /// No device in the lab loses an LLDP neighbour to a local port that names no interface.
    ///
    /// `count_dropped_neighbours` discards such a neighbour whole: no `lldp_chassis_id` is ever
    /// written, so the far end it names contributes nothing and the server has nothing to resolve.
    /// From the fixture's side this is invisible — the LLDP table still serves the neighbour, and a
    /// device test asserting on `neighbour_named` still passes, because the drop happens after the
    /// walk and before interfaces are built.
    ///
    /// That is how `switch-offsite-01` shipped serving two neighbours nothing could ever read: the
    /// GH #668 "Switch1" shape on port 4 and the address-as-identity subtypes on port 5 were added
    /// to `lldp()` and not to `if_table()`. Both cases existed only in unit tests while appearing
    /// to be covered end to end.
    ///
    /// Lab-wide rather than per device, because the mistake is in adding a fixture and every future
    /// fixture can make it. The two that exercise a *namespace* mismatch
    /// (`switch-exos-01`'s 1001+ ifIndexes, `switch-voss-01`'s identity mapping) resolve their
    /// neighbours through `lldpLocPortTable` and drop none, which is the point of both. The one
    /// device that does drop declares how many through
    /// [`SimDevice::expected_unplaceable_neighbours`] — losing them is the defect it reproduces,
    /// so the check pins the count rather than waiving it.
    #[tokio::test]
    async fn no_lab_device_drops_an_lldp_neighbour() {
        for device in lab() {
            let scan = harness::collect(&device).await;
            if scan.neighbours.records.is_empty() {
                continue;
            }
            assert_eq!(
                scan.dropped_neighbours,
                device.expected_unplaceable_neighbours(),
                "{} serves {} LLDP neighbour(s) and {} reach no interface — a local port in \
                 lldp() with no matching row in if_table() is discarded before the server sees it",
                device.name,
                scan.neighbours.records.len(),
                scan.dropped_neighbours,
            );
        }
    }
}
