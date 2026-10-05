//! `HostService::record_advertised_far_end_ports` against a real database (GH #717).
//!
//! A far end that answers no ifTable gets its ports from what its neighbours advertise. Driven
//! through `resolve_lldp_links`, the production entry, because the rows it writes are what the next
//! pass resolves against: whether a pass is idempotent depends on the snapshot finding them.

use std::collections::{HashMap, HashSet};

use chrono::Utc;
use uuid::Uuid;

use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    daemons::r#impl::api::ScannedEntityIds,
    hosts::{
        r#impl::{
            attributes::HostChassisIdValue,
            base::Host,
            name::{HostName, HostNameSources},
        },
        service::HostService,
    },
    interface_neighbors::{
        r#impl::base::{InterfaceNeighborEvidence, Neighbor},
        service::InterfaceNeighborService,
    },
    interfaces::r#impl::base::{
        IfAdminStatus, IfOperStatus, Interface, InterfaceBase, InterfaceDataComplete, if_type,
    },
    lldp::{LldpChassisId, LldpPortId},
    services::r#impl::patterns::ClientProbe,
    shared::{
        attribution::{AttributeSource, Attributed},
        storage::{factory::StorageFactory, filter::StorableFilter, traits::Storage},
        types::entities::EntitySource,
    },
};

use super::{host, network, organization, subnet, test_services};

/// The chassis id of the device every test is about: a management switch that answers only its
/// system MIB.
const MUTE_SWITCH: &str = "mgmt-switch";

struct Lab {
    host_service: std::sync::Arc<HostService>,
    neighbours: std::sync::Arc<InterfaceNeighborService>,
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
        storage.subnets.create(&subnet(&network.id)).await.unwrap();

        Self {
            host_service: services.host_service.clone(),
            neighbours: services.interface_neighbor_service.clone(),
            network_id: network.id,
            storage,
            _container,
        }
    }

    /// A host carrying `chassis_id` and nothing else: no interfaces, no addresses.
    async fn far_end(&self, chassis_id: &str) -> Host {
        let mut h = host(&self.network_id);
        h.base.name = HostName::manual(chassis_id.to_string());
        h.base.chassis_id = Some(Attributed::new(
            HostChassisIdValue(chassis_id.to_string()),
            AttributeSource::Probe(ClientProbe::Snmp),
        ));
        self.storage.hosts.create(&h).await.unwrap();
        h
    }

    /// A walked port: the shape an ifTable row has, `if_index` included.
    async fn walked_port(&self, host_id: Uuid, if_index: i32, name: &str) -> Interface {
        let entry = walked(self.network_id, host_id, if_index, name);
        self.storage.interfaces.create(&entry).await.unwrap();
        entry
    }

    /// A scanned neighbour with one walked port whose LLDP entry carries `evidence`. Returns that
    /// port, the interface the resulting adjacency hangs off.
    async fn neighbour_with(&self, name: &str, evidence: InterfaceNeighborEvidence) -> Interface {
        let mut h = host(&self.network_id);
        h.base.name = HostName::manual(name.to_string());
        self.storage.hosts.create(&h).await.unwrap();
        let port = self.walked_port(h.id, 1, &format!("{name}:ma1")).await;
        self.neighbours
            .replace_candidates_from_discovery(
                self.network_id,
                port.id,
                vec![evidence],
                InterfaceDataComplete::default(),
            )
            .await
            .unwrap();
        port
    }

    /// A neighbour that names `chassis_id` as its far end and `port` as the port it answers on.
    async fn neighbour(&self, name: &str, chassis_id: &str, port: &str) -> Interface {
        self.neighbour_with(
            name,
            InterfaceNeighborEvidence {
                lldp_chassis_id: Some(LldpChassisId::LocallyAssigned(chassis_id.to_string())),
                lldp_port_id: Some(LldpPortId::InterfaceName(port.to_string())),
                ..Default::default()
            },
        )
        .await
    }

    async fn resolve(&self) {
        self.host_service
            .resolve_lldp_links(self.network_id, Utc::now(), &Default::default())
            .await
            .unwrap();
    }

    async fn ports(&self, host_id: Uuid) -> Vec<Interface> {
        self.storage
            .interfaces
            .get_all(StorableFilter::<Interface>::new_from_host_ids(&[host_id]).live())
            .await
            .unwrap()
    }

    async fn port_names(&self, host_id: Uuid) -> HashSet<String> {
        self.ports(host_id)
            .await
            .into_iter()
            .filter_map(|i| i.base.if_name)
            .collect()
    }

    async fn host_by_chassis_id(&self, chassis_id: &str) -> Host {
        self.storage
            .hosts
            .get_all(StorableFilter::<Host>::new_from_network_ids(&[self.network_id]).live())
            .await
            .unwrap()
            .into_iter()
            .find(|h| {
                h.base
                    .chassis_id
                    .as_ref()
                    .is_some_and(|c| c.value().0 == chassis_id)
            })
            .expect("a host carrying the chassis id")
    }

    /// The adjacencies resolved on one local port.
    async fn neighbours_of(&self, interface_id: Uuid) -> Vec<Neighbor> {
        self.neighbours
            .resolved_for_interface(&interface_id)
            .await
            .unwrap()
            .into_iter()
            .map(|row| row.neighbor)
            .collect()
    }

    /// Walk `far_end` the way a daemon submits it, reporting `ports` as its ifTable.
    async fn walk(&self, far_end: &Host, ports: &[(i32, &str)], complete: bool) {
        let interfaces = ports
            .iter()
            .map(|(if_index, name)| walked(self.network_id, far_end.id, *if_index, name))
            .collect();
        self.host_service
            .discover_host(
                far_end.clone(),
                vec![],
                vec![],
                vec![],
                interfaces,
                vec![],
                complete,
                InterfaceDataComplete::default(),
                None,
                AuthenticatedEntity::System,
                None,
            )
            .await
            .unwrap();
    }
}

