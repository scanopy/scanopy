//! The simulated devices, resolved against a real database.
//!
//! These run against Postgres rather than the in-memory inventory on purpose. Every defect below
//! was a *SQL semantics* defect — a lookup with no `ORDER BY` and no `LIMIT` returning an arbitrary
//! row, or a tier that declined when it should have matched — and the fake inventory in
//! `server::lldp`'s tests re-implements those semantics in Rust, so it cannot fail the
//! way the database did.
//!
//! What makes them device tests rather than resolver tests is where the inventory comes from: the
//! far ends are seeded from what a real collection of the *simulated device* produced, so a fixture
//! that stops reporting the identifiers a neighbour is matched on breaks the test that depends on
//! it. Collection-side behaviour is covered without a database in each device's own module.

use std::collections::{HashMap, HashSet};
use std::net::IpAddr;

use mac_address::MacAddress;
use uuid::Uuid;

use crate::daemon::discovery::integration::snmp::convert_snmp_if_entry;
use crate::daemon::discovery::integration::snmp::sim::harness::{self, Collected};
use crate::daemon::discovery::integration::snmp::types::{IfTableEntry, LldpNeighbor};
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::hosts::r#impl::name::{HostName, HostNameSources};
use crate::server::interface_neighbors::service::InterfaceNeighborService;
use crate::server::interfaces::r#impl::base::InterfaceDataComplete;
use crate::server::interfaces::r#impl::wire::DiscoveryInterface;
use crate::server::{
    hosts::r#impl::base::Host,
    interfaces::r#impl::base::{IfAdminStatus, IfOperStatus, Interface, InterfaceBase},
    lldp::{
        AdvertisedIdentity, IdentityResolution, LldpChassisId, LldpPortId,
        resolver::{LldpResolver, LldpResolverImpl},
    },
    shared::storage::traits::Storage,
};

use super::{host, network, organization, subnet, test_services};
use crate::server::hosts::r#impl::attributes::{HostChassisIdValue, HostSysNameValue};
use crate::server::ip_addresses::r#impl::base::{MacEvidence, MacEvidenceValue, mac_of};
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::shared::attribution::{AttributeSource, Attributed};

/// What a credentialed SNMP walk claims for the fields these fixtures stand in for. Named once so
/// a fixture cannot accidentally assert a provenance no real scan produces.
const SNMP_READ: AttributeSource = AttributeSource::Probe(ClientProbe::Snmp);

/// A network holding simulated devices as scanned hosts, and a resolver pointed at the same
/// database.
struct Lab {
    resolver: LldpResolverImpl,
    interfaces: std::sync::Arc<crate::server::interfaces::service::InterfaceService>,
    neighbours: std::sync::Arc<InterfaceNeighborService>,
    storage: crate::server::shared::storage::factory::StorageFactory,
    network_id: Uuid,
    _subnet_id: Uuid,
    _container: testcontainers::ContainerAsync<testcontainers::GenericImage>,
}

impl Lab {
    async fn new() -> Self {
        let (storage, services, _container) = test_services().await;

        let org = organization();
        storage.organizations.create(&org).await.unwrap();
        let network = network(&org.id);
        storage.networks.create(&network).await.unwrap();
        let subnet = subnet(&network.id);
        storage.subnets.create(&subnet).await.unwrap();

        let resolver = LldpResolverImpl::new(
            services.interface_service.clone(),
            services.ip_address_service.clone(),
            storage.hosts.clone(),
        );

        Self {
            resolver,
            interfaces: services.interface_service.clone(),
            neighbours: services.interface_neighbor_service.clone(),
            network_id: network.id,
            _subnet_id: subnet.id,
            storage,
            _container,
        }
    }

