//! Other devices' addresses held by a guest or a daemon host, moved off on the next scan, end to
//! end through `discover_host` on a testcontainers Postgres.
//!
//! The rules are pure and unit-tested beside them (`rows_a_guest_sheds`,
//! `stale_self_reported_rows`, `HostCandidate::mac_is_foreign`). These cover the write path: the
//! rows move with their ids, what is bound only to them goes with them, and the device's own next
//! payload finds them where they went. Fixtures are the snmp-test lab VM and the Mac daemon host
//! as the dev DB held them on 2026-10-22.

use std::net::IpAddr;

use mac_address::MacAddress;
use uuid::Uuid;

use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::bindings::r#impl::base::Binding;
use crate::server::hosts::r#impl::api::HostResponse;
use crate::server::hosts::r#impl::base::{Host, HostBase};
use crate::server::hosts::r#impl::virtualization::HostVirtualizationDiscriminants;
use crate::server::interfaces::r#impl::base::{Interface, InterfaceBase, InterfaceDataComplete};
use crate::server::ip_addresses::r#impl::base::{
    IPAddress, IPAddressBase, MacEvidence, MacEvidenceValue,
};
use crate::server::services::definitions::ServiceDefinitionRegistry;
use crate::server::services::r#impl::base::{Service, ServiceBase};
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::shared::attribution::AttributeSource;
use crate::server::shared::services::factory::ServiceFactory;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::{Storable, Storage};
use crate::server::shared::types::entities::EntitySource;
use crate::server::sites::r#impl::{Site, SiteBase};
use crate::server::subnets::r#impl::base::{
    Subnet, SubnetBase, SubnetCidr, SubnetCidrValue, SubnetName, SubnetTypeAttributed,
};
use crate::server::subnets::r#impl::types::SubnetType;

use super::{daemon, organization, test_services, user};

fn mac(s: &str) -> MacAddress {
    s.parse().unwrap()
}
const ETH0: &str = "02:e9:65:cc:7f:7b";
const MV_SNMP231: &str = "42:d3:d4:77:c4:0f";
const VMWARE_GUEST: &str = "00:0c:29:87:81:5d";

struct Lab {
    site_id: Uuid,
    lan: Uuid,
}

