//! Subnet↔VLAN links drawn from a switch's forwarding database, against a real database.
//!
//! The firewall-and-switch network this exists for: the router's interfaces carry the subnets and
//! no VLAN, the switch's access ports carry VLANs and no addresses, and a host's MAC is the only
//! thing both sides saw. Real Postgres, because the evidence is found through `StorableFilter`
//! conditions (`fdb_macs ?|`, `mac_address = ANY`) that only a real query proves.

use std::collections::HashMap;
use std::net::IpAddr;

use mac_address::MacAddress;
use uuid::Uuid;

use crate::daemon::discovery::integration::snmp::sim::harness::{self, Collected};
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::hosts::r#impl::base::{Host, HostBase};
use crate::server::interfaces::r#impl::base::{
    FdbMacs, FdbMacsValue, Interface, InterfaceBase, InterfaceDataComplete, if_type,
};
use crate::server::ip_addresses::r#impl::base::{
    IPAddress, IPAddressBase, MacEvidence, MacEvidenceValue,
};
use crate::server::ports::r#impl::coverage::PortCoverage;
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::shared::attribution::AttributeSource;
use crate::server::shared::services::factory::ServiceFactory;
use crate::server::shared::storage::factory::StorageFactory;
use crate::server::shared::storage::traits::Storage;
use crate::server::shared::types::entities::EntitySource;
use crate::server::vlans::r#impl::base::{VlanName, VlanNameValue};

use super::{organization, site, subnet, test_services};

const SNMP: AttributeSource = AttributeSource::Probe(ClientProbe::Snmp);

/// A MAC `switch-core-01` learned on Gi0/3, its server port, which is an access port in VLAN 20.
const SERVER_MAC: &str = "00:1a:2b:00:14:01";
/// The switch's own management address, on the same subnet as the server.
const SWITCH_IP: &str = "192.168.1.2";

struct Lab {
    storage: StorageFactory,
    services: ServiceFactory,
    site_id: Uuid,
    subnet_id: Uuid,
    /// VLAN number to the VLAN entity the switch's scan resolves it to.
    vlans: HashMap<u16, Uuid>,
    _container: super::TestDb,
}

impl Lab {
    async fn new() -> Self {
        let (storage, services, _container) = test_services().await;
        let org = organization();
        storage.organizations.create(&org).await.unwrap();
        let site = site(&org.id);
        storage.sites.create(&site).await.unwrap();
        let subnet = subnet(&site.id);
        storage.subnets.create(&subnet).await.unwrap();

        let mut vlans = HashMap::new();
        for vlan in harness::scan("switch-core-01").await.vlans {
            let stored = services
                .vlan_service
                .upsert_from_discovery(
                    site.id,
                    org.id,
                    vlan.vlan_id,
                    VlanName::new(
                        VlanNameValue(vlan.name),
                        AttributeSource::Authored(ClientProbe::Snmp),
                    ),
                    None,
                )
                .await
                .unwrap();
            vlans.insert(vlan.vlan_id, stored.id);
        }

        Self {
            storage,
            services,
            site_id: site.id,
            subnet_id: subnet.id,
            vlans,
            _container,
        }
    }

    /// A host with one address on the test subnet, its MAC learned from a router's ARP table.
    async fn server(&self, address: &str, mac: &str) {
        let host = Host::new(HostBase {
            site_id: self.site_id,
            source: EntitySource::Discovery,
            ..Default::default()
        });
        self.storage.hosts.create(&host).await.unwrap();
        let ip = IPAddress::new(IPAddressBase {
            site_id: self.site_id,
            host_id: host.id,
            subnet_id: self.subnet_id,
            ip_address: address.parse().unwrap(),
            mac_address: Some(MacEvidence::new(
                MacEvidenceValue(mac.parse().unwrap()),
                AttributeSource::ForwardingTable,
            )),
            ..Default::default()
        });
        self.storage.ip_addresses.create(&ip).await.unwrap();
    }