    /// Scan a simulated device and persist what the collection read.
    ///
    /// The host's `chassis_id` and `sys_name` come from the device's own LLDP local identity, and
    /// its interfaces from its ifTable — the same two sources a real scan writes, which is what
    /// makes a neighbour pointing here resolve through the tier it is supposed to.
    async fn scan(&self, name: &str) -> Scanned {
        let device = crate::daemon::discovery::integration::snmp::sim::device(name);
        let collected = harness::collect(&device).await;

        // Most devices in this lab advertise LLDP; a stock Windows SNMP service does not answer
        // `lldpLocalSystemData` about itself (GH #668's `pc-windows-nic-filters`), which must still
        // be a scannable host — its resolution as a neighbour rests entirely on its interfaces'
        // MACs, not on a self-reported chassis id.
        let chassis_id = device.tables.lldp.as_ref().and_then(|table| {
            let (subtype, value) = table.chassis.id.to_snmp(table.chassis.encoding);
            LldpChassisId::from_snmp(subtype, &value).map(|id| id.identifier())
        });

        let mut record = host(&self.network_id);
        record.base.name = HostName::manual(name.to_string());
        record.base.chassis_id =
            chassis_id.map(|v| Attributed::new(HostChassisIdValue(v), SNMP_READ));
        record.base.sys_name = device
            .system
            .sys_name
            .clone()
            .map(|v| Attributed::new(HostSysNameValue(v), SNMP_READ));
        self.storage.hosts.create(&record).await.unwrap();

        // Which ifIndexes the device's own ipAddrTable binds an address to — see
        // `InterfaceBase::ip_configured`. Computed once per scan, exactly as `convert_snmp_if_
        // entry` does daemon-side, so a seeded interface's tie-break signal matches what a real
        // scan of this device would have set.
        let ip_configured_if_indexes: std::collections::HashSet<i32> = collected
            .ip_addr_table
            .values()
            .map(|e| e.if_index)
            .collect();
        for entry in &collected.if_table.entries {
            self.interface(
                record.id,
                entry,
                ip_configured_if_indexes.contains(&entry.if_index),
            )
            .await;
        }

        Scanned {
            host: record,
            collected,
        }
    }

    /// A host that answered SNMP and served nothing else: no interfaces, no chassis id, no
    /// `sysName` — the shape `switch-mute-01` has, and the shape every tier above the address is
    /// structurally unable to place. It holds exactly one thing: an address.
    async fn host_with_only_an_address(&self, name: &str, address: IpAddr) -> Host {
        self.mute_host(name, address, None).await
    }

    /// Seed a device that answers SNMP and serves nothing else, keeping whatever `sysName` it
    /// reports. `Lab::scan` cannot be used for one: it reads the device's LLDP local identity, and
    /// serving none is the whole point.
    async fn mute_host(&self, name: &str, address: IpAddr, sys_name: Option<&str>) -> Host {
        let mut record = host(&self.network_id);
        record.base.name = HostName::manual(name.to_string());
        record.base.chassis_id = None;
        record.base.sys_name =
            sys_name.map(|v| Attributed::new(HostSysNameValue(v.into()), SNMP_READ));
        self.storage.hosts.create(&record).await.unwrap();

        let mut ip = super::ip_address(&self.network_id, &self._subnet_id);
        ip.base.host_id = record.id;
        ip.base.ip_address = address;
        self.storage.ip_addresses.create(&ip).await.unwrap();

        record
    }

    async fn interface(
        &self,
        host_id: Uuid,
        entry: &IfTableEntry,
        ip_configured: bool,
    ) -> Interface {
        let interface = Interface::new(InterfaceBase {
            host_id,
            network_id: self.network_id,
            if_index: Some(entry.if_index),
            if_descr: entry.if_descr.clone(),
            if_name: entry.if_name.clone(),
            if_alias: entry.if_alias.clone(),
            if_type: Some(entry.if_type.unwrap_or_default()),
            mac_address: entry
                .if_phys_address
                .map(|m| MacEvidence::new(MacEvidenceValue(m), SNMP_READ)),
            admin_status: Some(IfAdminStatus::Up),
            oper_status: Some(IfOperStatus::Up),
            ip_configured,
            ..Default::default()
        });
        self.storage.interfaces.create(&interface).await.unwrap();
        interface
    }
}

struct Scanned {
    host: Host,
    collected: Collected,
}

impl Scanned {
    /// The chassis id a neighbour of this device advertises, as the far end reads it.
    fn advertised_chassis(neighbour: &LldpNeighbor) -> LldpChassisId {
        LldpChassisId::from_snmp(
            neighbour.remote_chassis_id_subtype.expect("a subtype"),
            neighbour.remote_chassis_id_bytes.as_ref().expect("a value"),
        )
        .expect("a chassis id")
    }

    fn advertised_port(neighbour: &LldpNeighbor) -> LldpPortId {
        LldpPortId::from_snmp(
            neighbour.remote_port_id_subtype.expect("a subtype"),
            neighbour.remote_port_id_bytes.as_ref().expect("a value"),
        )
        .expect("a port id")
    }
}

