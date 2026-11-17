//! Who owns a port after discovery: Unclaimed Open Ports, identified services and containers.
//!
//! Each test replays scans of one host through `discover_host` on a testcontainers Postgres and
//! checks which service ends up holding each port. These pin behaviour that held before GH #784's
//! changes to the binding-conflict chain in `hosts/service/create.rs`, so a change to that chain
//! cannot move a port nobody asked to move.

use std::net::IpAddr;

use mac_address::MacAddress;
use uuid::Uuid;

use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::bindings::r#impl::base::Binding;
use crate::server::hosts::r#impl::base::{Host, HostBase};
use crate::server::interfaces::r#impl::base::InterfaceDataComplete;
use crate::server::ip_addresses::r#impl::base::{
    IPAddress, IPAddressBase, MacEvidence, MacEvidenceValue,
};
use crate::server::ports::r#impl::base::{Port, PortBase, PortType};
use crate::server::services::definitions::ServiceDefinitionRegistry;
use crate::server::services::r#impl::base::{Service, ServiceBase};
use crate::server::services::r#impl::virtualization::{
    DockerVirtualization, ServiceVirtualization,
};
use crate::server::shared::attribution::AttributeSource;
use crate::server::shared::services::factory::ServiceFactory;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::{Storable, Storage};
use crate::server::shared::types::entities::EntitySource;
use crate::server::sites::r#impl::{Site, SiteBase};
use crate::server::subnets::r#impl::base::{Subnet, SubnetBase, SubnetCidr, SubnetCidrValue};
use crate::server::subnets::r#impl::types::SubnetType;

use super::{organization, test_services};
use crate::server::subnets::r#impl::base::{SubnetName, SubnetTypeAttributed};

const LAN_IP: &str = "192.168.4.58";

/// One host on a LAN with a Docker bridge, scanned repeatedly under the same host id.
struct Lab<'a> {
    services: &'a ServiceFactory,
    site_id: Uuid,
    lan: Uuid,
    bridge: Uuid,
    host_id: Uuid,
}

impl<'a> Lab<'a> {
    async fn new(
        services: &'a ServiceFactory,
        storage: &crate::server::shared::storage::factory::StorageFactory,
    ) -> Self {
        let org = organization();
        storage.organizations.create(&org).await.unwrap();
        let site_id = services
            .site_service
            .create(
                Site::new(SiteBase::new(org.id)),
                AuthenticatedEntity::System,
            )
            .await
            .unwrap()
            .id;
        let lan = Self::subnet(services, site_id, "192.168.4.0/22", SubnetType::Lan).await;
        let bridge =
            Self::subnet(services, site_id, "172.17.0.0/16", SubnetType::DockerBridge).await;
        Self {
            services,
            site_id,
            lan,
            bridge,
            host_id: Uuid::new_v4(),
        }
    }

    async fn subnet(
        services: &ServiceFactory,
        site_id: Uuid,
        cidr: &str,
        subnet_type: SubnetType,
    ) -> Uuid {
        services
            .subnet_service
            .create(
                Subnet::new(SubnetBase {
                    name: SubnetName::unattributed(cidr.to_string()),
                    site_id,
                    cidr: SubnetCidr::new(
                        SubnetCidrValue(cidr.parse().unwrap()),
                        AttributeSource::DaemonSelfReport,
                    ),
                    subnet_type: SubnetTypeAttributed::unattributed(subnet_type),
                    source: EntitySource::Discovery,
                    ..Default::default()
                }),
                AuthenticatedEntity::System,
            )
            .await
            .unwrap()
            .id
    }

    fn lan_ip(&self) -> IPAddress {
        let mac: MacAddress = "02:1c:65:32:e5:dd".parse().unwrap();
        IPAddress::new(IPAddressBase {
            site_id: self.site_id,
            subnet_id: self.lan,
            ip_address: LAN_IP.parse().unwrap(),
            mac_address: Some(MacEvidence::new(
                MacEvidenceValue(mac),
                AttributeSource::ArpReply,
            )),
            ..Default::default()
        })
    }

    fn bridge_ip(&self, ip: &str) -> IPAddress {
        IPAddress::new(IPAddressBase {
            site_id: self.site_id,
            subnet_id: self.bridge,
            ip_address: ip.parse().unwrap(),
            ..Default::default()
        })
    }

    fn tcp(&self, number: u16) -> Port {
        Port::new(PortBase::new(
            Uuid::nil(),
            self.site_id,
            PortType::new_tcp(number),
        ))
    }

