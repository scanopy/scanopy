//! Coverage for the host discovery write path — `HostService::create_with_children`.
//!
//! This path had no direct tests. Every integration test submits a host with four empty child
//! vectors, `CreateHostRequest` has no `subnets` field at all, and all 44 compat fixtures send
//! `subnets: []` — so the subnet stage was unreachable from any assertion, and the foreign key
//! break in GH #650 shipped undetected.
//!
//! These drive `discover_host`, the real production entry and the only one that reaches subnets,
//! against a testcontainers Postgres with the migrations applied. They run under
//! `cargo test --lib` in seconds; the docker-compose integration suite takes tens of minutes and
//! asserts almost nothing about persisted rows.

use crate::server::shared::attribution::AttributeSource;
use crate::server::subnets::r#impl::base::{SubnetCidr, SubnetCidrValue};
use std::net::{IpAddr, Ipv4Addr};

use cidr::IpCidr;
use uuid::Uuid;

use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::hosts::r#impl::api::HostResponse;
use crate::server::hosts::r#impl::base::{Host, HostBase};
use crate::server::interfaces::r#impl::base::InterfaceDataComplete;
use crate::server::ip_addresses::r#impl::base::{IPAddress, IPAddressBase};
use crate::server::networks::r#impl::{Network, NetworkBase};
use crate::server::ports::r#impl::base::{Port, PortBase, PortType};
use crate::server::services::definitions::ServiceDefinitionRegistry;
use crate::server::services::r#impl::base::{Service, ServiceBase};
use crate::server::shared::services::factory::ServiceFactory;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::{Storable, Storage};
use crate::server::shared::types::entities::EntitySource;
use crate::server::subnets::r#impl::base::{Subnet, SubnetBase};
use crate::server::subnets::r#impl::types::SubnetType;

use super::{organization, test_services};
use crate::server::hosts::r#impl::name::{HostName, HostNameSources};

const BRIDGE_CIDR: &str = "172.28.0.0/16";
const LAN_CIDR: &str = "192.168.1.0/24";

/// One host as the daemon submits it after a container scan: a LAN subnet, a Docker bridge
/// subnet owned by the runtime service, the runtime service itself, and a container service
/// running under that runtime.
///
/// Every id here is daemon-minted, exactly as it arrives over the wire — that is the whole point.
/// `Uuid::new_v4()` per scan is what makes the owner reference unresolvable at insert time.
struct Submission {
    host: Host,
    ip_addresses: Vec<IPAddress>,
    ports: Vec<Port>,
    services: Vec<Service>,
    subnets: Vec<Subnet>,
}

impl Submission {
    fn container_host(network_id: Uuid) -> Self {
        let host = Host::new(HostBase {
            name: HostName::manual("docker-host".to_string()),
            hostname: Some(crate::server::shared::attribution::Attributed::new(
                crate::server::hosts::r#impl::attributes::HostHostnameValue(
                    "docker-host.local".to_string(),
                ),
                crate::server::shared::attribution::AttributeSource::ReverseDns,
            )),
            network_id,
            source: EntitySource::Discovery,
            ..Default::default()
        });

        let lan = Subnet::new(SubnetBase {
            name: "lan".to_string(),
            network_id,
            cidr: SubnetCidr::new(
                SubnetCidrValue(LAN_CIDR.parse().unwrap()),
                AttributeSource::DaemonSelfReport,
            ),
            subnet_type: SubnetType::Lan,
            source: EntitySource::Discovery,
            ..Default::default()
        });

        let runtime = Service::new(ServiceBase {
            name: "Docker".to_string(),
            host_id: host.id,
            network_id,
            service_definition: ServiceDefinitionRegistry::find_by_id("Docker")
                .expect("Docker runtime definition is registered"),
            source: EntitySource::Discovery,
            ..Default::default()
        });

        // The bridge names the runtime service that owns it. On a first scan that id exists
        // only in daemon memory.
        let bridge = Subnet::new(SubnetBase {
            name: "sct-net-a".to_string(),
            network_id,
            cidr: SubnetCidr::new(
                SubnetCidrValue(BRIDGE_CIDR.parse().unwrap()),
                AttributeSource::DaemonSelfReport,
            ),
            subnet_type: SubnetType::DockerBridge,
            virtualization_service_id: Some(runtime.id),
            source: EntitySource::Discovery,
            ..Default::default()
        });

