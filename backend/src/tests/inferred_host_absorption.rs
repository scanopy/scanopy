//! An LLDP neighbour mints an Inferred host from a chassis id no host on the site carries. When a
//! scan later reads that chassis id off a host found earlier by ping alone, the two records are one
//! device, and the scan merges the Inferred one into it (`inferred_hosts_sharing_chassis_id`).
//!
//! `create_with_children` is shared by discovery, the neighbour mint and the forwarding-table mint,
//! so beside the merge these pin every path that must not merge: a mint, two scanned hosts sharing
//! the id, an Inferred host holding an address the scan doesn't report, a payload matching the
//! Inferred host itself, and a payload with no chassis id.

use std::net::IpAddr;

use mac_address::MacAddress;
use uuid::Uuid;

use crate::daemon::discovery::types::warnings::DiscoveryWarning;
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::hosts::r#impl::api::ConflictBehavior;
use crate::server::hosts::r#impl::attributes::HostChassisIdValue;
use crate::server::hosts::r#impl::base::{Host, HostBase};
use crate::server::hosts::r#impl::name::{HostName, HostNameSources};
use crate::server::interface_neighbors::r#impl::base::{InterfaceNeighborEvidence, Neighbor};
use crate::server::interfaces::r#impl::base::{Interface, InterfaceBase, InterfaceDataComplete};
use crate::server::ip_addresses::r#impl::base::{
    IPAddress, IPAddressBase, MacEvidence, MacEvidenceValue,
};
use crate::server::lldp::{LldpChassisId, LldpPortId};
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::shared::attribution::{AttributeSource, Attributed};
use crate::server::shared::services::factory::ServiceFactory;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::factory::StorageFactory;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::{Storable, Storage};
use crate::server::shared::types::entities::{EntitySource, InferredVia};
use crate::server::sites::r#impl::{Site, SiteBase};
use crate::server::subnets::r#impl::base::{
    Subnet, SubnetBase, SubnetCidr, SubnetCidrValue, SubnetName, SubnetTypeAttributed,
};
use crate::server::subnets::r#impl::types::SubnetType;

use super::{organization, test_services};

/// switch-voss-01 on snmp-test: the chassis MAC its neighbours advertise.
const CHASSIS: &str = "00:04:38:02:e0:00";
/// Its address, and the MAC an ARP sweep read for it. The chassis MAC is not that MAC.
const ADDRESS: &str = "192.168.7.201";
const ARP_MAC: &str = "b2:78:ef:e9:a2:61";

const SNMP: AttributeSource = AttributeSource::Probe(ClientProbe::Snmp);

struct Lab {
    services: ServiceFactory,
    storage: StorageFactory,
    site_id: Uuid,
    lan: Uuid,
    _container: super::TestDb,
}

impl Lab {
    async fn new() -> Self {
        let (storage, services, _container) = test_services().await;
        let org = organization();
        storage.organizations.create(&org).await.unwrap();
        let site = services
            .site_service
            .create(
                Site::new(SiteBase::new(org.id)),
                AuthenticatedEntity::System,
            )
            .await
            .unwrap();
        let lan = services
            .subnet_service
            .create(
                Subnet::new(SubnetBase {
                    name: SubnetName::unattributed("192.168.7.0/24".to_string()),
                    site_id: site.id,
                    cidr: SubnetCidr::new(
                        SubnetCidrValue("192.168.7.0/24".parse().unwrap()),
                        AttributeSource::DaemonSelfReport,
                    ),
                    subnet_type: SubnetTypeAttributed::unattributed(SubnetType::Lan),
                    source: EntitySource::Discovery,
                    ..Default::default()
                }),
                AuthenticatedEntity::System,
            )
            .await
            .unwrap()
            .id;
        Self {
            services,
            storage,
            site_id: site.id,
            lan,
            _container,
        }
    }