async fn lab(services: &ServiceFactory, org_id: Uuid) -> Lab {
    let site = services
        .site_service
        .create(
            Site::new(SiteBase::new(org_id)),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();
    let lan = services
        .subnet_service
        .create(
            Subnet::new(SubnetBase {
                name: SubnetName::unattributed("192.168.4.0/22".to_string()),
                site_id: site.id,
                cidr: SubnetCidr::new(
                    SubnetCidrValue("192.168.4.0/22".parse().unwrap()),
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
    Lab {
        site_id: site.id,
        lan,
    }
}

fn address(lab: &Lab, ip: &str, mac_address: &str, source: AttributeSource) -> IPAddress {
    IPAddress::new(IPAddressBase {
        site_id: lab.site_id,
        subnet_id: lab.lan,
        ip_address: ip.parse::<IpAddr>().unwrap(),
        mac_address: Some(MacEvidence::new(MacEvidenceValue(mac(mac_address)), source)),
        ip_address_source: source,
        ..Default::default()
    })
}

fn bound_service(lab: &Lab, definition: &str, to: &IPAddress) -> Service {
    Service::new(ServiceBase {
        name: definition.to_string(),
        site_id: lab.site_id,
        service_definition: ServiceDefinitionRegistry::find_by_id(definition)
            .unwrap_or_else(|| panic!("{definition} is registered")),
        bindings: vec![Binding::new_ip_address_serviceless(to.id)],
        source: EntitySource::Discovery,
        ..Default::default()
    })
}

fn interface(lab: &Lab, name: &str, mac_address: &str, source: AttributeSource) -> Interface {
    Interface::new(InterfaceBase {
        site_id: lab.site_id,
        if_name: Some(name.to_string()),
        mac_address: Some(MacEvidence::new(MacEvidenceValue(mac(mac_address)), source)),
        interface_source: source,
        ..Default::default()
    })
}

/// The `snmp-test` VM (107) as a guest of `kind`: the rules hold under every hypervisor.
fn snmp_test(kind: HostVirtualizationDiscriminants) -> HostBase {
    HostBase {
        virtualization_metadata: Some(kind.guest("snmp-test", "107")),
        ..Default::default()
    }
}

async fn discover(
    services: &ServiceFactory,
    lab: &Lab,
    host: HostBase,
    ip_addresses: Vec<IPAddress>,
    host_services: Vec<Service>,
    interfaces: Vec<Interface>,
) -> HostResponse {
    services
        .host_service
        .discover_host(
            Host::new(HostBase {
                site_id: lab.site_id,
                source: EntitySource::Discovery,
                ..host
            }),
            ip_addresses,
            vec![],
            host_services,
            interfaces,
            vec![],
            false,
            InterfaceDataComplete::none(),
            &crate::server::ports::r#impl::coverage::PortCoverage::default(),
            None,
            AuthenticatedEntity::System,
            None,
        )
        .await
        .expect("the submission persists")
        .host
}

async fn ips_of(services: &ServiceFactory, host_id: Uuid) -> Vec<IpAddr> {
    let mut ips: Vec<IpAddr> = services
        .ip_address_service
        .get_for_host(&host_id)
        .await
        .unwrap()
        .into_iter()
        .map(|a| a.base.ip_address)
        .collect();
    ips.sort();
    ips
}

async fn service_names(services: &ServiceFactory, host_id: Uuid) -> Vec<String> {
    let mut names: Vec<String> = services
        .service_service
        .get_all(StorableFilter::<Service>::new_from_host_ids(&[host_id]).live())
        .await
        .unwrap()
        .into_iter()
        .map(|s| s.base.name)
        .collect();
    names.sort();
    names
}

/// snmp-test carrying two other devices: the VMware guest at 6.1 with its Gateway service, and
/// switch-bulkdrop-01 at .231 with its SNMP service and its three-port ifTable. Its next hypervisor
/// report names eth0 and the `mv-snmp231` link. Each device's rows leave for a host of their own,
/// with what is bound only to them; the VM keeps its own address and its link; and .231's own
/// next payload finds .231's new host, not the VM.
#[tokio::test]
async fn a_guests_report_gives_other_devices_their_rows_back() {
    let (storage, services, _container) = test_services().await;
    for kind in HostVirtualizationDiscriminants::hypervisor_guests() {
        let org = organization();
        storage.organizations.create(&org).await.unwrap();
        let lab = lab(&services, org.id).await;

        let own = address(&lab, "192.168.4.21", ETH0, AttributeSource::ArpReply);
        let vmware = address(&lab, "192.168.6.1", VMWARE_GUEST, AttributeSource::ArpReply);
        let switch = address(&lab, "192.168.7.231", MV_SNMP231, AttributeSource::ArpReply);
        let snmp = AttributeSource::Probe(ClientProbe::Snmp);
        let vm = discover(
            &services,
            &lab,
            HostBase::default(),
            vec![own, vmware.clone(), switch.clone()],
            vec![
                bound_service(&lab, "Gateway", &vmware),
                bound_service(&lab, "SNMP", &switch),
            ],
            vec![
                interface(&lab, "Gi0/1", "00:1a:2b:00:5c:01", snmp),
                interface(&lab, "Gi0/2", "00:1a:2b:00:5c:02", snmp),
                interface(&lab, "Gi0/3", "00:1a:2b:00:5c:03", snmp),
            ],
        )
        .await;
        assert_eq!(
            ips_of(&services, vm.id).await.len(),
            3,
            "the stray state reproduces"
        );

        let reported = discover(
            &services,
            &lab,
            snmp_test(kind),
            vec![address(
                &lab,
                "192.168.4.21",
                ETH0,
                AttributeSource::HypervisorConfig,
            )],
            vec![],
            vec![interface(
                &lab,
                "mv-snmp231",
                MV_SNMP231,
                AttributeSource::GuestAgent,
            )],
        )
        .await;
        assert_eq!(reported.id, vm.id, "the report is the VM's");
        assert_eq!(
            ips_of(&services, vm.id).await,
            vec!["192.168.4.21".parse::<IpAddr>().unwrap()],
            "the VM keeps only its own address"
        );
        assert!(service_names(&services, vm.id).await.is_empty());
        let vm_interfaces: Vec<Option<String>> = services
            .interface_service
            .get_for_host(&vm.id)
            .await
            .unwrap()
            .into_iter()
            .map(|i| i.base.if_name)
            .collect();
        assert_eq!(
            vm_interfaces,
            vec![Some("mv-snmp231".to_string())],
            "the switch's ifTable leaves with its SNMP service; the VM's link stays"
        );

        let holder_of = |row: &IPAddress| {
            let row_id = row.id;
            let services = &services;
            async move {
                services
                    .ip_address_service
                    .get_by_id(&row_id)
                    .await
                    .unwrap()
                    .expect("the row moved with its id")
                    .base
                    .host_id
            }
        };
        let vmware_host = holder_of(&vmware).await;
        let switch_host = holder_of(&switch).await;
        assert_ne!(vmware_host, vm.id);
        assert_ne!(switch_host, vm.id);
        assert_ne!(vmware_host, switch_host, "one host per device");
        assert_eq!(service_names(&services, vmware_host).await, vec!["Gateway"]);
        assert_eq!(service_names(&services, switch_host).await, vec!["SNMP"]);

        let swept_again = discover(
            &services,
            &lab,
            HostBase::default(),
            vec![address(
                &lab,
                "192.168.7.231",
                MV_SNMP231,
                AttributeSource::ArpReply,
            )],
            vec![],
            vec![],
        )
        .await;
        assert_eq!(swept_again.id, switch_host, ".231 now finds its own host");
    }
}

/// A sim link's first ARP reply, with no stored address, finds nothing on the VM through the
/// link's interface row, so it becomes a host of its own.
#[tokio::test]
async fn a_links_first_arp_reply_never_lands_on_its_guest() {
    let (storage, services, _container) = test_services().await;
    for kind in HostVirtualizationDiscriminants::hypervisor_guests() {
        let org = organization();
        storage.organizations.create(&org).await.unwrap();
        let lab = lab(&services, org.id).await;

        let vm = discover(
            &services,
            &lab,
            snmp_test(kind),
            vec![address(
                &lab,
                "192.168.4.21",
                ETH0,
                AttributeSource::HypervisorConfig,
            )],
            vec![],
            vec![interface(
                &lab,
                "mv-snmp231",
                MV_SNMP231,
                AttributeSource::GuestAgent,
            )],
        )
        .await;
        let swept = discover(
            &services,
            &lab,
            HostBase::default(),
            vec![address(
                &lab,
                "192.168.7.231",
                MV_SNMP231,
                AttributeSource::ArpReply,
            )],
            vec![],
            vec![],
        )
        .await;
        assert_ne!(swept.id, vm.id);
        assert_eq!(
            ips_of(&services, vm.id).await,
            vec!["192.168.4.21".parse::<IpAddr>().unwrap()]
        );
    }
}

/// The Mac daemon host holding `bridge102`'s 192.168.4.1, with the router's services filed on it.
/// The router's ARP reply makes a host of its own, and the daemon's next report, which no longer
/// lists 4.1, retires the address and the services seen only there. The daemon's own service,
/// bound to its own address too, stays and loses only the 4.1 binding.
#[tokio::test]
async fn a_daemon_report_retires_an_address_it_no_longer_has() {
    let (storage, services, _container) = test_services().await;
    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let lab = lab(&services, org.id).await;
    let own = AttributeSource::DaemonSelfReport;

    let en0 = address(&lab, "192.168.5.93", "34:66:91:ec:6c:2f", own);
    let bridge102 = address(&lab, "192.168.4.1", "36:66:91:ce:e9:66", own);
    let mut daemon_service = bound_service(&lab, "Scanopy Daemon", &en0);
    daemon_service
        .base
        .bindings
        .push(Binding::new_ip_address_serviceless(bridge102.id));
    let mac_host = discover(
        &services,
        &lab,
        HostBase::default(),
        vec![en0, bridge102.clone()],
        vec![
            daemon_service,
            bound_service(&lab, "Eero Gateway", &bridge102),
        ],
        vec![],
    )
    .await;
    let owner = user(&org.id);
    storage.users.create(&owner).await.unwrap();
    let mut mac_daemon = daemon(&lab.site_id, &mac_host.id);
    mac_daemon.base.user_id = owner.id;
    storage.daemons.create(&mac_daemon).await.unwrap();

    let router = discover(
        &services,
        &lab,
        HostBase::default(),
        vec![address(
            &lab,
            "192.168.4.1",
            "c4:f1:74:2f:56:f2",
            AttributeSource::ArpReply,
        )],
        vec![],
        vec![],
    )
    .await;
    assert_ne!(
        router.id, mac_host.id,
        "the router is not the daemon's host"
    );

    let reported = discover(
        &services,
        &lab,
        HostBase::default(),
        vec![address(&lab, "192.168.5.93", "34:66:91:ec:6c:2f", own)],
        vec![],
        vec![],
    )
    .await;
    assert_eq!(reported.id, mac_host.id);
    assert_eq!(
        ips_of(&services, mac_host.id).await,
        vec!["192.168.5.93".parse::<IpAddr>().unwrap()]
    );
    assert_eq!(
        service_names(&services, mac_host.id).await,
        vec!["Scanopy Daemon"],
        "the router's service seen only at 4.1 is gone; the daemon's own stays"
    );
    let daemon_service = services
        .service_service
        .get_all(StorableFilter::<Service>::new_from_host_ids(&[mac_host.id]).live())
        .await
        .unwrap()
        .remove(0);
    assert_eq!(daemon_service.base.bindings.len(), 1);
    assert_eq!(
        ips_of(&services, router.id).await,
        vec!["192.168.4.1".parse::<IpAddr>().unwrap()],
        "the router keeps its own address"
    );
}
