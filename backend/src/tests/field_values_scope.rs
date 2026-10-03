//! The field-values endpoint counts only rows the caller could list: a network outside their
//! `network_ids` contributes no values, whether or not the request names it, and a list that
//! always carries a scope (the run history's `historical`) counts only rows inside it.

use chrono::Utc;
use uuid::Uuid;

use crate::daemon::discovery::types::base::DiscoveryPhase;
use crate::server::daemons::r#impl::api::DiscoveryUpdatePayload;
use crate::server::discovery::handlers::{DiscoveryFilterQuery, DiscoveryOrderField};
use crate::server::discovery::r#impl::base::{Discovery, DiscoveryBase};
use crate::server::discovery::r#impl::types::{DiscoveryType, RunType};
use crate::server::hosts::handlers::{HostFilterQuery, HostOrderField};
use crate::server::hosts::r#impl::attributes::HostManufacturerValue;
use crate::server::hosts::r#impl::base::Host;
use crate::server::services::handlers::{ServiceFilterQuery, ServiceOrderField};
use crate::server::services::r#impl::base::Service;
use crate::server::shared::attribution::{AttributeSource, Attributed};
use crate::server::shared::handlers::ordering::OrderField;
use crate::server::shared::handlers::traits::field_values_filter;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::Storage;

use super::{daemon, host, network, organization, service, test_services, user};

#[tokio::test]
async fn field_values_exclude_networks_outside_the_callers_access() {
    let (storage, services, _container) = test_services().await;

    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let visible = network(&org.id);
    storage.networks.create(&visible).await.unwrap();
    let hidden = network(&org.id);
    storage.networks.create(&hidden).await.unwrap();

    for (network_id, manufacturer) in [(visible.id, "Cisco"), (hidden.id, "Juniper")] {
        let mut h = host(&network_id);
        h.base.manufacturer = Some(Attributed::new(
            HostManufacturerValue(manufacturer.to_string()),
            AttributeSource::ReverseDns,
        ));
        storage.hosts.create(&h).await.unwrap();
    }

    let field = HostOrderField::Manufacturer;
    let values = |query: HostFilterQuery| {
        let filter = field_values_filter::<Host>(&[visible.id], org.id, &query, field);
        let services = &services;
        async move {
            services
                .host_service
                .count_by_group(filter, field.to_sql())
                .await
                .unwrap()
                .into_iter()
                .map(|g| g.value)
                .collect::<Vec<_>>()
        }
    };

    assert_eq!(
        values(HostFilterQuery::default()).await,
        vec![Some("Cisco".to_string())],
        "an unscoped request must only count the caller's networks"
    );

    let asks_for_hidden = HostFilterQuery {
        network_ids: Some(vec![hidden.id]),
        ..Default::default()
    };
    assert!(
        values(asks_for_hidden).await.is_empty(),
        "naming an inaccessible network must narrow to nothing, never widen"
    );
}

#[tokio::test]
async fn service_field_values_exclude_networks_outside_the_callers_access() {
    let (storage, services, _container) = test_services().await;

    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let visible = network(&org.id);
    storage.networks.create(&visible).await.unwrap();
    let hidden = network(&org.id);
    storage.networks.create(&hidden).await.unwrap();

    for (network_id, name) in [(visible.id, "Visible DNS"), (hidden.id, "Hidden DNS")] {
        let h = host(&network_id);
        storage.hosts.create(&h).await.unwrap();
        let mut s = service(&network_id, &h.id);
        s.base.name = name.to_string();
        storage.services.create(&s).await.unwrap();
    }

    let field = ServiceOrderField::Name;
    let values = |query: ServiceFilterQuery| {
        let filter = field_values_filter::<Service>(&[visible.id], org.id, &query, field);
        let services = &services;
        async move {
            services
                .service_service
                .count_by_group(filter, field.to_sql())
                .await
                .unwrap()
                .into_iter()
                .map(|g| g.value)
                .collect::<Vec<_>>()
        }
    };

    assert_eq!(
        values(ServiceFilterQuery::default()).await,
        vec![Some("Visible DNS".to_string())],
        "an unscoped request must only count the caller's networks"
    );
    assert!(
        values(ServiceFilterQuery {
            network_ids: Some(vec![hidden.id]),
            ..Default::default()
        })
        .await
        .is_empty(),
        "naming an inaccessible network must narrow to nothing, never widen"
    );
}