/// GH #685: a neighbour walk that stopped part way keeps every row it read.
///
/// The reporter's switch lost its whole neighbour set to a walk that did not finish, and the
/// warning said so. Driven end to end: the daemon's collection of a device whose walk stops, the
/// interfaces it would submit, the JSON they cross the wire as, and the server's ingest with LLDP
/// marked not authoritative — each step one that dropped the rows at some point.
#[tokio::test]
async fn a_neighbour_walk_that_stopped_part_way_keeps_every_row_it_read() {
    let lab = Lab::new().await;
    let device = crate::daemon::discovery::integration::snmp::sim::device("switch-quietcol-01");
    let collected = harness::collect(&device).await;
    assert!(
        !collected.neighbours.complete && !collected.neighbours.records.is_empty(),
        "the fixture has to stop part way having read something, or this proves nothing"
    );

    let mut record = host(&lab.network_id);
    record.base.name = HostName::manual(device.name.to_string());
    lab.storage.hosts.create(&record).await.unwrap();

    // What `execute` submits: neighbours on the interfaces they sit on, and LLDP marked
    // authoritative only when the walk finished.
    let collected_groups = InterfaceDataComplete {
        lldp: collected.neighbours.complete && !collected.neighbours.unsupported,
        ..Default::default()
    };
    assert!(!collected_groups.lldp);

    let mut claimed = HashSet::new();
    let mut persisted = HashMap::new();
    for entry in &collected.if_table.entries {
        let submitted = convert_snmp_if_entry(
            entry,
            lab.network_id,
            &collected.neighbours.records,
            &collected.cdp.records,
            &[],
            &[],
            &HashMap::new(),
            &HashSet::new(),
        );
        let wire = serde_json::to_value(&submitted).unwrap();
        let mut received: Interface = serde_json::from_value::<DiscoveryInterface>(wire)
            .unwrap()
            .into();
        received.base.host_id = record.id;

        let stored = lab
            .interfaces
            .create_or_update_from_discovery(
                received,
                &claimed,
                collected_groups,
                AuthenticatedEntity::System,
            )
            .await
            .unwrap();
        claimed.insert(stored.id);
        persisted.insert(entry.if_index, stored.id);
    }

    for neighbour in &collected.neighbours.records {
        let interface_id = persisted[&neighbour.local_port_index];
        let chassis: Vec<LldpChassisId> = lab
            .neighbours
            .candidates_for_interface(&interface_id)
            .await
            .unwrap()
            .into_iter()
            .filter_map(|c| c.base.evidence.lldp_chassis_id)
            .collect();
        assert_eq!(
            chassis,
            vec![Scanned::advertised_chassis(neighbour)],
            "the neighbour read on local port {} was not recorded",
            neighbour.local_port_index
        );
    }
}

/// GH #664: a chassis MAC that is on no port and no IP.
///
/// `switch-netgear-01`'s LLDP chassis id is `00:1a:2b:3c:4d:63` while its ports report `…:65/:66/
/// :67`, and it bears no address with that MAC. `switch-aruba-01`'s neighbour entries advertise
/// that chassis id, so the remote host is identifiable **only** through `hosts.chassis_id`,
/// recorded from switch-netgear-01's own LLDP local identity.
///
/// Matching against interfaces and IPs alone yields `hosts_resolved=0` and an empty L2 Physical
/// view. This runs against the database because that is where it failed.
#[tokio::test]
async fn a_chassis_id_on_no_port_still_finds_its_device() {
    let lab = Lab::new().await;
    let netgear = lab.scan("switch-netgear-01").await;
    let aruba = lab.scan("switch-aruba-01").await;

    // Cabled twice — `g1 ↔ port 41` and `g2 ↔ A5` — so both of its neighbours name the same far
    // end, and either will do for a chassis lookup.
    let neighbours = aruba.collected.neighbours_named("switch-netgear-01");
    assert_eq!(neighbours.len(), 2, "the pair is cabled twice on purpose");
    let advertised = Scanned::advertised_chassis(neighbours[0]);

    // The precondition: nothing but `hosts.chassis_id` can answer this.
    assert_eq!(
        lab.resolver
            .find_host_by_mac(&advertised.identifier(), lab.network_id)
            .await,
        IdentityResolution::NotFound,
        "the chassis MAC must be on no interface, or this test proves the wrong tier"
    );

    assert_eq!(
        advertised
            .resolve_host_id(&lab.resolver, lab.network_id, AdvertisedIdentity::default())
            .await,
        IdentityResolution::Resolved(netgear.host.id),
        "the far end is findable only through the chassis id it recorded about itself"
    );
}