    fn service(&self, definition: &str, bindings: Vec<(&Port, &IPAddress)>) -> Service {
        let mut service = Service::new(ServiceBase {
            name: definition.to_string(),
            site_id: self.site_id,
            service_definition: ServiceDefinitionRegistry::find_by_id(definition)
                .unwrap_or_else(|| panic!("{definition} is registered")),
            source: EntitySource::Discovery,
            ..Default::default()
        });
        service.base.bindings = bindings
            .into_iter()
            .map(|(port, ip)| Binding::new_port_serviceless(port.id, Some(ip.id)))
            .collect();
        service
    }

    /// The Docker runtime, bound to the host's LAN address.
    fn runtime(&self, lan: &IPAddress) -> Service {
        let mut runtime = self.service("Docker", vec![]);
        runtime.base.bindings = vec![Binding::new_ip_address_serviceless(lan.id)];
        runtime
    }

    /// A container the daemon could not identify, sent as a generic Docker Container.
    fn container(
        &self,
        name: &str,
        runtime: &Service,
        bindings: Vec<(&Port, &IPAddress)>,
    ) -> Service {
        let mut container = self.service("Docker Container", bindings);
        container.base.name = name.to_string();
        container.base.virtualization_service_id = Some(runtime.id);
        container.base.virtualization_metadata =
            Some(ServiceVirtualization::Docker(DockerVirtualization {
                state: Default::default(),
                container_name: Some(name.to_string()),
                container_id: Some(format!("{name}-id")),
                compose_project: None,
            }));
        container
    }

    async fn scan(&self, ip_addresses: Vec<IPAddress>, ports: Vec<Port>, services: Vec<Service>) {
        let mut host = Host::new(HostBase {
            site_id: self.site_id,
            source: EntitySource::Discovery,
            ..Default::default()
        });
        host.id = self.host_id;
        self.services
            .host_service
            .discover_host(
                host,
                ip_addresses,
                ports,
                services,
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
            .expect("the scan persists");
    }

    async fn stored(&self) -> Vec<Service> {
        self.services
            .service_service
            .get_all(StorableFilter::<Service>::new_from_site_ids(&[self.site_id]).live())
            .await
            .unwrap()
    }

    /// The live service with this name, if any.
    async fn named(&self, name: &str) -> Option<Service> {
        self.stored()
            .await
            .into_iter()
            .find(|s| s.base.name == name)
    }

    /// The (port, address) pairs a service binds, sorted.
    async fn ports_of(&self, service: &Service) -> Vec<(u16, IpAddr)> {
        let ports = self
            .services
            .port_service
            .get_for_host(&self.host_id)
            .await
            .unwrap();
        let ips = self
            .services
            .ip_address_service
            .get_for_host(&self.host_id)
            .await
            .unwrap();
        let mut pairs: Vec<(u16, IpAddr)> = service
            .base
            .bindings
            .iter()
            .filter_map(|b| {
                let port = ports.iter().find(|p| Some(p.id) == b.port_id())?;
                let ip = ips.iter().find(|i| Some(i.id) == b.ip_address_id())?;
                Some((port.base.port_type.number(), ip.base.ip_address))
            })
            .collect();
        pairs.sort();
        pairs
    }

    /// The ports the service with this name binds, or `None` when it is not stored.
    async fn ports_named(&self, name: &str) -> Option<Vec<(u16, IpAddr)>> {
        let service = self.named(name).await?;
        Some(self.ports_of(&service).await)
    }
}

fn at_lan(number: u16) -> (u16, IpAddr) {
    (number, LAN_IP.parse().unwrap())
}

/// An identified service found on a later scan takes its port from Unclaimed Open Ports, which
/// keeps the ports nobody claimed.
#[tokio::test]
async fn a_new_service_takes_one_port_and_open_ports_keeps_the_rest() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let (lan, p22, p3000) = (lab.lan_ip(), lab.tcp(22), lab.tcp(3000));
    let open_ports = lab.service("Unclaimed Open Ports", vec![(&p22, &lan), (&p3000, &lan)]);
    lab.scan(vec![lan], vec![p22, p3000], vec![open_ports])
        .await;

    let (lan, p3000) = (lab.lan_ip(), lab.tcp(3000));
    let grafana = lab.service("Grafana", vec![(&p3000, &lan)]);
    lab.scan(vec![lan], vec![p3000], vec![grafana]).await;

    assert_eq!(lab.ports_named("Grafana").await, Some(vec![at_lan(3000)]));
    assert_eq!(
        lab.ports_named("Unclaimed Open Ports").await,
        Some(vec![at_lan(22)])
    );
}

