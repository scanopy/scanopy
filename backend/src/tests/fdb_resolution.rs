//! `HostService::resolve_fdb_links` against a real database.
//!
//! Written before the function had a caller anywhere in the codebase (GH #709) — these pin its
//! current behaviour so wiring it into session completion cannot silently change what it does.
//! Real Postgres, not an in-memory fake, for the same reason `lldp_resolution.rs` uses it: the
//! function's own gating (single MAC, single host, single port) leans on `StorableFilter`
//! conditions (`jsonb_array_length`, `NOT EXISTS` subqueries) that only a real query plan proves.

use chrono::Utc;
use uuid::Uuid;

use crate::daemon::discovery::integration::snmp::sim::harness;
use crate::server::{
    hosts::{
        r#impl::{base::Host, name::HostNameSources},
        service::HostService,
    },
    interface_neighbors::{
        r#impl::base::{InterfaceNeighborEvidence, Neighbor},
        service::InterfaceNeighborService,
    },
    interfaces::r#impl::base::{
        IfAdminStatus, IfOperStatus, Interface, InterfaceBase, InterfaceDataComplete, if_type,
    },
    ip_addresses::r#impl::base::{MacEvidence, MacEvidenceValue},
    lldp::LldpChassisId,
    shared::{
        attribution::AttributeSource,
        storage::{factory::StorageFactory, filter::StorableFilter, traits::Storage},
    },
};

use super::{host, network, organization, subnet, test_services};

/// Everything an FDB resolution test needs: a network with hosts and interfaces in it, and the
/// two services `resolve_fdb_links` reads and writes through.
struct Lab {
    host_service: std::sync::Arc<HostService>,
    interface_neighbor_service: std::sync::Arc<InterfaceNeighborService>,
    storage: StorageFactory,
    network_id: Uuid,
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

