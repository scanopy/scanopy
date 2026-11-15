//! A scan's subnet targets live in `discovery_subnets`, whose foreign key drops a target with its
//! subnet. A scan whose last target is deleted reads as targeting none, which covers its daemon's
//! interfaced subnets.

use uuid::Uuid;

use super::{daemon, host, organization, site, subnet, test_services, user};
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    discovery::r#impl::{
        base::{Discovery, DiscoveryBase},
        types::{DiscoveryType, RunType},
    },
    shared::{
        events::types::DeletionReason,
        services::traits::CrudService,
        storage::traits::{Storable, Storage},
    },
};

fn targets(discovery: &Discovery) -> Option<Vec<Uuid>> {
    match &discovery.base.discovery_type {
        DiscoveryType::Unified { subnet_ids, .. } => subnet_ids.clone(),
        other => panic!("unexpected type {other:?}"),
    }
}

#[tokio::test]
async fn a_deleted_subnet_drops_out_of_the_scans_that_target_it() {
    let (storage, services, _db) = test_services().await;
    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let owner = user(&org.id);
    storage.users.create(&owner).await.unwrap();
    let s = site(&org.id);
    storage.sites.create(&s).await.unwrap();
    let h = host(&s.id);
    storage.hosts.create(&h).await.unwrap();
    let mut d = daemon(&s.id, &h.id);
    d.base.user_id = owner.id;
    storage.daemons.create(&d).await.unwrap();
    let (a, b) = (subnet(&s.id), subnet(&s.id));
    storage.subnets.create(&a).await.unwrap();
    storage.subnets.create(&b).await.unwrap();
    let mut both = vec![a.id, b.id];
    both.sort();

    let scan = services
        .discovery_service
        .create_discovery(
            Discovery::new(DiscoveryBase {
                name: "Office LAN".to_string(),
                site_id: s.id,
                daemon_id: d.id,
                run_type: RunType::AdHoc { last_run: None },
                discovery_type: DiscoveryType::Unified {
                    host_id: h.id,
                    subnet_ids: Some(vec![a.id, b.id]),
                    host_naming_fallback: Default::default(),
                    scan_settings: Default::default(),
                },
                ..Default::default()
            }),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();
    let read = |id| {
        let service = services.discovery_service.clone();
        async move { service.get_by_id(&id).await.unwrap().unwrap() }
    };
    assert_eq!(targets(&read(scan.id).await), Some(both));

    services
        .subnet_service
        .delete(
            &a.id,
            AuthenticatedEntity::System,
            DeletionReason::EntityRequested,
        )
        .await
        .unwrap();
    assert_eq!(targets(&read(scan.id).await), Some(vec![b.id]));

    services
        .subnet_service
        .delete(
            &b.id,
            AuthenticatedEntity::System,
            DeletionReason::EntityRequested,
        )
        .await
        .unwrap();
    assert_eq!(targets(&read(scan.id).await), None);
}
