//! A daemon's status report writes only what it changed. A report identical to the last one
//! costs the same number of statements however many subnets it carries, and a report that does
//! change the subnets lands in that poll.

use std::net::Ipv4Addr;

use cidr::{IpCidr, Ipv4Cidr};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{Layer, registry};
use uuid::Uuid;

use crate::daemon::runtime::state::DaemonStatus;
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::daemons::r#impl::base::Daemon;
use crate::server::metrics::db::{DbTimingLayer, measure};
use crate::server::shared::attribution::AttributeSource;
use crate::server::shared::events::types::DeletionReason;
use crate::server::shared::services::factory::ServiceFactory;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::factory::StorageFactory;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::{Storable, Storage};
use crate::server::shared::types::entities::EntitySource;
use crate::server::subnets::r#impl::base::{
    Subnet, SubnetBase, SubnetCidr, SubnetCidrValue, SubnetName, SubnetNameValue,
    SubnetTypeAttributed,
};
use crate::server::subnets::r#impl::types::SubnetType;

use super::{daemon, host, organization, site, test_services, user};

async fn stored_daemon(storage: &StorageFactory, services: &ServiceFactory) -> Daemon {
    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let site_row = services
        .site_service
        .create(site(&org.id), AuthenticatedEntity::System)
        .await
        .unwrap();
    let daemon_host = storage.hosts.create(&host(&site_row.id)).await.unwrap();
    let owner = user(&org.id);
    storage.users.create(&owner).await.unwrap();
    let mut row = daemon(&site_row.id, &daemon_host.id);
    row.base.user_id = owner.id;
    storage.daemons.create(&row).await.unwrap();
    row
}

/// The subnet a daemon reports for `10.0.{third}.0/24`.
fn reported(site_id: Uuid, third: u8) -> Subnet {
    let cidr = IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(10, 0, third, 0), 24).unwrap());
    Subnet::new(SubnetBase {
        vlan_ids: Vec::new(),
        name: SubnetName::new(
            SubnetNameValue(cidr.to_string()),
            AttributeSource::DaemonSelfReport,
        ),
        description: None,
        site_id,
        cidr: SubnetCidr::new(SubnetCidrValue(cidr), AttributeSource::DaemonSelfReport),
        subnet_type: SubnetTypeAttributed::unattributed(SubnetType::Lan),
        virtualization_service_id: None,
        source: EntitySource::Discovery,
        tags: Vec::new(),
    })
}

fn status(daemon: &Daemon, subnets: Vec<Subnet>) -> DaemonStatus {
    DaemonStatus {
        url: None,
        name: daemon.base.name.clone(),
        mode: daemon.base.mode,
        version: Some(semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap()),
        capabilities: Default::default(),
        interfaced_subnets: Some(subnets),
        interfaces_fingerprint: None,
        ready_for_work: true,
        os: None,
    }
}

/// The live subnets on `site_id` that a daemon reported, leaving out the ones a new site starts with.
async fn site_subnets(services: &ServiceFactory, site_id: Uuid) -> Vec<Subnet> {
    services
        .subnet_service
        .get_all(StorableFilter::<Subnet>::new_from_site_ids(&[site_id]).live())
        .await
        .unwrap()
        .into_iter()
        .filter(|s| s.base.source == EntitySource::Discovery)
        .collect()
}