/// Taking Unclaimed Open Ports' only port leaves nothing for it to hold, so it goes.
#[tokio::test]
async fn a_new_service_taking_the_only_port_deletes_open_ports() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let (lan, p3000) = (lab.lan_ip(), lab.tcp(3000));
    let open_ports = lab.service("Unclaimed Open Ports", vec![(&p3000, &lan)]);
    lab.scan(vec![lan], vec![p3000], vec![open_ports]).await;

    let (lan, p3000) = (lab.lan_ip(), lab.tcp(3000));
    let grafana = lab.service("Grafana", vec![(&p3000, &lan)]);
    lab.scan(vec![lan], vec![p3000], vec![grafana]).await;

    assert_eq!(lab.ports_named("Grafana").await, Some(vec![at_lan(3000)]));
    assert_eq!(lab.ports_named("Unclaimed Open Ports").await, None);
}

/// A stored service that a rescan reports on another identified service's port leaves that port
/// with its owner and keeps its own.
#[tokio::test]
async fn a_rediscovered_service_leaves_another_services_port_alone() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let (lan, p3000, p3001) = (lab.lan_ip(), lab.tcp(3000), lab.tcp(3001));
    let grafana = lab.service("Grafana", vec![(&p3000, &lan)]);
    let gitea = lab.service("Gitea", vec![(&p3001, &lan)]);
    lab.scan(vec![lan], vec![p3000, p3001], vec![grafana, gitea])
        .await;

    let (lan, p3000, p3001) = (lab.lan_ip(), lab.tcp(3000), lab.tcp(3001));
    let gitea = lab.service("Gitea", vec![(&p3001, &lan), (&p3000, &lan)]);
    lab.scan(vec![lan], vec![p3000, p3001], vec![gitea]).await;

    assert_eq!(lab.ports_named("Grafana").await, Some(vec![at_lan(3000)]));
    assert_eq!(lab.ports_named("Gitea").await, Some(vec![at_lan(3001)]));
}

/// Rescanning a service with nothing in conflict leaves Unclaimed Open Ports as it was.
#[tokio::test]
async fn a_rescan_with_no_conflict_leaves_open_ports_unchanged() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let (lan, p22, p3000) = (lab.lan_ip(), lab.tcp(22), lab.tcp(3000));
    let grafana = lab.service("Grafana", vec![(&p3000, &lan)]);
    let open_ports = lab.service("Unclaimed Open Ports", vec![(&p22, &lan)]);
    lab.scan(vec![lan], vec![p22, p3000], vec![grafana, open_ports])
        .await;

    let (lan, p3000) = (lab.lan_ip(), lab.tcp(3000));
    let grafana = lab.service("Grafana", vec![(&p3000, &lan)]);
    lab.scan(vec![lan], vec![p3000], vec![grafana]).await;

    assert_eq!(lab.ports_named("Grafana").await, Some(vec![at_lan(3000)]));
    assert_eq!(
        lab.ports_named("Unclaimed Open Ports").await,
        Some(vec![at_lan(22)])
    );
}

/// A later scan that reports an owned port as unclaimed does not take it from its owner.
#[tokio::test]
async fn a_later_scan_reporting_a_port_unclaimed_does_not_take_it_back() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let (lan, p22, p3000) = (lab.lan_ip(), lab.tcp(22), lab.tcp(3000));
    let grafana = lab.service("Grafana", vec![(&p3000, &lan)]);
    let open_ports = lab.service("Unclaimed Open Ports", vec![(&p22, &lan)]);
    lab.scan(vec![lan], vec![p22, p3000], vec![grafana, open_ports])
        .await;

    let (lan, p22, p3000) = (lab.lan_ip(), lab.tcp(22), lab.tcp(3000));
    let open_ports = lab.service("Unclaimed Open Ports", vec![(&p22, &lan), (&p3000, &lan)]);
    lab.scan(vec![lan], vec![p22, p3000], vec![open_ports])
        .await;

    assert_eq!(lab.ports_named("Grafana").await, Some(vec![at_lan(3000)]));
    assert_eq!(
        lab.ports_named("Unclaimed Open Ports").await,
        Some(vec![at_lan(22)])
    );
}

