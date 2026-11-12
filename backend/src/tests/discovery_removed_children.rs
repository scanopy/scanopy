//! Containers and VMs their runtime or hypervisor no longer has, and the ones it still has but
//! that are stopped.
//!
//! A stopped container is reported, with nothing probed, so the server knows the runtime still
//! has it. A removed one is not, and once a run lists the runtime's guests in full, it goes.
//! Shares `discovery_stale_port_guards`'s `Lab`.

use crate::server::shared::events::types::UpdateReason;
use secrecy::SecretString;
use uuid::Uuid;

use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::credentials::r#impl::base::{Credential, CredentialBase};
use crate::server::credentials::r#impl::run_results::{CredentialRunOutcome, CredentialRunResult};
use crate::server::credentials::r#impl::types::CredentialType;
use crate::server::credentials::r#impl::types::SecretValue;
use crate::server::daemons::r#impl::api::ScannedEntityIds;
use crate::server::hosts::r#impl::base::{Host, HostBase};
use crate::server::hosts::r#impl::virtualization::HostVirtualizationDiscriminants;
use crate::server::interfaces::r#impl::base::InterfaceDataComplete;
use crate::server::ip_addresses::r#impl::base::IPAddress;
use crate::server::ports::r#impl::coverage::PortCoverage;
use crate::server::services::r#impl::base::Service;
use crate::server::services::r#impl::virtualization::ServiceVirtualization;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::types::entities::EntitySource;
use crate::server::shared::types::guest_state::GuestState;

use super::discovery_stale_port_guards::{Lab, at_lan};
use super::test_services;

/// A Proxmox VE API token credential.
pub(super) fn proxmox_token() -> CredentialType {
    CredentialType::ProxmoxApiToken {
        port: 8006,
        token_id: "scanopy@pve!discovery".to_string(),
        token_secret: SecretValue::Inline {
            value: SecretString::from("token".to_string()),
        },
    }
}

/// The state a stored container service carries.
fn state_of(service: &Service) -> Option<GuestState> {
    match service.base.virtualization_metadata.as_ref()? {
        ServiceVirtualization::Docker(d) => Some(d.state),
        ServiceVirtualization::Podman(p) => Some(p.state),
    }
}

impl Lab<'_> {
    /// A credential of `credential_type` in this lab's organization.
    pub(super) async fn credential(&self, credential_type: CredentialType) -> Uuid {
        let site = self
            .services
            .site_service
            .get_by_id(&self.site_id)
            .await
            .unwrap()
            .unwrap();
        self.services
            .credential_service
            .create(
                Credential::new(CredentialBase {
                    organization_id: site.base.organization_id,
                    credential_type,
                    ..Default::default()
                }),
                AuthenticatedEntity::System,
            )
            .await
            .unwrap()
            .id
    }

    /// A libvirt credential in this lab's organization, over a new SSH login.
    pub(super) async fn libvirt_credential(&self) -> Uuid {
        let login = self
            .credential(CredentialType::SshPassword {
                port: 22,
                username: "scanopy".to_string(),
                password: SecretValue::Inline {
                    value: SecretString::from("password".to_string()),
                },
                host_key_fingerprint: None,
            })
            .await;
        let organization_id = self
            .services
            .site_service
            .get_by_id(&self.site_id)
            .await
            .unwrap()
            .unwrap()
            .base
            .organization_id;
        self.services
            .credential_service
            .create(
                Credential::new(CredentialBase {
                    organization_id,
                    login_credential_id: Some(login),
                    credential_type: CredentialType::Libvirt {
                        connection_uri: "qemu:///system".to_string(),
                    },
                    ..Default::default()
                }),
                AuthenticatedEntity::System,
            )
            .await
            .unwrap()
            .id
    }

    /// What a run reports when it submitted the stored services named in `services` (by name)
    /// and the hosts in `hosts`.
    pub(super) async fn reported(&self, hosts: &[Uuid], services: &[&str]) -> ScannedEntityIds {
        ScannedEntityIds {
            host_ids: hosts.to_vec(),
            service_ids: self
                .stored()
                .await
                .into_iter()
                .filter(|s| services.contains(&s.base.name.as_str()))
                .map(|s| s.id)
                .collect(),
            ..Default::default()
        }
    }

    /// End a run whose `credential_id` collection listed the guests of `listed` in full.
    pub(super) async fn finish_run(
        &self,
        credential_id: Uuid,
        listed: &[Uuid],
        scanned: &ScannedEntityIds,
    ) {
        self.services
            .host_service
            .remove_unreported_guests(
                self.site_id,
                scanned,
                &[CredentialRunResult {
                    credential_id,
                    outcome: CredentialRunOutcome::Collected {
                        hosts: 1,
                        hosts_with_guests_listed: listed.to_vec(),
                    },
                }],
                AuthenticatedEntity::System,
            )
            .await
            .expect("the run ends");
    }

    /// Submit a host other than the lab's own, as an integration does.
    pub(super) async fn submit_host(&self, mut host: Host, ip_addresses: Vec<IPAddress>) -> Uuid {
        host.base.site_id = self.site_id;
        self.services
            .host_service
            .discover_host(
                host,
                ip_addresses,
                vec![],
                vec![],
                vec![],
                vec![],
                false,
                InterfaceDataComplete::none(),
                &PortCoverage::default(),
                None,
                AuthenticatedEntity::System,
                None,
            )
            .await
            .expect("the host persists")
            .host
            .id
    }

    pub(super) async fn host_exists(&self, id: &Uuid) -> bool {
        self.services
            .host_service
            .get_by_id(id)
            .await
            .unwrap()
            .is_some()
    }
}