        Self {
            host_service: services.host_service.clone(),
            interface_neighbor_service: services.interface_neighbor_service.clone(),
            network_id: network.id,
            storage,
            _container,
        }
    }

    async fn host(&self, name: &str) -> Host {
        let mut h = host(&self.network_id);
        h.base.name = crate::server::hosts::r#impl::name::HostName::manual(name.to_string());
        self.storage.hosts.create(&h).await.unwrap();
        h
    }

    /// A physical ethernet port, optionally carrying a MAC — the far end a single-MAC FDB port
    /// resolves against.
    async fn port(
        &self,
        host_id: Uuid,
        if_index: i32,
        descr: &str,
        mac: Option<&str>,
    ) -> Interface {
        let entry = Interface::new(InterfaceBase {
            host_id,
            network_id: self.network_id,
            if_index: Some(if_index),
            if_descr: Some(descr.to_string()),
            if_type: Some(if_type::ETHERNET_CSMA_CD),
            mac_address: mac.map(|m| {
                MacEvidence::new(
                    MacEvidenceValue(m.parse().unwrap()),
                    AttributeSource::ArpReply,
                )
            }),
            admin_status: Some(IfAdminStatus::Up),
            oper_status: Some(IfOperStatus::Up),
            ..Default::default()
        });
        self.storage.interfaces.create(&entry).await.unwrap();
        entry
    }

    /// A switch port whose bridge FDB learned exactly one MAC — the shape
    /// `new_for_unresolved_fdb_in_network` selects, before any candidate/resolved row exists.
    async fn fdb_port(
        &self,
        host_id: Uuid,
        if_index: i32,
        descr: &str,
        learned_mac: &str,
    ) -> Interface {
        let entry = Interface::new(InterfaceBase {
            host_id,
            network_id: self.network_id,
            if_index: Some(if_index),
            if_descr: Some(descr.to_string()),
            if_type: Some(if_type::ETHERNET_CSMA_CD),
            fdb_macs: Some(vec![learned_mac.to_string()]),
            admin_status: Some(IfAdminStatus::Up),
            oper_status: Some(IfOperStatus::Up),
            ..Default::default()
        });
        self.storage.interfaces.create(&entry).await.unwrap();
        entry
    }

    /// A raw LLDP candidate on an interface — evidence the SQL filter treats as "LLDP/CDP already
    /// owns this row", regardless of what `fdb_macs` also holds.
    async fn give_lldp_candidate(&self, interface_id: Uuid) {
        self.interface_neighbor_service
            .replace_candidates_from_discovery(
                self.network_id,
                interface_id,
                vec![InterfaceNeighborEvidence {
                    lldp_chassis_id: Some(LldpChassisId::MacAddress("00:aa:bb:cc:dd:ee".into())),
                    ..Default::default()
                }],
                InterfaceDataComplete {
                    lldp: true,
                    cdp: true,
                    fdb: true,
                    vlan_membership: true,
                },
            )
            .await
            .unwrap();
    }

    /// Scan a simulated device and persist what the collection read, `fdb_macs` included — the
    /// same learned(3)-only filter `convert_snmp_if_entry` applies in production
    /// (`daemon/discovery/integration/snmp/mod.rs`). Unlike `snmp_sim_resolution::Lab::scan`, this
    /// does not require the device to advertise LLDP: a device with no LLDP table at all (this
    /// file's whole reason to exist) has nothing for that reader to read.
    async fn scan(&self, name: &str) -> Host {
        let collected = harness::scan(name).await;

        let mut record = host(&self.network_id);
        record.base.name = crate::server::hosts::r#impl::name::HostName::manual(name.to_string());
        self.storage.hosts.create(&record).await.unwrap();

        for entry in &collected.if_table.entries {
            let learned_macs: Vec<String> = collected
                .fdb
                .records
                .iter()
                .filter(|fdb| fdb.if_index == Some(entry.if_index) && fdb.status == 3)
                .map(|fdb| fdb.mac_address.to_string())
                .collect();

            let interface = Interface::new(InterfaceBase {
                host_id: record.id,
                network_id: self.network_id,
                if_index: Some(entry.if_index),
                if_descr: entry.if_descr.clone(),
                if_name: entry.if_name.clone(),
                if_alias: entry.if_alias.clone(),
                if_type: Some(entry.if_type.unwrap_or(if_type::ETHERNET_CSMA_CD)),
                mac_address: entry
                    .if_phys_address
                    .map(|m| MacEvidence::new(MacEvidenceValue(m), AttributeSource::ArpReply)),
                fdb_macs: (!learned_macs.is_empty()).then_some(learned_macs),
                admin_status: Some(IfAdminStatus::Up),
                oper_status: Some(IfOperStatus::Up),
                ..Default::default()
            });
            self.storage.interfaces.create(&interface).await.unwrap();
        }

        record
    }

    async fn unresolved_fdb_count(&self) -> usize {
        self.storage
            .interfaces
            .get_all(
                StorableFilter::<Interface>::new_for_unresolved_fdb_in_network(self.network_id),
            )
            .await
            .unwrap()
            .len()
    }

    async fn resolve(&self) -> anyhow::Result<u32> {
        self.host_service
            .resolve_fdb_links(self.network_id, Utc::now())
            .await
    }

    async fn resolved_neighbor(&self, interface_id: Uuid) -> Option<Neighbor> {
        self.interface_neighbor_service
            .resolved_for_interface(&interface_id)
            .await
            .unwrap()
            .into_iter()
            .map(|row| row.neighbor)
            .next()
    }
}

#[tokio::test]
async fn a_single_mac_resolves_to_the_specific_port() {
    let lab = Lab::new().await;
    let switch = lab.host("switch").await;
    let far_end = lab.host("far-end").await;
    let far_port = lab
        .port(far_end.id, 1, "Gi0/1", Some("00:1a:2b:00:10:01"))
        .await;

    let anchor = lab
        .fdb_port(switch.id, 5, "Gi0/5", "00:1a:2b:00:10:01")
        .await;

    assert_eq!(lab.resolve().await.unwrap(), 1);
    assert_eq!(
        lab.resolved_neighbor(anchor.id).await,
        Some(Neighbor::Interface(far_port.id))
    );
}