fn walked(network_id: Uuid, host_id: Uuid, if_index: i32, name: &str) -> Interface {
    Interface::new(InterfaceBase {
        network_id,
        host_id,
        if_index: Some(if_index),
        if_descr: Some(name.to_string()),
        if_name: Some(name.to_string()),
        if_type: Some(if_type::ETHERNET_CSMA_CD),
        admin_status: Some(IfAdminStatus::Up),
        oper_status: Some(IfOperStatus::Up),
        ..Default::default()
    })
}

/// The reporter's lab: six neighbours naming the management switch, each on its own port.
const SIX: [(&str, &str); 6] = [
    ("netlab-leaf1", "Ethernet46"),
    ("netlab-leaf1-swp46", "Ethernet48"),
    ("netlab-leaf2", "Ethernet47"),
    ("netlab-server", "Ethernet2"),
    ("netlab-spine1", "Ethernet44"),
    ("netlab-spine2", "Ethernet45"),
];

fn names(ports: &[(&str, &str)]) -> HashSet<String> {
    ports.iter().map(|(_, port)| port.to_string()).collect()
}

// ---------------------------------------------------------------------------------------------
// A far end a scan's neighbours re-advertise is seen by that scan.
// ---------------------------------------------------------------------------------------------

/// An Inferred far end with its Last seen at the scan that minted it, named again by `neighbour`.
async fn re_advertised_far_end(lab: &Lab) -> (Host, Interface) {
    let mut switch = lab.far_end(MUTE_SWITCH).await;
    switch.base.source = EntitySource::Inferred;
    // Microseconds, the precision Postgres keeps, so a test can compare the stored value to this
    // one. Linux clocks carry nanoseconds, which the round trip drops.
    switch.last_seen_at =
        chrono::SubsecRound::trunc_subsecs(Utc::now() - chrono::Duration::days(3), 6);
    lab.storage.hosts.update(&mut switch).await.unwrap();
    let neighbour = lab.neighbour("access-1", MUTE_SWITCH, "ge-0/0/1").await;
    (switch, neighbour)
}