/// Uptime Kuma's container was removed from the runtime, and the run listed the runtime's
/// containers in full. It goes; the runtime stays.
#[tokio::test]
async fn a_container_the_listed_runtime_no_longer_has_is_deleted() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let docker = lab
        .credential(CredentialType::DockerSocket { socket_path: None })
        .await;
    let (lan, p3001) = (lab.lan_ip(), lab.tcp(3001));
    let runtime = lab.runtime(&lan);
    let kuma = lab.container_as("UptimeKuma", "kuma", &runtime, vec![(&p3001, &lan)]);
    lab.scan(vec![lan], vec![p3001], vec![runtime, kuma]).await;

    let lan = lab.lan_ip();
    let runtime = lab.runtime(&lan);
    lab.scan(vec![lan], vec![], vec![runtime]).await;
    let scanned = lab.reported(&[lab.host_id], &["Docker"]).await;
    lab.finish_run(docker, &[lab.host_id], &scanned).await;

    assert!(lab.named("kuma").await.is_none());
    assert!(lab.named("Docker").await.is_some());
}

/// A host the run didn't list (a partial container scan, an older daemon) keeps its containers.
#[tokio::test]
async fn an_unlisted_host_keeps_its_containers() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let docker = lab
        .credential(CredentialType::DockerSocket { socket_path: None })
        .await;
    let (lan, p3001) = (lab.lan_ip(), lab.tcp(3001));
    let runtime = lab.runtime(&lan);
    let kuma = lab.container_as("UptimeKuma", "kuma", &runtime, vec![(&p3001, &lan)]);
    lab.scan(vec![lan], vec![p3001], vec![runtime, kuma]).await;

    let scanned = lab.reported(&[lab.host_id], &["Docker"]).await;
    lab.finish_run(docker, &[], &scanned).await;

    assert!(lab.named("kuma").await.is_some());
}

/// A listed host the run didn't report: its submission never landed, so nothing under it is
/// judged.
#[tokio::test]
async fn a_listed_host_the_run_did_not_report_keeps_its_containers() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let docker = lab
        .credential(CredentialType::DockerSocket { socket_path: None })
        .await;
    let (lan, p3001) = (lab.lan_ip(), lab.tcp(3001));
    let runtime = lab.runtime(&lan);
    let kuma = lab.container_as("UptimeKuma", "kuma", &runtime, vec![(&p3001, &lan)]);
    lab.scan(vec![lan], vec![p3001], vec![runtime, kuma]).await;

    lab.finish_run(docker, &[lab.host_id], &ScannedEntityIds::default())
        .await;

    assert!(lab.named("kuma").await.is_some());
}