#[tokio::test]
async fn a_single_mac_with_no_matching_interface_falls_back_to_the_host() {
    let lab = Lab::new().await;
    let switch = lab.host("switch").await;
    // The far end resolves by a MAC on an interface with no if_type set to a physical type — an
    // end device Scanopy never polled directly, the shape an unmanaged downstream device or a
    // bare host takes. `find_if_entry_by_mac` only searches physical if_types, so this MAC
    // resolves the host but no specific port on it.
    let far_end = lab.host("laptop").await;
    let entry = Interface::new(InterfaceBase {
        host_id: far_end.id,
        network_id: lab.network_id,
        if_index: Some(1),
        if_descr: Some("eth0".to_string()),
        if_type: Some(if_type::PROP_VIRTUAL),
        mac_address: Some(MacEvidence::new(
            MacEvidenceValue("00:1a:2b:00:10:02".parse().unwrap()),
            AttributeSource::ArpReply,
        )),
        ..Default::default()
    });
    lab.storage.interfaces.create(&entry).await.unwrap();

    let anchor = lab
        .fdb_port(switch.id, 5, "Gi0/5", "00:1a:2b:00:10:02")
        .await;

    assert_eq!(lab.resolve().await.unwrap(), 1);
    assert_eq!(
        lab.resolved_neighbor(anchor.id).await,
        Some(Neighbor::Host(far_end.id))
    );
}

#[tokio::test]
async fn a_mac_on_two_ports_of_the_same_host_falls_back_to_the_host() {
    let lab = Lab::new().await;
    let switch = lab.host("switch").await;
    let far_end = lab.host("d-link").await;
    for if_index in 1..=2 {
        lab.port(
            far_end.id,
            if_index,
            &format!("Slot0/{if_index}"),
            Some("00:ad:24:af:4e:00"),
        )
        .await;
    }

    let anchor = lab
        .fdb_port(switch.id, 5, "Gi0/5", "00:ad:24:af:4e:00")
        .await;

    assert_eq!(lab.resolve().await.unwrap(), 1);
    assert_eq!(
        lab.resolved_neighbor(anchor.id).await,
        Some(Neighbor::Host(far_end.id)),
        "a MAC shared by two ports of one host identifies the host, not an arbitrary port"
    );
}

#[tokio::test]
async fn a_mac_on_two_different_hosts_resolves_to_nothing() {
    let lab = Lab::new().await;
    let switch = lab.host("switch").await;
    for name in ["switch-a", "switch-b"] {
        let far_end = lab.host(name).await;
        lab.port(far_end.id, 1, "Gi0/1", Some("00:ad:24:af:4e:01"))
            .await;
    }

    let anchor = lab
        .fdb_port(switch.id, 5, "Gi0/5", "00:ad:24:af:4e:01")
        .await;

    assert_eq!(
        lab.resolve().await.unwrap(),
        0,
        "a MAC on two devices names neither"
    );
    assert_eq!(lab.resolved_neighbor(anchor.id).await, None);
}

/// Ordering is load-bearing: the SQL filter — not a Rust-side check inside `resolve_fdb_links` —
/// is what keeps the FDB pass off rows LLDP/CDP evidence already claims, so LLDP running first
/// leaves nothing here for FDB to act on.
#[tokio::test]
async fn an_interface_with_any_lldp_candidate_is_left_to_lldp() {
    let lab = Lab::new().await;
    let switch = lab.host("switch").await;
    let far_end = lab.host("far-end").await;
    lab.port(far_end.id, 1, "Gi0/1", Some("00:1a:2b:00:10:03"))
        .await;

    let anchor = lab
        .fdb_port(switch.id, 5, "Gi0/5", "00:1a:2b:00:10:03")
        .await;
    lab.give_lldp_candidate(anchor.id).await;

    assert_eq!(
        lab.unresolved_fdb_count().await,
        0,
        "a port with any raw LLDP/CDP evidence is not an FDB candidate, even with a single learned MAC"
    );
    assert_eq!(lab.resolve().await.unwrap(), 0);
    assert_eq!(lab.resolved_neighbor(anchor.id).await, None);
}