    fn address(&self, ip: &str, mac: &str, source: AttributeSource) -> IPAddress {
        IPAddress::new(IPAddressBase {
            site_id: self.site_id,
            subnet_id: self.lan,
            ip_address: ip.parse::<IpAddr>().unwrap(),
            mac_address: Some(MacEvidence::new(
                MacEvidenceValue(mac.parse::<MacAddress>().unwrap()),
                source,
            )),
            ..Default::default()
        })
    }

    /// One scan of the switch, as a daemon submits it. `chassis_id` is set once SNMP answered.
    async fn scan(&self, chassis_id: Option<&str>, source: AttributeSource) -> Uuid {
        let mut base = HostBase {
            site_id: self.site_id,
            source: EntitySource::Discovery,
            ..Default::default()
        };
        base.chassis_id = chassis_id.map(|id| Attributed::new(HostChassisIdValue(id.into()), SNMP));
        self.services
            .host_service
            .discover_host(
                Host::new(base),
                vec![self.address(ADDRESS, ARP_MAC, source)],
                vec![],
                vec![],
                vec![],
                vec![],
                false,
                InterfaceDataComplete::none(),
                &crate::server::ports::r#impl::coverage::PortCoverage::default(),
                None,
                AuthenticatedEntity::System,
                None,
            )
            .await
            .expect("the scan persists")
            .host
            .id
    }

    /// A ping sweep's record of the switch: an address and an ARP MAC, nothing from SNMP.
    async fn ping_found(&self) -> Uuid {
        self.scan(None, AttributeSource::ArpReply).await
    }

    /// What the neighbour pass submits for a far end it could not place (`infer_far_end_subnets`).
    async fn mint(&self, chassis_id: &str, ip_addresses: Vec<IPAddress>) -> Uuid {
        let mut base = HostBase {
            site_id: self.site_id,
            source: EntitySource::Inferred {
                via: InferredVia::Neighbour,
            },
            ..Default::default()
        };
        base.chassis_id = Some(Attributed::new(
            HostChassisIdValue(chassis_id.into()),
            AttributeSource::LldpChassisId,
        ));
        self.services
            .host_service
            .create_with_children(
                Host::new(base),
                ip_addresses,
                vec![],
                vec![],
                vec![],
                vec![],
                ConflictBehavior::Upsert,
                AuthenticatedEntity::System,
                None,
                false,
                InterfaceDataComplete::none(),
                &crate::server::ports::r#impl::coverage::PortCoverage::default(),
            )
            .await
            .expect("the mint persists")
            .host
            .id
    }

    /// A scanned switch with one walked port whose LLDP table names the far end by chassis MAC and
    /// sysName, with no management address, as snmp-test's switch-quietcol-01 does.
    async fn neighbour(&self) -> Interface {
        let mut h = super::host(&self.site_id);
        h.base.name = HostName::manual("switch-quietcol-01".to_string());
        h.base.source = EntitySource::Discovery;
        self.storage.hosts.create(&h).await.unwrap();
        let port = Interface::new(InterfaceBase {
            site_id: self.site_id,
            host_id: h.id,
            if_index: Some(3),
            if_descr: Some("Gi0/3".to_string()),
            if_name: Some("Gi0/3".to_string()),
            ..Default::default()
        });
        self.storage.interfaces.create(&port).await.unwrap();
        self.services
            .interface_neighbor_service
            .replace_candidates_from_discovery(
                self.site_id,
                port.id,
                vec![InterfaceNeighborEvidence {
                    lldp_chassis_id: Some(LldpChassisId::MacAddress(CHASSIS.to_string())),
                    lldp_port_id: Some(LldpPortId::InterfaceName("1/2".to_string())),
                    lldp_sys_name: Some("switch-voss-01".to_string()),
                    ..Default::default()
                }],
                InterfaceDataComplete::default(),
            )
            .await
            .unwrap();
        port
    }

    async fn resolve(&self) {
        self.services
            .host_service
            .resolve_lldp_links(
                self.site_id,
                crate::server::shared::clock::now(),
                &Default::default(),
            )
            .await
            .unwrap();
    }