/// A stopped container is reported, so it is not removed.
#[tokio::test]
async fn a_stopped_container_is_not_removed() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let docker = lab
        .credential(CredentialType::DockerSocket { socket_path: None })
        .await;
    let (lan, p3001) = (lab.lan_ip(), lab.tcp(3001));
    let runtime = lab.runtime(&lan);
    let kuma = lab.container_as("UptimeKuma", "kuma", &runtime, vec![(&p3001, &lan)]);
    lab.scan(vec![lan], vec![p3001], vec![runtime, kuma]).await;

    let lan = lab.lan_ip();
    let runtime = lab.runtime(&lan);
    let mut stopped = lab.container_as("Docker Container", "kuma", &runtime, vec![]);
    if let Some(ServiceVirtualization::Docker(d)) = stopped.base.virtualization_metadata.as_mut() {
        d.state = GuestState::Stopped;
    }
    lab.scan(vec![lan], vec![], vec![runtime, stopped]).await;
    let scanned = lab.reported(&[lab.host_id], &["Docker", "kuma"]).await;
    lab.finish_run(docker, &[lab.host_id], &scanned).await;

    let kuma = lab.named("kuma").await.expect("still stored");
    assert_eq!(kuma.base.service_definition.id(), "UptimeKuma");
    assert_eq!(state_of(&kuma), Some(GuestState::Stopped));
}

/// A stopped container can only be reported as a generic container, since nothing can be
/// probed. It is the stored Uptime Kuma by container id, so it stays Uptime Kuma with its
/// binding, and its state becomes Stopped.
#[tokio::test]
async fn a_stopped_container_keeps_its_identity_and_its_ports() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let (lan, p3001) = (lab.lan_ip(), lab.tcp(3001));
    let runtime = lab.runtime(&lan);
    let kuma = lab.container_as("UptimeKuma", "kuma", &runtime, vec![(&p3001, &lan)]);
    lab.scan(vec![lan], vec![p3001], vec![runtime, kuma]).await;

    let lan = lab.lan_ip();
    let runtime = lab.runtime(&lan);
    let mut stopped = lab.container_as("Docker Container", "kuma", &runtime, vec![]);
    if let Some(ServiceVirtualization::Docker(d)) = stopped.base.virtualization_metadata.as_mut() {
        d.state = GuestState::Stopped;
    }
    lab.scan(vec![lan], vec![], vec![runtime, stopped]).await;

    let containers: Vec<_> = lab
        .stored()
        .await
        .into_iter()
        .filter(|s| s.base.virtualization_metadata.is_some())
        .collect();
    assert_eq!(containers.len(), 1, "one service for the one container");
    assert_eq!(containers[0].base.service_definition.id(), "UptimeKuma");
    assert_eq!(state_of(&containers[0]), Some(GuestState::Stopped));
    assert_eq!(lab.ports_named("kuma").await, Some(vec![at_lan(3001)]));
}

/// A manually created child of a listed runtime is a person's record, and stays.
#[tokio::test]
async fn a_manual_container_is_not_removed() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let docker = lab
        .credential(CredentialType::DockerSocket { socket_path: None })
        .await;
    let (lan, p3001) = (lab.lan_ip(), lab.tcp(3001));
    let runtime = lab.runtime(&lan);
    lab.scan(vec![lan], vec![p3001], vec![runtime]).await;
    let runtime_id = lab.named("Docker").await.unwrap().id;
    let mut manual = lab
        .create_manual(
            "UptimeKuma",
            3001,
            super::discovery_stale_port_guards::LAN_IP,
        )
        .await;
    manual.base.virtualization_service_id = Some(runtime_id);
    services
        .service_service
        .update(
            &mut manual,
            AuthenticatedEntity::System,
            UpdateReason::EntityRequested,
        )
        .await
        .unwrap();

    let scanned = lab.reported(&[lab.host_id], &["Docker"]).await;
    lab.finish_run(docker, &[lab.host_id], &scanned).await;

    assert!(lab.named("UptimeKuma").await.is_some());
}