/// The run history's options come from history rows only: a scheduled configuration on the same
/// network must not add its name, nor a phase-less group.
#[tokio::test]
async fn discovery_field_values_with_historical_count_only_history_rows() {
    let (storage, services, _container) = test_services().await;

    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let net = network(&org.id);
    storage.networks.create(&net).await.unwrap();
    let daemon_host = host(&net.id);
    storage.hosts.create(&daemon_host).await.unwrap();
    let owner = user(&org.id);
    storage.users.create(&owner).await.unwrap();
    let mut d = daemon(&net.id, &daemon_host.id);
    d.base.user_id = owner.id;
    storage.daemons.create(&d).await.unwrap();

    let discovery = |name: &str, run_type: RunType| {
        let now = Utc::now();
        Discovery {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: DiscoveryBase {
                discovery_type: DiscoveryType::default(),
                run_type,
                name: name.to_string(),
                daemon_id: d.id,
                network_id: net.id,
                tags: Vec::new(),
            },
            ..Default::default()
        }
    };

    storage
        .discovery
        .create(&discovery(
            "Nightly sweep",
            RunType::Scheduled {
                cron_schedule: "0 0 * * *".to_string(),
                last_run: None,
                enabled: true,
                timezone: None,
            },
        ))
        .await
        .unwrap();
    storage
        .discovery
        .create(&discovery(
            "Completed run",
            RunType::Historical {
                results: Box::new(DiscoveryUpdatePayload {
                    session_id: Uuid::new_v4(),
                    daemon_id: d.id,
                    network_id: net.id,
                    phase: DiscoveryPhase::Complete,
                    discovery_type: DiscoveryType::default(),
                    progress: 100,
                    error: None,
                    warnings: Vec::new(),
                    credential_results: Vec::new(),
                    started_at: Some(Utc::now()),
                    finished_at: Some(Utc::now()),
                    hosts_discovered: None,
                    estimated_remaining_secs: None,
                    discovery_id: None,
                    scanned: None,
                    reason: None,
                    last_update_at: None,
                    daemon_version: None,
                }),
            },
        ))
        .await
        .unwrap();

    let values = |field: DiscoveryOrderField, query: DiscoveryFilterQuery| {
        let filter = field_values_filter::<Discovery>(&[net.id], org.id, &query, field);
        let services = &services;
        async move {
            services
                .discovery_service
                .count_by_group(filter, field.to_sql())
                .await
                .unwrap()
                .into_iter()
                .map(|g| g.value)
                .collect::<Vec<_>>()
        }
    };
    let history = || DiscoveryFilterQuery {
        historical: Some(true),
        ..Default::default()
    };

    assert_eq!(
        values(DiscoveryOrderField::Name, history()).await,
        vec![Some("Completed run".to_string())],
        "a scheduled configuration must not add its name to the history's options"
    );
    assert_eq!(
        values(DiscoveryOrderField::Phase, history()).await,
        vec![Some("Complete".to_string())],
        "a configuration has no phase, so it must not add an empty phase group"
    );
    assert_eq!(
        values(DiscoveryOrderField::Name, DiscoveryFilterQuery::default())
            .await
            .len(),
        2,
        "without the history scope both rows count, so the scope is what excludes the schedule"
    );
}

/// "Virtualized By" offers the virtualizing service's name, and the name filter must select
/// exactly the hosts counted under it.
#[tokio::test]
async fn virtualized_by_name_filter_matches_the_counted_name() {
    let (storage, services, _container) = test_services().await;

    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let net = network(&org.id);
    storage.networks.create(&net).await.unwrap();

    let hypervisor = host(&net.id);
    storage.hosts.create(&hypervisor).await.unwrap();
    let mut runtime = service(&net.id, &hypervisor.id);
    runtime.base.name = "Proxmox VE".to_string();
    storage.services.create(&runtime).await.unwrap();

    let mut guest = host(&net.id);
    guest.base.virtualization_service_id = Some(runtime.id);
    storage.hosts.create(&guest).await.unwrap();

    let field = HostOrderField::VirtualizedBy;
    let filter = field_values_filter::<Host>(&[net.id], org.id, &HostFilterQuery::default(), field);
    let counted: Vec<Option<String>> = services
        .host_service
        .count_by_group(filter, field.to_sql())
        .await
        .unwrap()
        .into_iter()
        .map(|g| g.value)
        .collect();
    assert_eq!(
        counted,
        vec![Some(String::new()), Some("Proxmox VE".to_string())],
        "the unvirtualized hypervisor counts as '', the guest under its runtime's name"
    );

    let selected =
        |names: &[String], include_null: bool| {
            let filter = StorableFilter::<Host>::new_from_network_ids(&[net.id])
                .virtualization_parent(&[], names, include_null);
            let services = &services;
            async move {
                let mut ids: Vec<Uuid> = services
                    .host_service
                    .get_all(filter)
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|h| h.id)
                    .collect();
                ids.sort();
                ids
            }
        };

    assert_eq!(
        selected(&["Proxmox VE".to_string()], false).await,
        vec![guest.id]
    );
    assert_eq!(selected(&[], true).await, vec![hypervisor.id]);
    let mut both = vec![guest.id, hypervisor.id];
    both.sort();
    assert_eq!(selected(&["Proxmox VE".to_string()], true).await, both);
}