/// GH #668: the far end that answered SNMP and served nothing else.
///
/// Such a device carries no interface row to hold the chassis MAC its neighbours advertise and no
/// `chassis_id` of its own, so the MAC tier and the chassis tier are structurally dead for it — and
/// with no `sysName` recorded either, the ladder had nothing left. The address it publishes in
/// `lldpRemManAddr` is the one identifier that survives, and this network already holds it.
///
/// Against the database because that is where it has to hold: the tier resolves through
/// `ip_addresses`, and the fake-inventory tests re-implement that lookup rather than running it.
#[tokio::test]
async fn a_far_end_with_no_tables_is_found_by_the_address_it_publishes() {
    let lab = Lab::new().await;
    let management_address: IpAddr = "192.168.1.248".parse().unwrap();
    let mute = lab
        .host_with_only_an_address("switch-mute-01", management_address)
        .await;

    // What a D-Link neighbour of it advertises: a chassis MAC that exists nowhere on our side.
    let advertised = LldpChassisId::MacAddress("00:ad:24:89:cc:f0".to_string());

    // The preconditions. Without these the test could pass through a tier it is not about.
    assert_eq!(
        lab.resolver
            .find_host_by_mac(&advertised.identifier(), lab.network_id)
            .await,
        IdentityResolution::NotFound,
        "the chassis MAC must be on no interface and no address"
    );
    assert_eq!(
        lab.resolver
            .find_host_by_chassis_id(&advertised.identifier(), lab.network_id)
            .await,
        IdentityResolution::NotFound,
        "and on no host's own chassis id"
    );

    assert_eq!(
        advertised
            .resolve_host_id(
                &lab.resolver,
                lab.network_id,
                AdvertisedIdentity {
                    sys_name: None,
                    address: Some(management_address),
                },
            )
            .await,
        IdentityResolution::Resolved(mute.id),
        "the address it published is the only tier that can place it"
    );
}

/// GH #668 end to end, against the devices themselves.
///
/// `switch-offsite-01` names `switch-mute-01` on Gi0/4 as `Switch1`, which is not the sysName that
/// device answers to. `switch-mute-01` serves no ifTable and no LLDP local identity, so it holds
/// neither the chassis MAC its neighbour advertises nor a `chassis_id` of its own — and the sysName
/// tier misses because the two names disagree. It is in the host list the whole time. The address
/// it publishes is the only tier that can place it, which is the report's actual complaint.
#[tokio::test]
async fn the_mute_far_end_is_placed_only_by_the_address_its_neighbour_publishes() {
    let lab = Lab::new().await;
    let mute_device = crate::daemon::discovery::integration::snmp::sim::device("switch-mute-01");
    let mute = lab
        .mute_host(
            "switch-mute-01",
            IpAddr::V4(mute_device.ip),
            mute_device.system.sys_name.as_deref(),
        )
        .await;
    let offsite = lab.scan("switch-offsite-01").await;

    let neighbour = offsite
        .collected
        .neighbours_named("Switch1")
        .into_iter()
        .next()
        .expect("the neighbour on Gi0/4");
    let advertised = Scanned::advertised_chassis(neighbour);

    // Every tier above the address, shown failing rather than assumed to.
    assert_eq!(
        lab.resolver
            .find_host_by_mac(&advertised.identifier(), lab.network_id)
            .await,
        IdentityResolution::NotFound,
        "the chassis MAC is on no interface, because the device serves no ifTable"
    );
    assert_eq!(
        lab.resolver
            .find_host_by_chassis_id(&advertised.identifier(), lab.network_id)
            .await,
        IdentityResolution::NotFound,
        "and on no host's own chassis id, because it publishes no LLDP local identity"
    );
    assert_eq!(
        lab.resolver
            .find_host_by_sys_name("Switch1", lab.network_id)
            .await,
        IdentityResolution::NotFound,
        "and the name it is advertised under is not the one it answers to"
    );

    assert_eq!(
        advertised
            .resolve_host_id(
                &lab.resolver,
                lab.network_id,
                AdvertisedIdentity {
                    sys_name: neighbour.remote_sys_name.as_deref(),
                    address: neighbour.remote_mgmt_addr,
                },
            )
            .await,
        IdentityResolution::Resolved(mute.id),
        "a device sitting in the host list, placeable only by the address it publishes"
    );
}