#[tokio::test]
async fn a_far_end_a_scan_re_reads_is_seen_by_that_scan() {
    let lab = Lab::new().await;
    let (switch, neighbour) = re_advertised_far_end(&lab).await;
    // Microseconds, the precision Postgres keeps, so the stored value compares equal.
    let scan_time = chrono::SubsecRound::trunc_subsecs(Utc::now(), 6);
    let run = ScannedEntityIds {
        interface_ids: vec![neighbour.id],
        ..Default::default()
    };

    let outcome = lab
        .host_service
        .resolve_lldp_links(lab.network_id, scan_time, &run)
        .await
        .unwrap();

    assert!(outcome.observed.host_ids.contains(&switch.id));
    let refreshed = lab.host_by_chassis_id(MUTE_SWITCH).await;
    assert_eq!(refreshed.last_seen_at, scan_time);
    for port in lab.ports(switch.id).await {
        assert!(outcome.observed.interface_ids.contains(&port.id));
        assert_eq!(port.last_seen_at, scan_time, "its ports are seen with it");
    }
}

/// Candidates on an interface this scan did not walk were read by an earlier one.
#[tokio::test]
async fn a_neighbour_this_scan_did_not_walk_is_no_evidence() {
    let lab = Lab::new().await;
    let (switch, _neighbour) = re_advertised_far_end(&lab).await;

    let outcome = lab
        .host_service
        .resolve_lldp_links(lab.network_id, Utc::now(), &ScannedEntityIds::default())
        .await
        .unwrap();

    assert!(!outcome.observed.host_ids.contains(&switch.id));
    assert_eq!(
        lab.host_by_chassis_id(MUTE_SWITCH).await.last_seen_at,
        switch.last_seen_at
    );
}

// ---------------------------------------------------------------------------------------------
// GH #717: every distinct advertised port, for as long as the device has described none itself.
// ---------------------------------------------------------------------------------------------

/// The reporter's management switch: six neighbours, six ports, six rows.
#[tokio::test]
async fn a_far_end_named_on_six_ports_holds_all_six() {
    let lab = Lab::new().await;
    let switch = lab.far_end(MUTE_SWITCH).await;
    for (name, port) in SIX {
        lab.neighbour(name, MUTE_SWITCH, port).await;
    }

    lab.resolve().await;

    assert_eq!(lab.port_names(switch.id).await, names(&SIX));
}

/// A neighbour that first names the far end after its ports were recorded still adds its port.
#[tokio::test]
async fn a_port_named_later_joins_the_recorded_ones() {
    let lab = Lab::new().await;
    let switch = lab.far_end(MUTE_SWITCH).await;
    for (name, port) in SIX {
        lab.neighbour(name, MUTE_SWITCH, port).await;
    }
    lab.resolve().await;

    lab.neighbour("netlab-leaf3", MUTE_SWITCH, "Ethernet1")
        .await;
    lab.resolve().await;

    let mut expected = names(&SIX);
    expected.insert("Ethernet1".to_string());
    assert_eq!(lab.port_names(switch.id).await, expected);
}

/// Minting keeps the first port named for a new far end; the pass after it records the rest.
#[tokio::test]
async fn a_minted_far_end_named_on_three_ports_holds_all_three() {
    let lab = Lab::new().await;
    for (name, port) in &SIX[..3] {
        lab.neighbour(name, MUTE_SWITCH, port).await;
    }

    lab.resolve().await;
    lab.resolve().await;

    let minted = lab.host_by_chassis_id(MUTE_SWITCH).await;
    assert_eq!(lab.port_names(minted.id).await, names(&SIX[..3]));
}