        let host_ip = IPAddress::new(IPAddressBase {
            network_id,
            subnet_id: lan.id,
            ip_address: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 50)),
            mac_address: None,
            position: 0,
            name: Some("eth0".to_string()),
            host_id: host.id,
        });

        let container_ip = IPAddress::new(IPAddressBase {
            network_id,
            subnet_id: bridge.id,
            ip_address: IpAddr::V4(Ipv4Addr::new(172, 28, 0, 2)),
            mac_address: None,
            position: 1,
            name: Some("eth0".to_string()),
            host_id: host.id,
        });

        let port = Port::new(PortBase {
            port_type: PortType::Http,
            host_id: host.id,
            network_id,
        });

        // A container running under that runtime. It names the runtime as its owner, exactly as
        // a bridge subnet does — so it exercises `services_virtualization_service_id_fkey`,
        // which has the same shape as the subnet one but is reached from the main service loop.
        let container = Service::new(ServiceBase {
            name: "sct-web-nginx".to_string(),
            host_id: host.id,
            network_id,
            service_definition: ServiceDefinitionRegistry::find_by_id("Nginx")
                .or_else(|| ServiceDefinitionRegistry::find_by_id("Docker Container"))
                .expect("a container-shaped definition is registered"),
            virtualization_service_id: Some(runtime.id),
            source: EntitySource::Discovery,
            ..Default::default()
        });

        Self {
            host,
            ip_addresses: vec![host_ip, container_ip],
            ports: vec![port],
            services: vec![runtime, container],
            subnets: vec![lan, bridge],
        }
    }

    fn bridge_cidr(&self) -> IpCidr {
        BRIDGE_CIDR.parse().unwrap()
    }

    /// A second scan of the same host, as a real daemon would submit it.
    ///
    /// The daemon learns server-side **host and subnet** ids from `CreatedEntitiesPayload` and
    /// reuses them, so those are stable across scans — host matching depends on it, since
    /// `ip_addresses_share_address` compares IP *and* `subnet_id`.
    ///
    /// **Service** ids are absent from that payload, so the daemon cannot learn them and mints a
    /// fresh UUID for the runtime service every scan. That asymmetry is the whole of GH #650:
    /// the bridge subnet's owner reference churns while the service it names does not.
    fn rescan(&mut self, host_id: Uuid, lan_id: Uuid, bridge_id: Uuid) {
        self.host.id = host_id;
        self.subnets[0].id = lan_id;
        self.subnets[1].id = bridge_id;
        self.ip_addresses[0].base.subnet_id = lan_id;
        self.ip_addresses[1].base.subnet_id = bridge_id;
        for ip in &mut self.ip_addresses {
            ip.base.host_id = host_id;
        }
        for port in &mut self.ports {
            port.base.host_id = host_id;
        }
        for svc in &mut self.services {
            svc.base.host_id = host_id;
        }
        // The runtime service keeps its freshly minted id, and the bridge keeps naming it.
        self.subnets[1].base.virtualization_service_id = Some(self.services[0].id);
    }
}

/// Seed an organization and a network, and hand back the wired-up services.
///
/// The container is returned so the caller keeps it alive for the test's duration — dropping it
/// tears down Postgres mid-test.
macro_rules! harness {
    ($services:ident, $network_id:ident, $container:ident) => {
        let (storage, $services, $container) = test_services().await;

        let org = organization();
        storage.organizations.create(&org).await.unwrap();

        let network = $services
            .network_service
            .create(
                Network::new(NetworkBase::new(org.id)),
                AuthenticatedEntity::System,
            )
            .await
            .unwrap();
        let $network_id = network.id;
    };
}

async fn submit(services: &ServiceFactory, s: Submission) -> anyhow::Result<HostResponse> {
    services
        .host_service
        .discover_host(
            s.host,
            s.ip_addresses,
            s.ports,
            s.services,
            vec![],
            s.subnets,
            true,
            InterfaceDataComplete::default(),
            None,
            AuthenticatedEntity::System,
            None,
        )
        .await
}