/// An address nothing holds stays `NotFound` — the tier must not invent a match, and this is the
/// population the subnet inference then runs on.
#[tokio::test]
async fn a_published_address_this_network_does_not_hold_resolves_to_nothing() {
    let lab = Lab::new().await;
    lab.host_with_only_an_address("switch-mute-01", "192.168.1.248".parse().unwrap())
        .await;

    let advertised = LldpChassisId::MacAddress("00:ad:24:89:cc:f0".to_string());
    assert_eq!(
        advertised
            .resolve_host_id(
                &lab.resolver,
                lab.network_id,
                AdvertisedIdentity {
                    sys_name: None,
                    address: Some("10.20.30.11".parse().unwrap()),
                },
            )
            .await,
        IdentityResolution::NotFound
    );
}

/// GH #649: locally-assigned port ids (subtype 7), reaching a port two different ways.
///
/// `switch-netgear-01` advertises `41` — which is `switch-aruba-01`'s `ifDescr` — and `197`, which
/// matches only its `ifIndex` because that port is labelled `A5`. Treating subtype 7 as
/// unresolvable stops at the host, and a host-only neighbour draws no edge at all.
#[tokio::test]
async fn locally_assigned_port_ids_reach_a_port_by_name_and_by_index() {
    let lab = Lab::new().await;
    let aruba = lab.scan("switch-aruba-01").await;
    let netgear = lab.scan("switch-netgear-01").await;

    let ports: Vec<LldpPortId> = netgear
        .collected
        .neighbours
        .records
        .iter()
        .map(Scanned::advertised_port)
        .collect();

    let by_descr = ports
        .iter()
        .find(|id| matches!(id, LldpPortId::LocallyAssigned(v) if v == "41"))
        .expect("the ifDescr-shaped id");
    let by_index = ports
        .iter()
        .find(|id| matches!(id, LldpPortId::LocallyAssigned(v) if v == "197"))
        .expect("the ifIndex-shaped id");

    assert!(
        matches!(
            by_descr
                .resolve_if_entry_id(&lab.resolver, aruba.host.id)
                .await,
            IdentityResolution::Resolved(_)
        ),
        "`41` is that switch's ifDescr and must reach the port"
    );
    assert!(
        matches!(
            by_index
                .resolve_if_entry_id(&lab.resolver, aruba.host.id)
                .await,
            IdentityResolution::Resolved(_)
        ),
        "`197` matches no name on that switch, so it must fall through to ifIndex"
    );
}

/// GH #668: a MAC port id that belongs to three ports identifies none of them.
///
/// `switch-tplink-01`'s `1/0/4` advertises `lldpRemPortIdSubtype` = 3 with the address
/// `switch-dlink-01` reports on all of its ports. The chassis id resolves the host; the port id
/// must then resolve *nothing*, because a MAC belonging to three ports identifies no port.
///
/// Expect an ambiguous verdict rather than whichever row the database returned first — which is
/// what it drew before #668, and what a lookup with no `ORDER BY` and no `LIMIT` will do again.
#[tokio::test]
async fn a_mac_on_every_port_of_the_far_end_resolves_to_no_port() {
    let lab = Lab::new().await;
    let dlink = lab.scan("switch-dlink-01").await;
    let tplink = lab.scan("switch-tplink-01").await;

    // Local port 4 is the shared-MAC case; it names switch-dlink-01 like local port 3 does.
    let ambiguous = tplink
        .collected
        .neighbours_on(4)
        .into_iter()
        .next()
        .expect("a neighbour on 1/0/4");
    let port_id = Scanned::advertised_port(ambiguous);
    assert!(matches!(port_id, LldpPortId::MacAddress(_)));

    // The host resolves; only the port must not.
    assert_eq!(
        Scanned::advertised_chassis(ambiguous)
            .resolve_host_id(&lab.resolver, lab.network_id, AdvertisedIdentity::default())
            .await,
        IdentityResolution::Resolved(dlink.host.id)
    );
    assert_eq!(
        port_id
            .resolve_if_entry_id(&lab.resolver, dlink.host.id)
            .await,
        IdentityResolution::Ambiguous,
        "an address on three ports must be declined, not resolved to whichever row came back first"
    );
}