/// A far end that repeats one MAC as every port id, with each neighbour also reporting the port's
/// description. With each port recorded, the MAC is ambiguous on that device and each neighbour
/// lands on its own description.
#[tokio::test]
async fn neighbours_sharing_a_port_mac_bind_to_their_own_ports() {
    let lab = Lab::new().await;
    let switch = lab.far_end(MUTE_SWITCH).await;
    let mut named: Vec<(Uuid, &str)> = Vec::new();
    for (name, port) in &SIX[..3] {
        let local = lab
            .neighbour_with(
                name,
                InterfaceNeighborEvidence {
                    lldp_chassis_id: Some(LldpChassisId::LocallyAssigned(MUTE_SWITCH.into())),
                    lldp_port_id: Some(LldpPortId::MacAddress("e8:80:88:be:30:e7".into())),
                    lldp_port_desc: Some(port.to_string()),
                    ..Default::default()
                },
            )
            .await;
        named.push((local.id, port));
    }

    lab.resolve().await;
    lab.resolve().await;

    let recorded: HashMap<String, Uuid> = lab
        .ports(switch.id)
        .await
        .into_iter()
        .filter_map(|i| i.base.if_name.map(|n| (n, i.id)))
        .collect();
    assert_eq!(recorded.len(), 3);
    for (local, port) in named {
        assert_eq!(
            lab.neighbours_of(local).await,
            vec![Neighbor::Interface(recorded[port])],
            "the neighbour naming {port}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Regression guards. Each asserts a property that holds for one synthesised row and must keep
// holding for several.
// ---------------------------------------------------------------------------------------------

/// A walked port is the device describing itself; an advertisement must not add beside it.
#[tokio::test]
async fn a_walked_far_end_gains_nothing_from_an_advertisement() {
    let lab = Lab::new().await;
    let switch = lab.far_end(MUTE_SWITCH).await;
    let own = lab.walked_port(switch.id, 1, "Gi0/1").await;
    lab.neighbour("leaf1", MUTE_SWITCH, "1").await;
    lab.neighbour("leaf2", MUTE_SWITCH, "2").await;

    lab.resolve().await;
    lab.resolve().await;

    let ids: Vec<Uuid> = lab.ports(switch.id).await.iter().map(|i| i.id).collect();
    assert_eq!(ids, vec![own.id]);
}

/// A pass over state it already wrote writes nothing.
#[tokio::test]
async fn a_repeat_pass_adds_nothing() {
    let lab = Lab::new().await;
    let switch = lab.far_end(MUTE_SWITCH).await;
    for (name, port) in SIX {
        lab.neighbour(name, MUTE_SWITCH, port).await;
    }

    lab.resolve().await;
    let first: HashSet<Uuid> = lab.ports(switch.id).await.iter().map(|i| i.id).collect();
    lab.resolve().await;
    lab.resolve().await;
    let later: HashSet<Uuid> = lab.ports(switch.id).await.iter().map(|i| i.id).collect();

    assert!(!first.is_empty());
    assert_eq!(first, later);
}

/// Two neighbours naming one port describe one port. `(host_id, if_name)` is unique on live rows,
/// so a second insert would fail rather than duplicate, and the pass must not attempt it.
#[tokio::test]
async fn two_neighbours_naming_one_port_make_one_row() {
    let lab = Lab::new().await;
    let switch = lab.far_end(MUTE_SWITCH).await;
    lab.neighbour("leaf1", MUTE_SWITCH, "Ethernet46").await;
    lab.neighbour("leaf2", MUTE_SWITCH, "Ethernet46").await;

    lab.resolve().await;
    lab.resolve().await;

    let ports = lab.ports(switch.id).await;
    assert_eq!(ports.len(), 1);
    assert_eq!(ports[0].base.if_name.as_deref(), Some("Ethernet46"));
}

/// A far end that uses its chassis MAC as every port id advertises one MAC however many
/// neighbours report it, and that is one port as far as anything can tell.
#[tokio::test]
async fn neighbours_advertising_one_port_mac_make_one_row() {
    let lab = Lab::new().await;
    let switch = lab.far_end(MUTE_SWITCH).await;
    for name in ["leaf1", "leaf2", "leaf3"] {
        lab.neighbour_with(
            name,
            InterfaceNeighborEvidence {
                lldp_chassis_id: Some(LldpChassisId::LocallyAssigned(MUTE_SWITCH.into())),
                lldp_port_id: Some(LldpPortId::MacAddress("e8:80:88:be:30:e7".into())),
                ..Default::default()
            },
        )
        .await;
    }

    lab.resolve().await;
    lab.resolve().await;

    assert_eq!(lab.ports(switch.id).await.len(), 1);
}

/// A complete walk replaces advertisements with the device's own ifTable. A synthesised row the
/// walk also reports is the same port, so it is updated in place rather than replaced.
#[tokio::test]
async fn a_complete_walk_leaves_only_walked_ports() {
    let lab = Lab::new().await;
    let switch = lab.far_end(MUTE_SWITCH).await;
    lab.neighbour("leaf1", MUTE_SWITCH, "Ethernet46").await;
    lab.neighbour("leaf2", MUTE_SWITCH, "Ethernet48").await;
    lab.resolve().await;

    let before: HashMap<String, Uuid> = lab
        .ports(switch.id)
        .await
        .into_iter()
        .filter_map(|i| i.base.if_name.map(|n| (n, i.id)))
        .collect();
    assert!(!before.is_empty());

    lab.walk(&switch, &[(46, "Ethernet46"), (1, "Ethernet1")], true)
        .await;

    let after = lab.ports(switch.id).await;
    assert!(after.iter().all(|i| i.base.if_index.is_some()));
    assert_eq!(
        lab.port_names(switch.id).await,
        HashSet::from(["Ethernet46".to_string(), "Ethernet1".to_string()])
    );
    if let Some(id) = before.get("Ethernet46") {
        let walked_46 = after
            .iter()
            .find(|i| i.base.if_name.as_deref() == Some("Ethernet46"))
            .unwrap();
        assert_eq!(walked_46.id, *id, "the walk updates the row it matches");
    }
}

/// A walk cut short is no authority to prune, and once the device has described any port of its
/// own, advertisements stop adding to it.
#[tokio::test]
async fn a_partial_walk_neither_prunes_nor_lets_advertisements_add() {
    let lab = Lab::new().await;
    let switch = lab.far_end(MUTE_SWITCH).await;
    lab.neighbour("leaf1", MUTE_SWITCH, "Ethernet46").await;
    lab.neighbour("leaf2", MUTE_SWITCH, "Ethernet48").await;
    lab.resolve().await;
    let synthesised = lab.port_names(switch.id).await;

    lab.walk(&switch, &[(1, "Ethernet1")], false).await;
    lab.neighbour("leaf3", MUTE_SWITCH, "Ethernet50").await;
    lab.resolve().await;
    lab.resolve().await;

    let names = lab.port_names(switch.id).await;
    assert!(synthesised.is_subset(&names), "nothing pruned");
    assert!(names.contains("Ethernet1"));
    assert!(!names.contains("Ethernet50"), "nothing added");
}

/// A port recorded during a scan is bound during that scan. The pass that records it has already
/// placed its links, so resolution runs again rather than leaving the neighbour on the device until
/// the next scan.
#[tokio::test]
async fn a_recorded_port_is_bound_in_the_scan_that_records_it() {
    let lab = Lab::new().await;
    let switch = lab.far_end(MUTE_SWITCH).await;
    let mut named: Vec<(Uuid, &str)> = Vec::new();
    for (name, port) in SIX {
        named.push((lab.neighbour(name, MUTE_SWITCH, port).await.id, port));
    }

    lab.resolve().await;

    let recorded: HashMap<String, Uuid> = lab
        .ports(switch.id)
        .await
        .into_iter()
        .filter_map(|i| i.base.if_name.map(|n| (n, i.id)))
        .collect();
    for (local, port) in named {
        assert_eq!(
            lab.neighbours_of(local).await,
            vec![Neighbor::Interface(recorded[port])],
            "the neighbour naming {port}"
        );
    }
}

/// The point of recording a port: the neighbour that named it links to it on the next pass.
#[tokio::test]
async fn each_neighbour_binds_to_the_port_recorded_for_it() {
    let lab = Lab::new().await;
    let switch = lab.far_end(MUTE_SWITCH).await;
    let mut named: Vec<(Uuid, &str)> = Vec::new();
    for (name, port) in SIX {
        named.push((lab.neighbour(name, MUTE_SWITCH, port).await.id, port));
    }

    lab.resolve().await;
    lab.resolve().await;

    let recorded: HashMap<String, Uuid> = lab
        .ports(switch.id)
        .await
        .into_iter()
        .filter_map(|i| i.base.if_name.map(|n| (n, i.id)))
        .collect();
    assert!(!recorded.is_empty());
    for (local, port) in named {
        if let Some(&far) = recorded.get(port) {
            assert_eq!(
                lab.neighbours_of(local).await,
                vec![Neighbor::Interface(far)],
                "the neighbour naming {port}"
            );
        }
    }
}

/// A link placed by MAC stays on its port when a later advertisement records a sibling carrying
/// the same MAC. The shared MAC reopens the binding, and the port description places it again.
#[tokio::test]
async fn a_mac_bound_link_keeps_its_port_when_a_sibling_shares_the_mac() {
    let lab = Lab::new().await;
    let switch = lab.far_end(MUTE_SWITCH).await;
    let evidence = |desc: &str| InterfaceNeighborEvidence {
        lldp_chassis_id: Some(LldpChassisId::LocallyAssigned(MUTE_SWITCH.into())),
        lldp_port_id: Some(LldpPortId::MacAddress("e8:80:88:be:30:e7".into())),
        lldp_port_desc: Some(desc.to_string()),
        ..Default::default()
    };
    let first = lab.neighbour_with("leaf1", evidence("Ethernet46")).await;
    lab.resolve().await;
    lab.resolve().await;
    let port_46 = lab
        .ports(switch.id)
        .await
        .into_iter()
        .find(|i| i.base.if_name.as_deref() == Some("Ethernet46"))
        .unwrap();
    assert_eq!(
        lab.neighbours_of(first.id).await,
        vec![Neighbor::Interface(port_46.id)]
    );

    lab.neighbour_with("leaf2", evidence("Ethernet48")).await;
    lab.resolve().await;
    lab.resolve().await;

    assert_eq!(
        lab.neighbours_of(first.id).await,
        vec![Neighbor::Interface(port_46.id)]
    );
}

/// A chassis id of subtype `InterfaceName` finds its host by `if_descr` across the whole network.
/// A port recorded from someone else's advertisement is no evidence of which host owns a name, so
/// a generic one (`Ethernet2`) must not make another device's link unresolvable.
///
/// The link naming the other host is created last. An adjacency already bound is carried forward
/// when a pass cannot re-confirm it, so only a first resolution shows whether the lookup answers.
#[tokio::test]
async fn an_advertised_port_name_does_not_contest_another_hosts_identity() {
    let lab = Lab::new().await;
    let switch = lab.far_end(MUTE_SWITCH).await;

    let mut other = host(&lab.network_id);
    other.base.name = HostName::manual("access-sw".to_string());
    lab.storage.hosts.create(&other).await.unwrap();
    lab.walked_port(other.id, 2, "Ethernet2").await;

    lab.neighbour("leaf1", MUTE_SWITCH, "Ethernet46").await;
    lab.resolve().await;
    lab.neighbour("server", MUTE_SWITCH, "Ethernet2").await;
    lab.resolve().await;

    let naming_other = lab
        .neighbour_with(
            "edge",
            InterfaceNeighborEvidence {
                lldp_chassis_id: Some(LldpChassisId::InterfaceName("Ethernet2".into())),
                ..Default::default()
            },
        )
        .await;
    lab.resolve().await;

    assert!(!lab.ports(switch.id).await.is_empty());
    assert_eq!(
        lab.neighbours_of(naming_other.id).await,
        vec![Neighbor::Host(other.id)]
    );
}