    /// Discover `switch-core-01` as the collection read it: its ports, each port's native and
    /// tagged VLANs, and what each port learned, less any MAC in `forgotten`.
    async fn scan_switch(&self, forgotten: &[&str]) {
        let collected = harness::scan("switch-core-01").await;
        let interfaces = self.interfaces_of(&collected, forgotten);

        let host = Host::new(HostBase {
            site_id: self.site_id,
            source: EntitySource::Discovery,
            ..Default::default()
        });
        let ip = IPAddress::new(IPAddressBase {
            site_id: self.site_id,
            subnet_id: self.subnet_id,
            ip_address: SWITCH_IP.parse::<IpAddr>().unwrap(),
            ..Default::default()
        });
        self.services
            .host_service
            .discover_host(
                host,
                vec![ip],
                vec![],
                vec![],
                interfaces,
                vec![],
                true,
                InterfaceDataComplete {
                    lldp: true,
                    cdp: true,
                    fdb: true,
                    vlan_membership: true,
                },
                &PortCoverage::default(),
                None,
                AuthenticatedEntity::System,
                None,
            )
            .await
            .expect("the switch persists");
    }

    fn interfaces_of(&self, collected: &Collected, forgotten: &[&str]) -> Vec<Interface> {
        let forgotten: Vec<MacAddress> = forgotten.iter().map(|m| m.parse().unwrap()).collect();
        collected
            .if_table
            .entries
            .iter()
            .map(|entry| {
                let membership = collected
                    .vlan_membership
                    .iter()
                    .find(|m| m.if_index == entry.if_index);
                let learned: Vec<String> = collected
                    .fdb
                    .records
                    .iter()
                    .filter(|fdb| fdb.if_index == Some(entry.if_index) && fdb.status == 3)
                    .filter(|fdb| !forgotten.contains(&fdb.mac_address))
                    .map(|fdb| fdb.mac_address.to_string())
                    .collect();
                Interface::new(InterfaceBase {
                    site_id: self.site_id,
                    if_index: Some(entry.if_index),
                    if_descr: entry.if_descr.clone(),
                    if_type: Some(entry.if_type.unwrap_or(if_type::ETHERNET_CSMA_CD)),
                    mac_address: entry
                        .if_phys_address
                        .map(|m| MacEvidence::new(MacEvidenceValue(m), SNMP)),
                    native_vlan_id: membership
                        .and_then(|m| m.native_vlan)
                        .and_then(|number| self.vlans.get(&number).copied()),
                    vlan_ids: membership.map(|m| {
                        m.tagged_vlans
                            .iter()
                            .filter_map(|number| self.vlans.get(number).copied())
                            .collect()
                    }),
                    vlan_membership_source: SNMP,
                    fdb_macs: (!learned.is_empty())
                        .then(|| FdbMacs::new(FdbMacsValue(learned), SNMP)),
                    ..Default::default()
                })
            })
            .collect()
    }

    async fn linked_vlans(&self) -> Vec<Uuid> {
        self.services
            .vlan_service
            .vlan_ids_for_subnet(&self.subnet_id)
            .await
            .unwrap()
    }
}

/// The server's MAC is learned on an access port in VLAN 20, so its subnet is on VLAN 20, and on
/// no VLAN the switch's other ports carry.
#[tokio::test]
async fn a_host_learned_on_an_access_port_links_its_subnet_to_that_vlan() {
    let lab = Lab::new().await;
    lab.server("192.168.1.50", SERVER_MAC).await;

    lab.scan_switch(&[]).await;

    assert_eq!(lab.linked_vlans().await, vec![lab.vlans[&20]]);
}

/// The link lasts only as long as a port learns the host. Once the switch's next scan no longer
/// has the MAC, the subnet loses the VLAN.
#[tokio::test]
async fn the_link_drops_when_no_port_learns_the_host_any_more() {
    let lab = Lab::new().await;
    lab.server("192.168.1.50", SERVER_MAC).await;
    lab.scan_switch(&[]).await;
    assert_eq!(lab.linked_vlans().await, vec![lab.vlans[&20]]);

    lab.scan_switch(&[SERVER_MAC]).await;

    assert!(lab.linked_vlans().await.is_empty());
}