/// A credential whose integration doesn't report containers judges none: a Proxmox collection
/// listing this host says nothing about its Docker runtime.
#[tokio::test]
async fn another_integration_does_not_remove_containers() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let proxmox = lab.credential(proxmox_token()).await;
    let (lan, p3001) = (lab.lan_ip(), lab.tcp(3001));
    let runtime = lab.runtime(&lan);
    let kuma = lab.container_as("UptimeKuma", "kuma", &runtime, vec![(&p3001, &lan)]);
    lab.scan(vec![lan], vec![p3001], vec![runtime, kuma]).await;

    let scanned = lab.reported(&[lab.host_id], &["Docker"]).await;
    lab.finish_run(proxmox, &[lab.host_id], &scanned).await;

    assert!(lab.named("kuma").await.is_some());
}

/// A guest VM removed from a listed hypervisor goes. A guest the run reported stays. Holds for
/// each integration that lists a hypervisor's guests.
#[tokio::test]
async fn a_guest_the_listed_hypervisor_no_longer_has_is_deleted() {
    for virtualization in [
        HostVirtualizationDiscriminants::Proxmox,
        HostVirtualizationDiscriminants::Libvirt,
    ] {
        let (storage, services, _container) = test_services().await;
        let lab = Lab::new(&services, &storage).await;
        let credential = match virtualization {
            HostVirtualizationDiscriminants::Proxmox => lab.credential(proxmox_token()).await,
            _ => lab.libvirt_credential().await,
        };
        let lan = lab.lan_ip();
        let definition = virtualization.manager_definition();
        let mut hypervisor = lab.service(definition, vec![]);
        hypervisor.base.bindings = vec![
            crate::server::bindings::r#impl::base::Binding::new_ip_address_serviceless(lan.id),
        ];
        lab.scan(vec![lan], vec![], vec![hypervisor]).await;
        let hypervisor_id = lab.named(definition).await.unwrap().id;

        let guest = |name: &str| {
            Host::new(HostBase {
                source: EntitySource::Discovery,
                virtualization_metadata: Some(virtualization.guest(name, "100")),
                virtualization_service_id: Some(hypervisor_id),
                ..Default::default()
            })
        };
        let kept = lab
            .submit_host(
                guest("kept"),
                vec![lab.lan_ip_at("192.168.4.70", "02:00:00:00:00:70")],
            )
            .await;
        let removed = lab
            .submit_host(
                guest("removed"),
                vec![lab.lan_ip_at("192.168.4.71", "02:00:00:00:00:71")],
            )
            .await;

        let scanned = ScannedEntityIds {
            host_ids: vec![lab.host_id, kept],
            service_ids: vec![hypervisor_id],
            ..Default::default()
        };
        lab.finish_run(credential, &[lab.host_id], &scanned).await;

        assert!(lab.host_exists(&kept).await, "{virtualization:?}");
        assert!(!lab.host_exists(&removed).await, "{virtualization:?}");
    }
}

