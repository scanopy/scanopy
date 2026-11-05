//! Deleting a runtime or hypervisor deletes what it runs, and port turnover never deletes one.

use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::bindings::r#impl::base::Binding;
use crate::server::hosts::r#impl::base::{Host, HostBase};
use crate::server::hosts::r#impl::virtualization::HostVirtualizationDiscriminants;
use crate::server::shared::attribution::AttributeSource;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::traits::Storable;
use crate::server::shared::types::entities::EntitySource;
use crate::server::subnets::r#impl::base::{Subnet, SubnetBase, SubnetCidr, SubnetCidrValue};
use crate::server::subnets::r#impl::types::SubnetType;
use uuid::Uuid;

use super::discovery_stale_port_guards::{LAN_IP, Lab};
use super::discovery_stale_port_holders::covering;
use super::test_services;
use crate::server::subnets::r#impl::base::{SubnetName, SubnetTypeAttributed};

/// Deleting the Docker service deletes its containers and the bridge subnet it owns.
#[tokio::test]
async fn deleting_a_runtime_deletes_its_containers_and_bridge_subnets() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let (lan, p3001) = (lab.lan_ip(), lab.tcp(3001));
    let runtime = lab.runtime(&lan);
    let kuma = lab.container_as("UptimeKuma", "kuma", &runtime, vec![(&p3001, &lan)]);
    lab.scan(vec![lan], vec![p3001], vec![runtime, kuma]).await;
    let runtime_id = lab.named("Docker").await.unwrap().id;
    let bridge = services
        .subnet_service
        .create(
            Subnet::new(SubnetBase {
                name: SubnetName::unattributed("docker-compose_default".to_string()),
                site_id: lab.site_id,
                cidr: SubnetCidr::new(
                    SubnetCidrValue("172.18.0.0/16".parse().unwrap()),
                    AttributeSource::DaemonSelfReport,
                ),
                subnet_type: SubnetTypeAttributed::unattributed(SubnetType::DockerBridge),
                source: EntitySource::Discovery,
                virtualization_service_id: Some(runtime_id),
                ..Default::default()
            }),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    services
        .service_service
        .delete(
            &runtime_id,
            AuthenticatedEntity::System,
            crate::server::shared::events::types::DeletionReason::EntityRequested,
        )
        .await
        .unwrap();

    assert!(lab.named("kuma").await.is_none());
    assert!(
        services
            .subnet_service
            .get_by_id(&bridge.id)
            .await
            .unwrap()
            .is_none()
    );
}

/// Deleting a hypervisor's host deletes the VMs its hypervisor service runs, under every
/// hypervisor.
#[tokio::test]
async fn deleting_a_hypervisor_host_deletes_its_guests() {
    let (storage, services, _container) = test_services().await;
    for kind in HostVirtualizationDiscriminants::hypervisor_guests() {
        let lab = Lab::new(&services, &storage).await;
        let guest = hypervisor_with_guest(&lab, kind).await;

        services
            .host_service
            .delete_host(
                &lab.host_id,
                AuthenticatedEntity::System,
                crate::server::shared::events::types::DeletionReason::EntityRequested,
            )
            .await
            .unwrap();

        assert!(!lab.host_exists(&guest).await, "{kind:?}");
    }
}

/// A runtime whose last port is handed over keeps running its containers: port turnover never
/// deletes a runtime.
#[tokio::test]
async fn an_emptied_runtime_is_not_deleted_by_a_handover() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let (lan, p2375, p3001) = (lab.lan_ip(), lab.tcp(2375), lab.tcp(3001));
    let mut runtime = lab.service("Docker", vec![(&p2375, &lan)]);
    runtime.base.name = "Docker".to_string();
    let kuma = lab.container_as("UptimeKuma", "kuma", &runtime, vec![(&p3001, &lan)]);
    lab.scan(vec![lan], vec![p2375, p3001], vec![runtime, kuma])
        .await;

    let (lan, p2375) = (lab.lan_ip(), lab.tcp(2375));
    let traefik = lab.service("Traefik", vec![(&p2375, &lan)]);
    lab.scan_covering(
        vec![lan],
        vec![p2375],
        vec![traefik],
        covering(LAN_IP, &[2375]),
    )
    .await;

    assert!(
        lab.named("Traefik").await.is_some(),
        "the newcomer took the port"
    );
    assert!(lab.named("Docker").await.is_some());
    assert!(lab.named("kuma").await.is_some());
}

/// A stored service as the host modal sends it back on save.
fn as_input(
    service: &crate::server::services::r#impl::base::Service,
) -> crate::server::hosts::r#impl::api::ServiceInput {
    use crate::server::bindings::r#impl::base::BindingType;
    use crate::server::hosts::r#impl::api::{BindingInput, ServiceInput};
    ServiceInput {
        id: service.id,
        service_definition: service.base.service_definition.clone(),
        name: service.base.name.clone(),
        bindings: service
            .base
            .bindings
            .iter()
            .map(|b| match b.base.binding_type {
                BindingType::IPAddress { ip_address_id } => BindingInput::IPAddress {
                    id: b.id(),
                    ip_address_id,
                },
                BindingType::Port {
                    port_id,
                    ip_address_id,
                } => BindingInput::Port {
                    id: b.id(),
                    port_id,
                    ip_address_id,
                },
            })
            .collect(),
        virtualization_metadata: service.base.virtualization_metadata.clone(),
        virtualization_service_id: service.base.virtualization_service_id,
        tags: service.base.tags.clone(),
        position: None,
    }
}