    async fn live(&self) -> Vec<Host> {
        self.services
            .host_service
            .get_all(StorableFilter::<Host>::new_from_site_ids(&[self.site_id]).live())
            .await
            .unwrap()
    }

    async fn holding_chassis(&self) -> Vec<Host> {
        self.live()
            .await
            .into_iter()
            .filter(|h| {
                crate::server::shared::attribution::text_of(&h.base.chassis_id).as_deref()
                    == Some(CHASSIS)
            })
            .collect()
    }

    /// The host each adjacency on `port` names, through a bound far-end port where there is one.
    async fn far_hosts(&self, port: Uuid) -> Vec<Uuid> {
        let mut hosts = Vec::new();
        for row in self
            .services
            .interface_neighbor_service
            .resolved_for_interface(&port)
            .await
            .unwrap()
        {
            hosts.push(match row.neighbor {
                Neighbor::Host(id) => id,
                Neighbor::Interface(id) => {
                    self.services
                        .interface_service
                        .get_by_id(&id)
                        .await
                        .unwrap()
                        .expect("bound port is live")
                        .base
                        .host_id
                }
            });
        }
        hosts
    }
}

fn is_inferred(host: &Host) -> bool {
    matches!(host.base.source, EntitySource::Inferred { .. })
}

/// snmp-test, 2026-11-11 to 11-12: the switch is found by ping at 22:03, the neighbour pass at
/// 22:11:52 finds nothing carrying its chassis MAC or sysName and mints an Inferred host, and the
/// first scan that walks the switch reads that chassis MAC off it. The scan leaves one host, with
/// the scanned host's id and the Inferred host's link.
#[tokio::test]
async fn a_scan_reading_a_minted_chassis_id_absorbs_the_inferred_host() {
    let lab = Lab::new().await;
    let switch = lab.ping_found().await;
    let port = lab.neighbour().await;

    lab.resolve().await;
    let minted = lab.holding_chassis().await;
    assert_eq!(minted.len(), 1, "the neighbour pass mints the far end");
    assert!(is_inferred(&minted[0]));
    assert_eq!(lab.far_hosts(port.id).await, vec![minted[0].id]);

    let scanned = lab.scan(Some(CHASSIS), SNMP).await;
    assert_eq!(scanned, switch, "the scan lands on the ping-found host");

    let holders = lab.holding_chassis().await;
    assert_eq!(
        holders.iter().map(|h| h.id).collect::<Vec<_>>(),
        vec![switch],
        "one host carries the chassis id, and it is the scanned one"
    );
    assert!(lab.live().await.iter().all(|h| !is_inferred(h)));
    assert_eq!(lab.far_hosts(port.id).await, vec![switch]);

    // The next pass resolves the neighbour to the one holder, not to two.
    lab.resolve().await;
    assert_eq!(lab.far_hosts(port.id).await, vec![switch]);
    assert_eq!(lab.holding_chassis().await.len(), 1);
}

/// Two scanned hosts sharing a chassis id can be distinct devices (GH #668). Neither is merged.
#[tokio::test]
async fn two_scanned_hosts_sharing_a_chassis_id_are_left_alone() {
    let lab = Lab::new().await;
    let scanned = lab.scan(Some(CHASSIS), SNMP).await;
    let mut other = super::host(&lab.site_id);
    other.base.source = EntitySource::Discovery;
    other.base.chassis_id = Some(Attributed::new(HostChassisIdValue(CHASSIS.into()), SNMP));
    lab.storage.hosts.create(&other).await.unwrap();

    lab.scan(Some(CHASSIS), SNMP).await;

    let mut ids: Vec<Uuid> = lab.holding_chassis().await.iter().map(|h| h.id).collect();
    ids.sort();
    let mut expected = vec![scanned, other.id];
    expected.sort();
    assert_eq!(ids, expected);
}

