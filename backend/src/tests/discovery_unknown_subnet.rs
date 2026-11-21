//! A daemon report naming ids the server no longer holds, or that belong to another site.
//!
//! A daemon learns a subnet's id from the server (the subnet POST, the loopback seeded at
//! registration) and names it in later host reports. A subnet deleted in between, or one on
//! another site, must cost that one address, not the host, and never reach a daemon as a raw
//! database error. These drive `DaemonService::process_discovery_entities`, which both daemon
//! modes pass through.

use std::net::IpAddr;

use uuid::Uuid;

use super::{organization, site, test_services};
use crate::daemon::runtime::state::{BufferedEntities, CreatedEntitiesPayload};
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    bindings::r#impl::base::Binding,
    credentials::r#impl::{
        base::{Credential, CredentialBase},
        types::{CredentialAssignment, CredentialType, SecretValue},
    },
    hosts::r#impl::{
        api::{ConflictBehavior, DiscoveryHostRequest},
        base::{Host, HostBase},
    },
    interfaces::r#impl::base::{Interface, InterfaceBase, InterfaceDataComplete},
    ip_addresses::r#impl::base::{IPAddress, IPAddressBase},
    ports::r#impl::coverage::PortCoverage,
    services::{
        definitions::ServiceDefinitionRegistry,
        r#impl::base::{Service, ServiceBase},
    },
    shared::{
        attribution::AttributeSource,
        events::types::DeletionReason,
        services::{factory::ServiceFactory, traits::CrudService},
        storage::{
            filter::StorableFilter,
            traits::{Storable, Storage},
        },
        types::{api::ApiError, entities::EntitySource},
    },
    subnets::r#impl::base::{SubnetCidr, SubnetCidrValue, SubnetName},
    vlans::r#impl::base::{Vlan, VlanBase, VlanName, VlanNameValue},
};

struct Lab {
    services: ServiceFactory,
    org_id: Uuid,
    site_id: Uuid,
    other_site_id: Uuid,
    lan_id: Uuid,
    _container: super::TestDb,
}

/// An organization with two seeded sites. The first holds `192.168.1.0/24`.
async fn lab() -> Lab {
    let (storage, services, container) = test_services().await;
    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let mut site_ids = Vec::new();
    for _ in 0..2 {
        let s = site(&org.id);
        storage.sites.create(&s).await.unwrap();
        services
            .org_setup_service
            .seed_site(s.id, AuthenticatedEntity::System)
            .await
            .unwrap();
        site_ids.push(s.id);
    }
    let lan_id = subnet_on(&services, site_ids[0], "192.168.1.0/24").await;
    Lab {
        services,
        org_id: org.id,
        site_id: site_ids[0],
        other_site_id: site_ids[1],
        lan_id,
        _container: container,
    }
}

async fn subnet_on(services: &ServiceFactory, site_id: Uuid, cidr: &str) -> Uuid {
    let mut subnet = super::subnet(&site_id);
    subnet.base.cidr = SubnetCidr::new(
        SubnetCidrValue(cidr.parse().unwrap()),
        AttributeSource::DaemonSelfReport,
    );
    subnet.base.name = SubnetName::unattributed(cidr.to_string());
    services
        .subnet_service
        .create(subnet, AuthenticatedEntity::System)
        .await
        .unwrap()
        .id
}

async fn vlan_on(lab: &Lab, site_id: Uuid, number: u16) -> Uuid {
    let vlan = Vlan::new(VlanBase {
        vlan_number: number,
        name: VlanName::new(
            VlanNameValue(format!("vlan{number}")),
            AttributeSource::DaemonSelfReport,
        ),
        site_id,
        organization_id: lab.org_id,
        ..Default::default()
    });
    lab.services
        .vlan_service
        .storage()
        .create(&vlan)
        .await
        .unwrap()
        .id
}