/// The positive half of the same pair, and the reason the guard above cannot simply reject every
/// MAC port id.
///
/// `switch-dlink-01`'s local port 3 advertises subtype 3 with `00:07:7c:20:01:e3`, which is
/// `switch-macport-01`'s `ifPhysAddress` for `eth3` and for nothing else — so it must resolve to
/// that port. A guard that rejected this too would look correct while quietly costing every vendor
/// that addresses its ports individually.
#[tokio::test]
async fn a_mac_that_identifies_exactly_one_port_still_resolves() {
    let lab = Lab::new().await;
    let macport = lab.scan("switch-macport-01").await;
    let dlink = lab.scan("switch-dlink-01").await;

    let neighbour = dlink
        .collected
        .neighbours_on(3)
        .into_iter()
        .next()
        .expect("a neighbour on local port 3");
    let port_id = Scanned::advertised_port(neighbour);

    let resolved = port_id
        .resolve_if_entry_id(&lab.resolver, macport.host.id)
        .await;
    let IdentityResolution::Resolved(interface_id) = resolved else {
        panic!("a unique per-port address must resolve, got {resolved:?}");
    };

    let interface = lab
        .storage
        .interfaces
        .get_by_id(&interface_id)
        .await
        .unwrap()
        .expect("the interface exists");
    assert_eq!(
        mac_of(&interface.base.mac_address),
        Some("00:07:7c:20:01:e3".parse::<MacAddress>().unwrap()),
        "it must land on the port that actually carries that address"
    );
}

/// GH #668's last reported symptom, and the acceptance case for `ip_configured`.
///
/// `pc-windows-nic-filters` exposes one MAC on a real NIC (ifIndex 7) and on three NDIS filter/LWF
/// pseudo-interfaces layered on top of it (18-20) — all four report an ordinary ethernet
/// `if_type`, so `physical_if_types()`'s exclusion does not separate them and, before the fix,
/// this is exactly `a_mac_on_every_port_of_the_far_end_resolves_to_no_port`'s shape:
/// `switch-dlink-02`'s neighbour on port 7 resolves the host but leaves the port `Ambiguous`. The
/// one thing that does separate the four candidates, confirmed against the reporting customer's
/// own debug log: `ipAddrTable` binds the host's address to the real NIC's ifIndex only.
#[tokio::test]
async fn the_ip_configured_nic_resolves_among_its_own_filter_pseudo_interfaces() {
    let lab = Lab::new().await;
    let pc = lab.scan("pc-windows-nic-filters").await;
    let switch = lab.scan("switch-dlink-02").await;

    let neighbour = switch
        .collected
        .neighbours_on(7)
        .into_iter()
        .next()
        .expect("a neighbour on local port 7");
    let port_id = Scanned::advertised_port(neighbour);
    assert!(matches!(port_id, LldpPortId::MacAddress(_)));

    // The host resolves via the MAC on its interfaces — this device never advertises its own
    // chassis id, matching the real PC-069's `has_lldp_local=false`.
    assert_eq!(
        Scanned::advertised_chassis(neighbour)
            .resolve_host_id(&lab.resolver, lab.network_id, AdvertisedIdentity::default())
            .await,
        IdentityResolution::Resolved(pc.host.id)
    );

    let resolved = port_id.resolve_if_entry_id(&lab.resolver, pc.host.id).await;
    let IdentityResolution::Resolved(interface_id) = resolved else {
        panic!("the ipAddrTable-bound NIC must resolve the tie, got {resolved:?}");
    };

    let interface = lab
        .storage
        .interfaces
        .get_by_id(&interface_id)
        .await
        .unwrap()
        .expect("the interface exists");
    assert_eq!(
        interface.base.if_index,
        Some(7),
        "it must land on the real NIC, not one of its filter pseudo-interfaces"
    );
    assert!(
        interface.base.ip_configured,
        "the resolved interface must be the one ipAddrTable actually bound"
    );
}