/// Statements run by the second of two identical reports carrying `count` subnets.
async fn repeat_poll_statements(count: u8) -> u64 {
    let (storage, services, _db) = test_services().await;
    let stored = stored_daemon(&storage, &services).await;
    let subnets: Vec<Subnet> = (0..count)
        .map(|i| reported(stored.base.site_id, i))
        .collect();
    let daemons = &services.daemon_service;
    daemons
        .process_status(
            stored.id,
            status(&stored, subnets.clone()),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();
    assert_eq!(
        daemons.get_interfaced_subnet_ids(&stored.id).await.len(),
        count as usize
    );

    // A callsite another test thread first hit while no dispatcher was enabled for it can cache
    // "never"; recompute every callsite's interest with this thread's dispatcher in place.
    tracing::callsite::rebuild_interest_cache();
    let (result, totals) = measure(daemons.process_status(
        stored.id,
        status(&stored, subnets),
        AuthenticatedEntity::System,
    ))
    .await;
    result.unwrap();
    totals.queries
}

#[tokio::test]
async fn an_unchanged_report_costs_the_same_for_one_subnet_or_twenty() {
    // Installed before any statement runs, so sqlx's statement events are enabled for this
    // thread from their first use.
    let subscriber = registry().with(DbTimingLayer.with_filter(DbTimingLayer::filter()));
    let _tracing = tracing::subscriber::set_default(subscriber);
    let one = repeat_poll_statements(1).await;
    let twenty = repeat_poll_statements(20).await;
    assert_eq!(one, twenty, "statements grew with the reported subnets");
    // Daemon read (2), site subnets (1), junction (1), last_seen (1).
    assert!(
        (1..=6).contains(&one),
        "an unchanged report ran {one} statements"
    );
}

#[tokio::test]
async fn a_changed_subnet_set_updates_the_junction_in_that_poll() {
    let (storage, services, _db) = test_services().await;
    let stored = stored_daemon(&storage, &services).await;
    let site_id = stored.base.site_id;
    let daemons = &services.daemon_service;
    daemons
        .process_status(
            stored.id,
            status(&stored, vec![reported(site_id, 1), reported(site_id, 2)]),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    daemons
        .process_status(
            stored.id,
            status(&stored, vec![reported(site_id, 2), reported(site_id, 3)]),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    let subnets = site_subnets(&services, site_id).await;
    assert_eq!(subnets.len(), 3, "the new CIDR creates exactly one subnet");
    let id_of = |third: u8| {
        let cidr = *reported(site_id, third).base.cidr;
        subnets
            .iter()
            .find(|s| *s.base.cidr == cidr)
            .map(|s| s.id)
            .unwrap()
    };
    let mut expected = vec![id_of(2), id_of(3)];
    expected.sort();
    assert_eq!(
        daemons.get_interfaced_subnet_ids(&stored.id).await,
        expected
    );
}

#[tokio::test]
async fn a_deleted_interfaced_subnet_is_recreated_on_the_next_poll() {
    let (storage, services, _db) = test_services().await;
    let stored = stored_daemon(&storage, &services).await;
    let site_id = stored.base.site_id;
    let daemons = &services.daemon_service;
    let report = vec![reported(site_id, 1)];
    daemons
        .process_status(
            stored.id,
            status(&stored, report.clone()),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();
    let first = daemons.get_interfaced_subnet_ids(&stored.id).await;
    services
        .subnet_service
        .delete(
            &first[0],
            AuthenticatedEntity::System,
            DeletionReason::EntityRequested,
        )
        .await
        .unwrap();
    assert!(
        daemons
            .get_interfaced_subnet_ids(&stored.id)
            .await
            .is_empty()
    );

    daemons
        .process_status(
            stored.id,
            status(&stored, report),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    let subnets = site_subnets(&services, site_id).await;
    assert_eq!(subnets.len(), 1);
    assert_eq!(
        daemons.get_interfaced_subnet_ids(&stored.id).await,
        vec![subnets[0].id]
    );
}

#[tokio::test]
async fn a_report_naming_another_site_lands_on_the_daemons_site() {
    let (storage, services, _db) = test_services().await;
    let stored = stored_daemon(&storage, &services).await;
    let elsewhere = Uuid::new_v4();
    services
        .daemon_service
        .process_status(
            stored.id,
            status(&stored, vec![reported(elsewhere, 1)]),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    assert_eq!(site_subnets(&services, stored.base.site_id).await.len(), 1);
}

#[tokio::test]
async fn a_legacy_daemon_still_sets_the_junction_from_ids() {
    let (storage, services, _db) = test_services().await;
    let stored = stored_daemon(&storage, &services).await;
    let existing = services
        .subnet_service
        .create(
            reported(stored.base.site_id, 1),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();
    let mut legacy = status(&stored, Vec::new());
    legacy.version = Some(semver::Version::new(0, 14, 0));
    legacy.interfaced_subnets = None;
    legacy.capabilities.interfaced_subnet_ids = vec![existing.id, Uuid::new_v4()];

    services
        .daemon_service
        .process_status(stored.id, legacy, AuthenticatedEntity::System)
        .await
        .unwrap();

    assert_eq!(
        services
            .daemon_service
            .get_interfaced_subnet_ids(&stored.id)
            .await,
        vec![existing.id]
    );
}