/// The host modal saved with the Docker service removed but its container still in the form:
/// the save succeeds, and the container goes with its runtime.
#[tokio::test]
async fn removing_a_runtime_in_the_host_modal_deletes_its_containers() {
    use crate::server::hosts::r#impl::api::UpdateHostRequest;

    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let (lan, p3001) = (lab.lan_ip(), lab.tcp(3001));
    let runtime = lab.runtime(&lan);
    let kuma = lab.container_as("UptimeKuma", "kuma", &runtime, vec![(&p3001, &lan)]);
    lab.scan(vec![lan], vec![p3001], vec![runtime, kuma]).await;
    let host = services
        .host_service
        .get_by_id(&lab.host_id)
        .await
        .unwrap()
        .unwrap();
    let kept: Vec<_> = lab
        .stored()
        .await
        .into_iter()
        .filter(|s| s.base.name != "Docker")
        .map(|s| as_input(&s))
        .collect();

    services
        .host_service
        .update_from_request(
            UpdateHostRequest {
                id: host.id,
                name: host.base.name.value().as_str().to_string(),
                hostname: None,
                description: None,
                virtualization_metadata: None,
                virtualization_service_id: None,
                virtualization_interface_id: None,
                tags: vec![],
                expected_updated_at: None,
                manufacturer: None,
                model: None,
                serial_number: None,
                asset_tag: None,
                sys_location: None,
                sys_contact: None,
                management_url: None,
                os: None,
                device_type: None,
                ip_addresses: None,
                ports: None,
                services: Some(kept),
                interfaces: None,
                credential_assignments: None,
            },
            AuthenticatedEntity::System,
        )
        .await
        .expect("the save succeeds");

    assert!(lab.named("Docker").await.is_none());
    assert!(
        lab.named("kuma").await.is_none(),
        "the container goes with its runtime"
    );
}

/// The bulk delete deletes what a runtime runs, the same as the single delete.
#[tokio::test]
async fn bulk_deleting_a_runtime_deletes_its_containers() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let (lan, p3001) = (lab.lan_ip(), lab.tcp(3001));
    let runtime = lab.runtime(&lan);
    let kuma = lab.container_as("UptimeKuma", "kuma", &runtime, vec![(&p3001, &lan)]);
    lab.scan(vec![lan], vec![p3001], vec![runtime, kuma]).await;
    let runtime_id = lab.named("Docker").await.unwrap().id;

    services
        .service_service
        .delete_many(
            &[runtime_id],
            AuthenticatedEntity::System,
            crate::server::shared::events::types::DeletionReason::EntityRequested,
        )
        .await
        .unwrap();

    assert!(lab.named("kuma").await.is_none());
}

/// The host delete the API reaches through the CRUD trait deletes the hypervisor's guests.
#[tokio::test]
async fn deleting_a_hypervisor_host_through_the_api_deletes_its_guests() {
    let (storage, services, _container) = test_services().await;
    for kind in HostVirtualizationDiscriminants::hypervisor_guests() {
        let lab = Lab::new(&services, &storage).await;
        let guest = hypervisor_with_guest(&lab, kind).await;

        CrudService::<Host>::delete(
            &*services.host_service,
            &lab.host_id,
            AuthenticatedEntity::System,
            crate::server::shared::events::types::DeletionReason::EntityRequested,
        )
        .await
        .unwrap();

        assert!(!lab.host_exists(&guest).await, "{kind:?}");
    }
}

/// Bulk-deleting a hypervisor and its guest together: the guest is already gone when its turn
/// comes, and that is not an error.
#[tokio::test]
async fn bulk_deleting_a_hypervisor_with_its_guest_deletes_both() {
    let (storage, services, _container) = test_services().await;
    for kind in HostVirtualizationDiscriminants::hypervisor_guests() {
        let lab = Lab::new(&services, &storage).await;
        let guest = hypervisor_with_guest(&lab, kind).await;

        CrudService::<Host>::delete_many(
            &*services.host_service,
            &[lab.host_id, guest],
            AuthenticatedEntity::System,
            crate::server::shared::events::types::DeletionReason::EntityRequested,
        )
        .await
        .expect("both go without an error");

        assert!(!lab.host_exists(&lab.host_id).await, "{kind:?}");
        assert!(!lab.host_exists(&guest).await, "{kind:?}");
    }
}

/// A host on `lab`'s LAN running the hypervisor service for `kind`, with one guest under it.
/// Returns the guest's host id.
async fn hypervisor_with_guest(lab: &Lab<'_>, kind: HostVirtualizationDiscriminants) -> Uuid {
    let lan = lab.lan_ip();
    let definition = kind.manager_definition();
    let mut manager = lab.service(definition, vec![]);
    manager.base.bindings = vec![Binding::new_ip_address_serviceless(lan.id)];
    lab.scan(vec![lan], vec![], vec![manager]).await;
    let manager_id = lab.named(definition).await.unwrap().id;
    lab.submit_host(
        Host::new(HostBase {
            source: EntitySource::Discovery,
            virtualization_metadata: Some(kind.guest("vm", "100")),
            virtualization_service_id: Some(manager_id),
            ..Default::default()
        }),
        vec![lab.lan_ip_at("192.168.4.70", "02:00:00:00:00:70")],
    )
    .await
}