/// GH #650, as a test.
///
/// The daemon stamps a bridge subnet with the runtime service's pending id, but subnets are
/// written before services, so the reference is unresolvable at INSERT. Red before the reorder
/// with `subnets_virtualization_service_id_fkey`; green after, with the subnet pointing at the
/// persisted runtime service.
#[tokio::test]
async fn bridge_subnet_owner_resolves_on_first_scan() {
    harness!(services, network_id, _container);

    let submission = Submission::container_host(network_id);
    let bridge_cidr = submission.bridge_cidr();

    let response = submit(&services, submission)
        .await
        .expect("a first container scan must persist");

    let runtime = response
        .services
        .iter()
        .find(|s| s.base.service_definition.name() == "Docker")
        .expect("the Docker runtime service is persisted");

    let bridge = services
        .subnet_service
        .get_all(StorableFilter::<Subnet>::new_from_network_ids(&[network_id]).live())
        .await
        .unwrap()
        .into_iter()
        .find(|s| *s.base.cidr == bridge_cidr)
        .expect("the bridge subnet is persisted");

    assert_eq!(
        bridge.base.virtualization_service_id,
        Some(runtime.id),
        "the bridge subnet must point at the runtime service that owns it — an unowned bridge \
         deduplicates on CIDR alone and merges with other hosts' bridges"
    );
}

/// The duplicate-subnet half of GH #650: 353 rows for 70 distinct CIDRs.
///
/// A rescan mints fresh pending ids for everything. The bridge must resolve to the same
/// persisted subnet rather than inserting a second row for the same CIDR.
#[tokio::test]
async fn second_scan_reuses_the_one_bridge_subnet() {
    harness!(services, network_id, _container);

    let first = Submission::container_host(network_id);
    let bridge_cidr = first.bridge_cidr();
    let response = submit(&services, first).await.expect("first scan persists");

    let persisted = services
        .subnet_service
        .get_all(StorableFilter::<Subnet>::new_from_network_ids(&[network_id]).live())
        .await
        .unwrap();
    let lan_id = persisted
        .iter()
        .find(|s| s.base.subnet_type == SubnetType::Lan)
        .expect("lan subnet persisted")
        .id;
    let bridge_id = persisted
        .iter()
        .find(|s| *s.base.cidr == bridge_cidr)
        .expect("bridge subnet persisted")
        .id;

    // Rescan the way a real daemon does: stable host and subnet ids, a fresh service id.
    let mut second = Submission::container_host(network_id);
    second.rescan(response.id, lan_id, bridge_id);
    submit(&services, second).await.expect("rescan persists");

    let bridges: Vec<Subnet> = services
        .subnet_service
        .get_all(StorableFilter::<Subnet>::new_from_network_ids(&[network_id]).live())
        .await
        .unwrap()
        .into_iter()
        .filter(|s| *s.base.cidr == bridge_cidr)
        .collect();

    assert_eq!(
        bridges.len(),
        1,
        "a rescan must reuse the bridge subnet, not add a row per scan (GH #650)"
    );
    assert!(
        bridges[0].base.virtualization_service_id.is_some(),
        "the surviving bridge must still name its owning runtime service"
    );
}

/// A bridge naming a service that is in no submission must degrade that one subnet, not abort
/// the whole host. Before the guard this was an FK violation that lost every entity for the host.
#[tokio::test]
async fn unknown_bridge_owner_is_nulled_not_fatal() {
    harness!(services, network_id, _container);

    let mut submission = Submission::container_host(network_id);
    let bridge_cidr = submission.bridge_cidr();
    // Point the bridge at a service nobody will create.
    submission.subnets[1].base.virtualization_service_id = Some(Uuid::new_v4());

    submit(&services, submission)
        .await
        .expect("an unresolvable bridge owner must not fail the host");

    let bridge = services
        .subnet_service
        .get_all(StorableFilter::<Subnet>::new_from_network_ids(&[network_id]).live())
        .await
        .unwrap()
        .into_iter()
        .find(|s| *s.base.cidr == bridge_cidr)
        .expect("the bridge subnet is still persisted");

    assert_eq!(
        bridge.base.virtualization_service_id, None,
        "an unresolvable owner degrades to NULL rather than aborting the host"
    );
}

/// The property the whole two-phase split rests on: `upsert_service` merges bindings additively
/// and never clears them, so a row-phase call with no bindings leaves a persisted service's
/// bindings intact.
#[tokio::test]
async fn row_phase_upsert_does_not_drop_existing_bindings() {
    harness!(services, network_id, _container);

    let first = Submission::container_host(network_id);
    let response = submit(&services, first).await.expect("first scan persists");

    let runtime_before = response
        .services
        .iter()
        .find(|s| s.base.service_definition.name() == "Docker")
        .expect("runtime service persisted")
        .clone();

    // Re-submit the same runtime service carrying NO bindings, which is what the row phase does.
    let existing = services
        .service_service
        .get_by_id(&runtime_before.id)
        .await
        .unwrap()
        .expect("runtime service is live");
    let bindings_before = existing.base.bindings.len();

    let mut stripped = existing.clone();
    stripped.base.bindings = vec![];
    services
        .service_service
        .upsert_service(existing, stripped, AuthenticatedEntity::System)
        .await
        .expect("an empty-binding upsert must succeed");

    let after = services
        .service_service
        .get_by_id(&runtime_before.id)
        .await
        .unwrap()
        .expect("runtime service still live");

    assert_eq!(
        after.base.bindings.len(),
        bindings_before,
        "upserting with no bindings must not clear the ones already persisted — the row phase \
         relies on this"
    );
}