/// The neighbour mint goes through the same matcher. A mint payload landing on the scanned host by
/// its address merges nothing, even with an Inferred host holding its chassis id.
#[tokio::test]
async fn a_mint_never_absorbs_an_inferred_host() {
    let lab = Lab::new().await;
    let switch = lab.ping_found().await;
    let inferred = lab.mint(CHASSIS, vec![]).await;
    assert_ne!(inferred, switch);

    let landed = lab
        .mint(
            CHASSIS,
            vec![lab.address(ADDRESS, ARP_MAC, AttributeSource::LldpChassisId)],
        )
        .await;
    assert_eq!(
        landed, switch,
        "the mint matched the scanned host by address"
    );

    let live: Vec<Uuid> = lab.live().await.iter().map(|h| h.id).collect();
    assert!(live.contains(&switch));
    assert!(live.contains(&inferred), "a mint merged an Inferred host");
}

/// A far end that published an address the scan doesn't report is a different device.
#[tokio::test]
async fn an_inferred_host_at_an_address_the_scan_does_not_report_is_kept() {
    let lab = Lab::new().await;
    let switch = lab.ping_found().await;
    let inferred = lab
        .mint(
            CHASSIS,
            vec![lab.address(
                "192.168.7.99",
                "02:00:00:00:00:99",
                AttributeSource::LldpChassisId,
            )],
        )
        .await;

    lab.scan(Some(CHASSIS), SNMP).await;

    let live: Vec<Uuid> = lab.live().await.iter().map(|h| h.id).collect();
    assert!(live.contains(&switch));
    assert!(live.contains(&inferred));
}

/// With no other holder, a scan carrying the chassis id matches the Inferred host itself and
/// promotes it. There is nothing to merge, and no attempt to merge a host into itself.
#[tokio::test]
async fn a_scan_matching_the_inferred_host_itself_promotes_it() {
    let lab = Lab::new().await;
    let inferred = lab.mint(CHASSIS, vec![]).await;

    let scanned = lab.scan(Some(CHASSIS), SNMP).await;

    assert_eq!(scanned, inferred);
    let live = lab.live().await;
    assert_eq!(live.len(), 1);
    assert_eq!(live[0].base.source, EntitySource::Discovery);
}

/// A scan that read no chassis id proves nothing, and leaves the Inferred host as it is today.
#[tokio::test]
async fn a_scan_without_a_chassis_id_merges_nothing() {
    let lab = Lab::new().await;
    let switch = lab.ping_found().await;
    let inferred = lab.mint(CHASSIS, vec![]).await;

    lab.scan(None, SNMP).await;

    let live: Vec<Uuid> = lab.live().await.iter().map(|h| h.id).collect();
    assert!(live.contains(&switch));
    assert!(live.contains(&inferred));
}

/// Two scanned hosts carrying the chassis id a neighbour advertises: the neighbour resolves to
/// neither, and its warning names both, so the row can offer to merge them.
#[tokio::test]
async fn the_ambiguity_warning_names_the_hosts_it_could_not_choose_between() {
    let lab = Lab::new().await;
    let scanned = lab.scan(Some(CHASSIS), SNMP).await;
    let mut other = super::host(&lab.site_id);
    other.base.source = EntitySource::Discovery;
    other.base.chassis_id = Some(Attributed::new(HostChassisIdValue(CHASSIS.into()), SNMP));
    lab.storage.hosts.create(&other).await.unwrap();
    let port = lab.neighbour().await;

    let outcome = lab
        .services
        .host_service
        .resolve_lldp_links(
            lab.site_id,
            crate::server::shared::clock::now(),
            &Default::default(),
        )
        .await
        .unwrap();

    let candidates: Vec<Vec<Uuid>> = outcome
        .warnings
        .iter()
        .filter_map(|w| match w {
            DiscoveryWarning::LldpNeighbourAmbiguous(n) => Some(n.candidate_host_ids.clone()),
            _ => None,
        })
        .collect();
    let mut expected = vec![scanned, other.id];
    expected.sort();
    assert_eq!(candidates.len(), 1, "one ambiguous neighbour, one warning");
    let mut named = candidates[0].clone();
    named.sort();
    assert_eq!(named, expected);
    assert!(lab.far_hosts(port.id).await.is_empty(), "no link to either");
}