/// Older daemons send no list, and an older server's payload reads with none.
#[test]
fn a_collected_result_without_the_list_reads_as_listing_nothing() {
    let parsed: CredentialRunOutcome =
        serde_json::from_str(r#"{"type":"Collected","hosts":3}"#).unwrap();
    assert_eq!(
        parsed,
        CredentialRunOutcome::Collected {
            hosts: 3,
            hosts_with_guests_listed: vec![],
        }
    );
}

/// Every Service `Updated` published, as the service each one carries.
#[derive(Default)]
struct ServiceUpdates(std::sync::Mutex<Vec<Service>>);

#[async_trait::async_trait]
impl
    crate::server::shared::events::traits::Subscriber<
        crate::server::shared::events::types::EntityOperation,
    > for ServiceUpdates
{
    fn filter(&self) -> crate::server::shared::events::traits::EntityEventFilter {
        use crate::server::shared::entities::EntityDiscriminants;
        use crate::server::shared::events::types::EntityOperationDiscriminants;
        crate::server::shared::events::traits::EntityEventFilter::by_entity(
            std::collections::HashMap::from([(
                EntityDiscriminants::Service,
                Some(vec![EntityOperationDiscriminants::Updated]),
            )]),
        )
    }

    async fn handle(
        &self,
        events: Vec<
            crate::server::shared::events::traits::Event<
                crate::server::shared::events::types::EntityOperation,
            >,
        >,
    ) -> anyhow::Result<()> {
        let mut updates = self.0.lock().unwrap();
        for event in events {
            if let crate::server::shared::entities::Entity::Service(service) =
                event.scope.entity_type()
            {
                updates.push(service.clone());
            }
        }
        Ok(())
    }
}

/// A container stopping changes nothing but its state, and that is still an update: the open
/// page and the topology hear it.
#[tokio::test]
async fn a_container_stopping_publishes_an_update() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let (lan, p3001) = (lab.lan_ip(), lab.tcp(3001));
    let runtime = lab.runtime(&lan);
    let kuma = lab.container_as("UptimeKuma", "kuma", &runtime, vec![(&p3001, &lan)]);
    lab.scan(vec![lan], vec![p3001], vec![runtime, kuma]).await;
    let kuma_id = lab.named("kuma").await.unwrap().id;
    let updates = std::sync::Arc::new(ServiceUpdates::default());
    services
        .event_bus
        .register::<crate::server::shared::events::types::EntityOperation>(
            updates.clone(),
            "test_service_updates",
        )
        .await;

    let lan = lab.lan_ip();
    let runtime = lab.runtime(&lan);
    let mut stopped = lab.container_as("Docker Container", "kuma", &runtime, vec![]);
    if let Some(ServiceVirtualization::Docker(d)) = stopped.base.virtualization_metadata.as_mut() {
        d.state = GuestState::Stopped;
    }
    lab.scan(vec![lan], vec![], vec![runtime, stopped]).await;
    services.event_bus.flush().await;

    let heard: Vec<Option<GuestState>> = updates
        .0
        .lock()
        .unwrap()
        .iter()
        .filter(|s| s.id == kuma_id)
        .map(state_of)
        .collect();
    assert_eq!(heard, vec![Some(GuestState::Stopped)]);
}

/// A rescan that finds the container as it was publishes nothing for it.
#[tokio::test]
async fn an_unchanged_container_publishes_no_update() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let (lan, p3001) = (lab.lan_ip(), lab.tcp(3001));
    let runtime = lab.runtime(&lan);
    let kuma = lab.container_as("UptimeKuma", "kuma", &runtime, vec![(&p3001, &lan)]);
    lab.scan(vec![lan], vec![p3001], vec![runtime, kuma]).await;
    let kuma_id = lab.named("kuma").await.unwrap().id;
    let updates = std::sync::Arc::new(ServiceUpdates::default());
    services
        .event_bus
        .register::<crate::server::shared::events::types::EntityOperation>(
            updates.clone(),
            "test_service_updates",
        )
        .await;

    let (lan, p3001) = (lab.lan_ip(), lab.tcp(3001));
    let runtime = lab.runtime(&lan);
    let kuma = lab.container_as("UptimeKuma", "kuma", &runtime, vec![(&p3001, &lan)]);
    lab.scan(vec![lan], vec![p3001], vec![runtime, kuma]).await;
    services.event_bus.flush().await;

    assert!(
        updates.0.lock().unwrap().iter().all(|s| s.id != kuma_id),
        "an unchanged container is not updated"
    );
}