/// An unidentified container on an identified service's port links that service to Docker and
/// is not stored itself. The service keeps its port.
#[tokio::test]
async fn a_container_on_an_identified_services_port_enriches_it() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let (lan, p3000) = (lab.lan_ip(), lab.tcp(3000));
    let grafana = lab.service("Grafana", vec![(&p3000, &lan)]);
    lab.scan(vec![lan], vec![p3000], vec![grafana]).await;

    let (lan, p3000) = (lab.lan_ip(), lab.tcp(3000));
    let runtime = lab.runtime(&lan);
    let container = lab.container("grafana-oss", &runtime, vec![(&p3000, &lan)]);
    lab.scan(vec![lan], vec![p3000], vec![runtime, container])
        .await;

    let runtime = lab.named("Docker").await.expect("runtime stored");
    let grafana = lab.named("Grafana").await.expect("grafana stored");
    assert_eq!(grafana.base.virtualization_service_id, Some(runtime.id));
    assert!(lab.ports_of(&grafana).await.contains(&at_lan(3000)));
    assert!(lab.named("grafana-oss").await.is_none());
}

/// An unidentified container on a port matched by port alone (PostgreSQL on 5432) is not stored
/// as a second service, the match keeps its port, and none of the container's bindings land in
/// Unclaimed Open Ports.
#[tokio::test]
async fn a_container_on_a_port_only_match_is_not_stored_and_adds_nothing_to_open_ports() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let (lan, p5432) = (lab.lan_ip(), lab.tcp(5432));
    let postgres = lab.service("PostgreSQL", vec![(&p5432, &lan)]);
    lab.scan(vec![lan], vec![p5432], vec![postgres]).await;

    let (lan, bridge, p5432) = (lab.lan_ip(), lab.bridge_ip("172.17.0.3"), lab.tcp(5432));
    let runtime = lab.runtime(&lan);
    let container = lab.container(
        "scanopy-postgres",
        &runtime,
        vec![(&p5432, &lan), (&p5432, &bridge)],
    );
    lab.scan(vec![lan, bridge], vec![p5432], vec![runtime, container])
        .await;

    assert!(lab.named("scanopy-postgres").await.is_none());
    assert!(
        lab.ports_named("PostgreSQL")
            .await
            .expect("postgres stored")
            .contains(&at_lan(5432))
    );
    assert_eq!(lab.ports_named("Unclaimed Open Ports").await, None);
}

/// An unidentified container on a port another container holds is not stored, and none of its
/// bindings land in Unclaimed Open Ports.
#[tokio::test]
async fn a_container_conflicting_with_another_container_adds_nothing_to_open_ports() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let (lan, p8080) = (lab.lan_ip(), lab.tcp(8080));
    let runtime = lab.runtime(&lan);
    let first = lab.container("first", &runtime, vec![(&p8080, &lan)]);
    lab.scan(vec![lan], vec![p8080], vec![runtime, first]).await;

    let (lan, bridge) = (lab.lan_ip(), lab.bridge_ip("172.17.0.9"));
    let (p8080, p9090) = (lab.tcp(8080), lab.tcp(9090));
    let runtime = lab.runtime(&lan);
    let second = lab.container("second", &runtime, vec![(&p8080, &lan), (&p9090, &bridge)]);
    lab.scan(vec![lan, bridge], vec![p8080, p9090], vec![runtime, second])
        .await;

    assert_eq!(lab.ports_named("first").await, Some(vec![at_lan(8080)]));
    assert!(lab.named("second").await.is_none());
    assert_eq!(lab.ports_named("Unclaimed Open Ports").await, None);
}

/// A container conflicting with an identified service on one port and with Unclaimed Open Ports
/// on another links the identified service and is not stored. Unclaimed Open Ports keeps its port.
#[tokio::test]
async fn a_mixed_conflict_enriches_the_identified_service_and_keeps_open_ports() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let (lan, p3000, p8083) = (lab.lan_ip(), lab.tcp(3000), lab.tcp(8083));
    let grafana = lab.service("Grafana", vec![(&p3000, &lan)]);
    let open_ports = lab.service("Unclaimed Open Ports", vec![(&p8083, &lan)]);
    lab.scan(vec![lan], vec![p3000, p8083], vec![grafana, open_ports])
        .await;

    let (lan, p3000, p8083) = (lab.lan_ip(), lab.tcp(3000), lab.tcp(8083));
    let runtime = lab.runtime(&lan);
    let container = lab.container(
        "grafana-oss",
        &runtime,
        vec![(&p3000, &lan), (&p8083, &lan)],
    );
    lab.scan(vec![lan], vec![p3000, p8083], vec![runtime, container])
        .await;

    let runtime = lab.named("Docker").await.expect("runtime stored");
    let grafana = lab.named("Grafana").await.expect("grafana stored");
    assert_eq!(grafana.base.virtualization_service_id, Some(runtime.id));
    assert!(lab.named("grafana-oss").await.is_none());
    assert_eq!(
        lab.ports_named("Unclaimed Open Ports").await,
        Some(vec![at_lan(8083)])
    );
}
