//! `UpdateHostRequest.interfaces` — manual interface sync via the host update endpoint.
//!
//! Mirrors `ip_addresses`/`ports`/`services`, which already sync this way: a client-provided
//! `id` that exists on the host is updated, one that doesn't is created, and an existing row
//! missing from the list is deleted. `None` preserves whatever is already there. Newly wired —
//! `InterfaceInput` existed but nothing constructed it from a request before this.

use uuid::Uuid;

use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::hosts::r#impl::api::{InterfaceInput, UpdateHostRequest};
use crate::server::hosts::r#impl::base::{Host, HostBase};
use crate::server::hosts::r#impl::name::{HostName, HostNameSources};
use crate::server::networks::r#impl::{Network, NetworkBase};
use crate::server::shared::services::factory::ServiceFactory;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::traits::{Storable, Storage};
use crate::server::shared::types::entities::EntitySource;

use super::{organization, test_services};

async fn make_host(services: &ServiceFactory, network_id: Uuid) -> Host {
    services
        .host_service
        .create(
            Host::new(HostBase {
                network_id,
                source: EntitySource::Manual,
                ..Default::default()
            }),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap()
}

fn update_request(id: Uuid, interfaces: Option<Vec<InterfaceInput>>) -> UpdateHostRequest {
    UpdateHostRequest {
        id,
        name: HostName::unnamed().value().as_str().to_string(),
        hostname: None,
        description: None,
        virtualization_metadata: None,
        virtualization_service_id: None,
        hidden: false,
        tags: vec![],
        expected_updated_at: None,
        ip_addresses: None,
        ports: None,
        services: None,
        interfaces,
        credential_assignments: None,
    }
}

fn interface_input(id: Uuid, if_descr: &str) -> InterfaceInput {
    InterfaceInput {
        id,
        if_index: 1,
        if_descr: if_descr.to_string(),
        if_alias: None,
        if_type: None,
        speed_bps: None,
        admin_status: None,
        oper_status: None,
        mac_address: None,
        ip_address_id: None,
    }
}

#[tokio::test]
async fn a_provided_interface_is_created() {
    let (storage, services, _container) = test_services().await;
    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let network = services
        .network_service
        .create(
            Network::new(NetworkBase::new(org.id)),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    let host = make_host(&services, network.id).await;
    let interface_id = Uuid::new_v4();

    services
        .host_service
        .update_from_request(
            update_request(host.id, Some(vec![interface_input(interface_id, "eth0")])),
            AuthenticatedEntity::System,
        )
        .await
        .expect("the update must succeed");

    let persisted = services
        .interface_service
        .get_for_host(&host.id)
        .await
        .unwrap();
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].id, interface_id);
    assert_eq!(persisted[0].base.if_descr.as_deref(), Some("eth0"));
}

#[tokio::test]
async fn omitting_interfaces_preserves_existing_ones() {
    let (storage, services, _container) = test_services().await;
    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let network = services
        .network_service
        .create(
            Network::new(NetworkBase::new(org.id)),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    let host = make_host(&services, network.id).await;
    let interface_id = Uuid::new_v4();
    services
        .host_service
        .update_from_request(
            update_request(host.id, Some(vec![interface_input(interface_id, "eth0")])),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    // A second update that says nothing about interfaces at all.
    services
        .host_service
        .update_from_request(update_request(host.id, None), AuthenticatedEntity::System)
        .await
        .expect("the update must succeed");

    let persisted = services
        .interface_service
        .get_for_host(&host.id)
        .await
        .unwrap();
    assert_eq!(
        persisted.len(),
        1,
        "omitting the field must preserve the existing interface, not delete it"
    );
    assert_eq!(persisted[0].id, interface_id);
}

#[tokio::test]
async fn a_provided_list_deletes_what_it_omits_and_updates_what_it_names() {
    let (storage, services, _container) = test_services().await;
    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let network = services
        .network_service
        .create(
            Network::new(NetworkBase::new(org.id)),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    let host = make_host(&services, network.id).await;
    let kept_id = Uuid::new_v4();
    let removed_id = Uuid::new_v4();
    services
        .host_service
        .update_from_request(
            update_request(
                host.id,
                Some(vec![
                    interface_input(kept_id, "eth0"),
                    interface_input(removed_id, "eth1"),
                ]),
            ),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    // Second update: renames the kept one, omits the other entirely.
    services
        .host_service
        .update_from_request(
            update_request(host.id, Some(vec![interface_input(kept_id, "renamed")])),
            AuthenticatedEntity::System,
        )
        .await
        .expect("the update must succeed");

    let persisted = services
        .interface_service
        .get_for_host(&host.id)
        .await
        .unwrap();
    assert_eq!(persisted.len(), 1, "the omitted interface must be deleted");
    assert_eq!(persisted[0].id, kept_id);
    assert_eq!(persisted[0].base.if_descr.as_deref(), Some("renamed"));
}