async fn credential_in(lab: &Lab, org_id: Uuid) -> Uuid {
    lab.services
        .credential_service
        .create(
            Credential::new(CredentialBase {
                organization_id: org_id,
                credential_type: CredentialType::SnmpCommunity {
                    community: SecretValue::Inline {
                        // Unique, so no seeded default refuses it as a duplicate.
                        value: secrecy::SecretString::from(Uuid::new_v4().to_string()),
                    },
                },
                ..Default::default()
            }),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap()
        .id
}

/// The loopback subnet a daemon was told about at registration, deleted since.
async fn deleted_loopback(lab: &Lab) -> Uuid {
    let id = subnet_on(&lab.services, lab.site_id, "127.0.0.0/8").await;
    lab.services
        .subnet_service
        .delete(
            &id,
            AuthenticatedEntity::System,
            DeletionReason::EntityRequested,
        )
        .await
        .unwrap();
    id
}

fn address(host: &Host, subnet_id: Uuid, ip: &str, position: i32) -> IPAddress {
    IPAddress::new(IPAddressBase {
        site_id: host.base.site_id,
        host_id: host.id,
        subnet_id,
        ip_address: ip.parse().unwrap(),
        position,
        ..Default::default()
    })
}

fn discovered_host(site_id: Uuid) -> Host {
    Host::new(HostBase {
        site_id,
        source: EntitySource::Discovery,
        ..Default::default()
    })
}

fn report(host: Host, ip_addresses: Vec<IPAddress>) -> DiscoveryHostRequest {
    DiscoveryHostRequest {
        host,
        ip_addresses,
        ports: vec![],
        services: vec![],
        interfaces: vec![],
        subnets: vec![],
        interfaces_complete: true,
        interface_data_complete: InterfaceDataComplete::default(),
        port_coverage: PortCoverage::default(),
        superseded_wire_shape: false,
    }
}

/// A report of one host at `lan_ip`, plus a loopback and a public address naming `stale`.
fn report_naming(lab: &Lab, lan_ip: &str, stale: Uuid) -> DiscoveryHostRequest {
    let host = discovered_host(lab.site_id);
    let ips = vec![
        address(&host, stale, "127.0.0.1", 0),
        address(&host, lab.lan_id, lan_ip, 1),
        address(&host, stale, "203.0.113.7", 2),
    ];
    report(host, ips)
}

async fn process(lab: &Lab, hosts: Vec<DiscoveryHostRequest>) -> CreatedEntitiesPayload {
    lab.services
        .daemon_service
        .process_discovery_entities(
            BufferedEntities {
                hosts,
                subnets: vec![],
                vlans: vec![],
            },
            AuthenticatedEntity::System,
        )
        .await
        .expect("the batch is processed")
}

async fn stored_ips(lab: &Lab, host_id: Uuid) -> Vec<IpAddr> {
    let mut ips: Vec<IpAddr> = lab
        .services
        .ip_address_service
        .get_for_host(&host_id)
        .await
        .unwrap()
        .into_iter()
        .map(|ip| ip.base.ip_address)
        .collect();
    ips.sort();
    ips
}

/// The host lands with its LAN address. The addresses naming `stale` are dropped, since no
/// subnet of the site may hold them.
async fn assert_lands_without_stale_addresses(lab: &Lab, stale: Uuid) {
    let created = process(lab, vec![report_naming(lab, "192.168.1.50", stale)]).await;

    assert!(
        created.first_host_error.is_none(),
        "{:?}",
        created.first_host_error
    );
    let [(_, host)] = created.hosts.as_slice() else {
        panic!("one host lands, got {}", created.hosts.len());
    };
    assert_eq!(
        stored_ips(lab, host.id).await,
        vec!["192.168.1.50".parse::<IpAddr>().unwrap()]
    );
}

#[tokio::test]
async fn an_address_naming_a_deleted_subnet_costs_only_that_address() {
    let lab = lab().await;
    let stale = deleted_loopback(&lab).await;
    assert_lands_without_stale_addresses(&lab, stale).await;
}

#[tokio::test]
async fn an_address_naming_another_sites_subnet_is_treated_as_a_deleted_one() {
    let lab = lab().await;
    let foreign = subnet_on(&lab.services, lab.other_site_id, "127.0.0.0/8").await;
    assert_lands_without_stale_addresses(&lab, foreign).await;
}

/// A ServerPoll batch: one host's stale id does not cost the hosts around it.
#[tokio::test]
async fn a_batch_with_one_stale_subnet_id_lands_every_host() {
    let lab = lab().await;
    let stale = deleted_loopback(&lab).await;
    let host_at = |ip: &str| {
        let host = discovered_host(lab.site_id);
        let ips = vec![address(&host, lab.lan_id, ip, 0)];
        report(host, ips)
    };

    let created = process(
        &lab,
        vec![
            host_at("192.168.1.10"),
            report_naming(&lab, "192.168.1.11", stale),
            host_at("192.168.1.12"),
        ],
    )
    .await;

    assert_eq!(created.hosts.len(), 3);
    assert!(created.first_host_error.is_none());
}

/// The API create path names the bad id instead of answering with the foreign-key failure.
#[tokio::test]
async fn an_api_create_naming_a_deleted_subnet_is_refused_as_invalid() {
    let lab = lab().await;
    let stale = deleted_loopback(&lab).await;
    let mut host = discovered_host(lab.site_id);
    host.base.source = EntitySource::Manual;
    let ips = vec![address(&host, stale, "127.0.0.1", 0)];

    let result = lab
        .services
        .host_service
        .create_with_children(
            host,
            ips,
            vec![],
            vec![],
            vec![],
            vec![],
            ConflictBehavior::Error,
            AuthenticatedEntity::System,
            None,
            true,
            InterfaceDataComplete::default(),
            &PortCoverage::default(),
        )
        .await;
    let Err(err) = result else {
        panic!("the stale subnet id is refused");
    };

    assert_eq!(
        ApiError::from(err).status,
        axum::http::StatusCode::BAD_REQUEST
    );
}

/// A foreign-key failure that does reach the database answers 400 without the database's text.
#[tokio::test]
async fn a_foreign_key_failure_does_not_leak_database_text() {
    let lab = lab().await;
    let host = lab
        .services
        .host_service
        .create(super::host(&lab.site_id), AuthenticatedEntity::System)
        .await
        .unwrap();

    let err = lab
        .services
        .ip_address_service
        .create(
            address(&host, Uuid::new_v4(), "192.168.1.70", 0),
            AuthenticatedEntity::System,
        )
        .await
        .expect_err("no subnet holds that id");

    let api = ApiError::from(err);
    assert_eq!(api.status, axum::http::StatusCode::BAD_REQUEST);
    assert!(!api.message.contains("violates"), "{}", api.message);
}

/// An address dropped from a report (here, one naming another site's row) leaves every other
/// address's service binding on that address's own row.
#[tokio::test]
async fn a_dropped_address_leaves_the_others_bindings_in_place() {
    let lab = lab().await;

    // A row on the other site, which the report names by id.
    let other_lan = subnet_on(&lab.services, lab.other_site_id, "10.9.0.0/24").await;
    let other_host = lab
        .services
        .host_service
        .create(super::host(&lab.other_site_id), AuthenticatedEntity::System)
        .await
        .unwrap();
    let foreign_row = lab
        .services
        .ip_address_service
        .create(
            address(&other_host, other_lan, "10.9.0.5", 0),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    // First scan stores the host with two addresses.
    let host = discovered_host(lab.site_id);
    let b = address(&host, lab.lan_id, "192.168.1.20", 0);
    let c = address(&host, lab.lan_id, "192.168.1.21", 1);
    let created = process(&lab, vec![report(host.clone(), vec![b.clone(), c.clone()])]).await;
    let (_, stored) = &created.hosts[0];
    let stored_id = |ip: &str| {
        stored
            .ip_addresses
            .iter()
            .find(|row| row.base.ip_address == ip.parse::<IpAddr>().unwrap())
            .unwrap()
            .id
    };
    let (b_id, c_id) = (stored_id("192.168.1.20"), stored_id("192.168.1.21"));

    // Second scan, with the server's ids, a foreign row first, and a service bound to B.
    let mut host = host;
    host.id = stored.id;
    let mut foreign = address(&host, lab.lan_id, "192.168.1.22", 0);
    foreign.id = foreign_row.id;
    let mut b = address(&host, lab.lan_id, "192.168.1.20", 1);
    b.id = b_id;
    let mut c = address(&host, lab.lan_id, "192.168.1.21", 2);
    c.id = c_id;
    let mut web = Service::new(ServiceBase {
        name: "Jenkins".to_string(),
        host_id: host.id,
        site_id: lab.site_id,
        service_definition: ServiceDefinitionRegistry::find_by_id("Jenkins").unwrap(),
        source: EntitySource::Discovery,
        ..Default::default()
    });
    web.base.bindings = vec![Binding::new_ip_address_serviceless(b_id)];
    let mut second = report(host.clone(), vec![foreign, b, c]);
    second.services = vec![web];
    let created = process(&lab, vec![second]).await;
    assert!(
        created.first_host_error.is_none(),
        "{:?}",
        created.first_host_error
    );

    let services = lab
        .services
        .service_service
        .get_all(StorableFilter::<Service>::new_from_host_ids(&[host.id]).live())
        .await
        .unwrap();
    let bound: Vec<Uuid> = services
        .iter()
        .flat_map(|s| s.base.bindings.iter().filter_map(|b| b.ip_address_id()))
        .collect();
    assert_eq!(bound, vec![b_id]);
}

/// A native VLAN id deleted since the VLAN batch, or on another site, is cleared; the interface
/// and the host still land.
#[tokio::test]
async fn an_interface_naming_an_unusable_native_vlan_keeps_the_interface() {
    let lab = lab().await;
    let kept = vlan_on(&lab, lab.site_id, 10).await;
    let deleted = vlan_on(&lab, lab.site_id, 20).await;
    lab.services
        .vlan_service
        .delete(
            &deleted,
            AuthenticatedEntity::System,
            DeletionReason::EntityRequested,
        )
        .await
        .unwrap();
    let foreign = vlan_on(&lab, lab.other_site_id, 30).await;

    let host = discovered_host(lab.site_id);
    let ips = vec![address(&host, lab.lan_id, "192.168.1.30", 0)];
    let mut request = report(host, ips);
    request.interfaces = [("eth0", kept), ("eth1", deleted), ("eth2", foreign)]
        .into_iter()
        .enumerate()
        .map(|(i, (name, vlan))| {
            Interface::new(InterfaceBase {
                site_id: lab.site_id,
                if_index: Some(i as i32 + 1),
                if_name: Some(name.to_string()),
                native_vlan_id: Some(vlan),
                ..Default::default()
            })
        })
        .collect();

    let created = process(&lab, vec![request]).await;
    assert!(
        created.first_host_error.is_none(),
        "{:?}",
        created.first_host_error
    );

    let mut native: Vec<(Option<String>, Option<Uuid>)> = lab
        .services
        .interface_service
        .get_for_host(&created.hosts[0].1.id)
        .await
        .unwrap()
        .into_iter()
        .map(|i| (i.base.if_name, i.base.native_vlan_id))
        .collect();
    native.sort();
    assert_eq!(
        native,
        vec![
            (Some("eth0".to_string()), Some(kept)),
            (Some("eth1".to_string()), None),
            (Some("eth2".to_string()), None),
        ]
    );
}

/// Only an assignment of a credential in the host's organization is stored. A deleted credential
/// and another organization's are dropped alike, and do not take the valid one with them.
#[tokio::test]
async fn credential_assignments_keep_only_this_organizations_credentials() {
    let lab = lab().await;
    let valid = credential_in(&lab, lab.org_id).await;
    let deleted = credential_in(&lab, lab.org_id).await;
    lab.services
        .credential_service
        .delete(
            &deleted,
            AuthenticatedEntity::System,
            DeletionReason::EntityRequested,
        )
        .await
        .unwrap();
    let other_org = organization();
    lab.services
        .organization_service
        .storage()
        .create(&other_org)
        .await
        .unwrap();
    let foreign = credential_in(&lab, other_org.id).await;

    let mut host = discovered_host(lab.site_id);
    host.base.credential_assignments = [valid, deleted, foreign]
        .into_iter()
        .map(|credential_id| CredentialAssignment {
            credential_id,
            ip_address_ids: None,
        })
        .collect();
    let ips = vec![address(&host, lab.lan_id, "192.168.1.40", 0)];
    let created = process(&lab, vec![report(host, ips)]).await;
    assert!(
        created.first_host_error.is_none(),
        "{:?}",
        created.first_host_error
    );

    let stored: Vec<Uuid> = lab
        .services
        .credential_service
        .get_credential_assignments_for_host(&created.hosts[0].1.id)
        .await
        .unwrap()
        .into_iter()
        .map(|a| a.credential_id)
        .collect();
    assert_eq!(stored, vec![valid]);
}