/// The other half of "ordering is load-bearing": a port already resolved (by LLDP, or by an
/// earlier FDB pass) is not re-examined here.
#[tokio::test]
async fn an_interface_already_resolved_is_left_alone() {
    let lab = Lab::new().await;
    let switch = lab.host("switch").await;
    let far_end = lab.host("far-end").await;
    lab.port(far_end.id, 1, "Gi0/1", Some("00:1a:2b:00:10:04"))
        .await;

    let anchor = lab
        .fdb_port(switch.id, 5, "Gi0/5", "00:1a:2b:00:10:04")
        .await;

    let scan_time = Utc::now();
    lab.interface_neighbor_service
        .reconcile_interface_neighbors(
            lab.network_id,
            anchor.id,
            &[(Neighbor::Host(far_end.id), Some(scan_time))],
            scan_time,
            None,
        )
        .await
        .unwrap();

    assert_eq!(
        lab.unresolved_fdb_count().await,
        0,
        "a port with an existing resolved row is not an FDB candidate"
    );
    assert_eq!(lab.resolve().await.unwrap(), 0);
}

/// The SQL filter's `jsonb_array_length(fdb_col) = 1` check, not `resolve_fdb_links`'s own
/// re-check of the same condition, is what a passing call path actually exercises — an interface
/// with zero or several learned MACs never reaches the function as `unresolved` in the first
/// place.
#[tokio::test]
async fn a_port_with_two_learned_macs_is_not_an_fdb_candidate() {
    let lab = Lab::new().await;
    let switch = lab.host("switch").await;

    let entry = Interface::new(InterfaceBase {
        host_id: switch.id,
        network_id: lab.network_id,
        if_index: Some(5),
        if_descr: Some("Gi0/5".to_string()),
        if_type: Some(if_type::ETHERNET_CSMA_CD),
        fdb_macs: Some(vec![
            "00:1a:2b:00:10:05".to_string(),
            "00:1a:2b:00:10:06".to_string(),
        ]),
        admin_status: Some(IfAdminStatus::Up),
        oper_status: Some(IfOperStatus::Up),
        ..Default::default()
    });
    lab.storage.interfaces.create(&entry).await.unwrap();

    assert_eq!(lab.unresolved_fdb_count().await, 0);
    assert_eq!(lab.resolve().await.unwrap(), 0);
}

/// GH #709 end to end, against the simulated device the report describes: bridge FDB served, no
/// LLDP remote table at all. `switch-fdb-only-01`'s one learned MAC is `switch-core-01`'s Gi0/3 —
/// a real device already in the lab, so this resolves against genuine collected data on both ends
/// rather than a MAC nothing else recognises.
#[tokio::test]
async fn a_bridge_only_device_resolves_its_single_mac_port_via_resolve_fdb_links() {
    let lab = Lab::new().await;
    let core = lab.scan("switch-core-01").await;
    let anchor_host = lab.scan("switch-fdb-only-01").await;

    let far_port = lab
        .storage
        .interfaces
        .get_all(StorableFilter::<Interface>::new_from_network_ids(&[
            lab.network_id
        ]))
        .await
        .unwrap()
        .into_iter()
        .find(|i| i.base.host_id == core.id && i.base.if_index == Some(3))
        .expect("switch-core-01's Gi0/3");

    let anchor = lab
        .storage
        .interfaces
        .get_all(StorableFilter::<Interface>::new_from_network_ids(&[
            lab.network_id
        ]))
        .await
        .unwrap()
        .into_iter()
        .find(|i| i.base.host_id == anchor_host.id)
        .expect("switch-fdb-only-01's one port");

    // `BridgeFdbEntry::mac_address.to_string()` (what production's own `convert_snmp_if_entry`
    // stores) renders uppercase — asserted here so the value driving resolution below is provably
    // the collected one, not a hand-typed lowercase stand-in.
    assert_eq!(
        anchor.base.fdb_macs.as_deref(),
        Some(["00:1A:2B:00:10:03".to_string()].as_slice()),
        "the collected FDB, not a hand-typed value, is what this test resolves"
    );

    assert_eq!(lab.resolve().await.unwrap(), 1);
    assert_eq!(
        lab.resolved_neighbor(anchor.id).await,
        Some(Neighbor::Interface(far_port.id)),
        "a switch with no LLDP remote table at all must still produce a physical link from FDB"
    );
}