/// GH #664's other half, and the one that decides whether the fixture is worth anything.
///
/// `switch-netgear-01`'s chassis id is on none of its *physical* ports — but its own scanned rows
/// must still carry addresses, or the MAC tier is untested rather than declining. This asserts the
/// tier runs and finds nothing, which is a different outcome from the tier never running.
#[tokio::test]
async fn the_mac_tier_runs_and_declines_rather_than_being_skipped() {
    let lab = Lab::new().await;
    let netgear = lab.scan("switch-netgear-01").await;

    let port_mac = netgear
        .collected
        .if_table
        .entries
        .iter()
        .find_map(|e| e.if_phys_address)
        .expect("its ports carry addresses");

    // A port address does find the device...
    assert_eq!(
        lab.resolver
            .find_host_by_mac(&port_mac.to_string().to_lowercase(), lab.network_id)
            .await,
        IdentityResolution::Resolved(netgear.host.id)
    );
    // ...while the chassis address, which is on no port, does not.
    assert_eq!(
        lab.resolver
            .find_host_by_mac("00:1a:2b:3c:4d:63", lab.network_id)
            .await,
        IdentityResolution::NotFound
    );
}

/// An endpoint no device in the lab knows.
///
/// `switch-tplink-01`'s `1/0/2` advertises a desk phone whose MAC and sysName belong to nothing
/// here, so every host tier fails. It is the environment's only source of a non-zero
/// `host_not_found`, and that counter is otherwise permanently 0 — which left the server-side
/// summary that names unmatched far ends with no way to fire.
#[tokio::test]
async fn a_far_end_nobody_scanned_resolves_to_nothing() {
    let lab = Lab::new().await;
    let tplink = lab.scan("switch-tplink-01").await;
    lab.scan("switch-dlink-01").await;

    let stranger = tplink
        .collected
        .neighbours_on(2)
        .into_iter()
        .next()
        .expect("a neighbour on 1/0/2");

    let resolved = Scanned::advertised_chassis(stranger)
        .resolve_host_id(
            &lab.resolver,
            lab.network_id,
            AdvertisedIdentity {
                sys_name: stranger.remote_sys_name.as_deref(),
                address: stranger.remote_mgmt_addr,
            },
        )
        .await;
    assert_eq!(
        resolved,
        IdentityResolution::NotFound,
        "an endpoint nobody scanned is not found, which is not the same as unresolvable"
    );
}

/// GH #685: the neighbours `switch-quietcol-01` reads before its walk stops reach the ports they
/// name, so the partial read it exists to exercise draws port-level links in the lab.
///
/// Its first fixture borrowed identifiers from another device: one chassis id belonged to
/// `router-gw-01` and two far-end ports did not exist, so a scan drew host-level links, one of them
/// to the wrong device, and nothing failed.
#[tokio::test]
async fn the_quietcol_neighbours_reach_the_ports_they_name() {
    let lab = Lab::new().await;
    let quietcol = lab.scan("switch-quietcol-01").await;
    let mut far_ends = std::collections::HashMap::new();
    for name in ["switch-voss-01", "switch-dell-01", "switch-exos-01"] {
        far_ends.insert(name, lab.scan(name).await.host.id);
    }

    assert_eq!(quietcol.collected.neighbours.records.len(), 3);
    for neighbour in &quietcol.collected.neighbours.records {
        let name = neighbour.remote_sys_name.as_deref().expect("a sysName");
        let expected = *far_ends.get(name).unwrap_or_else(|| {
            panic!("{name} is not one of the far ends this device is cabled to")
        });

        let host = Scanned::advertised_chassis(neighbour)
            .resolve_host_id(&lab.resolver, lab.network_id, AdvertisedIdentity::default())
            .await;
        assert_eq!(
            host,
            IdentityResolution::Resolved(expected),
            "the chassis id advertised for {name} must identify {name}"
        );

        assert!(
            matches!(
                Scanned::advertised_port(neighbour)
                    .resolve_if_entry_id(&lab.resolver, expected)
                    .await,
                IdentityResolution::Resolved(_)
            ),
            "the port advertised for {name} must be a port {name} has"
        );
    }
}

/// A sanity check on the seeding itself: the far ends really are in the database with the
/// identifiers the neighbours name them by.
///
/// Without this, every test above could pass against an empty inventory for the wrong reason.
#[tokio::test]
async fn the_seeded_far_ends_carry_the_identifiers_they_are_named_by() {
    let lab = Lab::new().await;
    let core = lab.scan("switch-core-01").await;

    assert_eq!(
        lab.resolver
            .find_host_by_chassis_id("00:1a:2b:00:10:00", lab.network_id)
            .await,
        IdentityResolution::Resolved(core.host.id)
    );
    let _: IpAddr = "192.168.7.230".parse().unwrap();
    assert_eq!(core.collected.if_table.entries.len(), 4);
}