/// A container service names the runtime that hosts it, and on a rescan that reference is the
/// daemon's freshly minted id — not the persisted one. Without resolving it before the insert,
/// `services_virtualization_service_id_fkey` aborts the entire host.
///
/// This is the service-side twin of `bridge_subnet_owner_resolves_on_first_scan`: same broken
/// reference, reached from the main service loop rather than the subnet loop.
#[tokio::test]
async fn container_service_owner_survives_a_rescan() {
    harness!(services, network_id, _container);

    let first = Submission::container_host(network_id);
    let bridge_cidr = first.bridge_cidr();
    let response = submit(&services, first).await.expect("first scan persists");

    let persisted = services
        .subnet_service
        .get_all(StorableFilter::<Subnet>::new_from_network_ids(&[network_id]).live())
        .await
        .unwrap();
    let lan_id = persisted
        .iter()
        .find(|s| s.base.subnet_type == SubnetType::Lan)
        .expect("lan persisted")
        .id;
    let bridge_id = persisted
        .iter()
        .find(|s| *s.base.cidr == bridge_cidr)
        .expect("bridge persisted")
        .id;

    let mut second = Submission::container_host(network_id);
    second.rescan(response.id, lan_id, bridge_id);
    // The container keeps naming the runtime by the id minted for THIS scan.
    second.services[1].base.virtualization_service_id = Some(second.services[0].id);
    submit(&services, second).await.expect("rescan persists");

    let all = services
        .service_service
        .get_all(StorableFilter::<Service>::new_from_network_ids(&[network_id]).live())
        .await
        .unwrap();

    let runtime = all
        .iter()
        .find(|s| s.base.service_definition.name() == "Docker")
        .expect("runtime service persisted");
    let container = all
        .iter()
        .find(|s| s.base.service_definition.name() != "Docker")
        .expect("container service persisted");

    assert_eq!(
        container.base.virtualization_service_id,
        Some(runtime.id),
        "the container must still point at the runtime that hosts it after a rescan"
    );

    let runtimes = all
        .iter()
        .filter(|s| s.base.service_definition.name() == "Docker")
        .count();
    assert_eq!(
        runtimes, 1,
        "the runtime service must not duplicate on rescan"
    );
}

/// `hosts.virtualization_service_id` is server-authoritative.
///
/// A VM guest names a hypervisor service on a different host, submitted in a different call, so
/// no ordering inside a submission could resolve it — and `CreatedEntitiesPayload` never returns
/// service mappings, so a daemon cannot learn the real id to send. Anything the payload carries
/// is therefore dropped rather than written, which is what keeps the FK satisfiable when the
/// Proxmox integration starts populating this field.
#[tokio::test]
async fn daemon_supplied_host_owner_is_ignored() {
    harness!(services, network_id, _container);

    let mut submission = Submission::container_host(network_id);
    submission.host.base.virtualization_service_id = Some(Uuid::new_v4());

    let response = submit(&services, submission)
        .await
        .expect("a daemon-supplied host owner must not fail the host");

    assert_eq!(
        response.virtualization_service_id, None,
        "discovery must not write a host virtualizer it cannot resolve"
    );
}

/// The API *does* own this field — the UI sets it when a user assigns a VM to a hypervisor — so
/// a bad id there is a caller error and should say so, rather than surfacing as a foreign-key
/// 500 from Postgres.
#[tokio::test]
async fn api_rejects_an_unresolvable_host_virtualizer() {
    harness!(services, network_id, _container);

    let result = services
        .host_service
        .validate_virtualization_service(Some(Uuid::new_v4()))
        .await;

    assert!(
        result.is_err(),
        "an unresolvable virtualization_service_id must be rejected as a validation error"
    );

    services
        .host_service
        .validate_virtualization_service(None)
        .await
        .expect("a bare-metal host has no virtualizer and must be accepted");

    let _ = network_id;
}
