//! A historical run's warnings are rows of `discovery_warnings`: written when the run is recorded,
//! read back onto the run, and kept when what they name is deleted.

use std::net::{IpAddr, Ipv4Addr};

use cidr::{IpCidr, Ipv4Cidr};

use chrono::Duration;
use sqlx::Row;
use uuid::Uuid;

use crate::daemon::discovery::types::base::DiscoveryPhase;
use crate::daemon::discovery::types::warnings::{
    CredentialAttempt, DiscoveryWarning, ProvisionalSubnet, UnresolvedPort,
};
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::credentials::r#impl::base::{Credential, CredentialBase};
use crate::server::credentials::r#impl::mapping::CredentialQueryPayloadDiscriminants;
use crate::server::daemons::r#impl::api::DiscoveryUpdatePayload;
use crate::server::discovery::r#impl::base::Discovery;
use crate::server::discovery::r#impl::types::{DiscoveryType, RunType};
use crate::server::discovery_warnings::r#impl::base::DiscoveryWarningRecord;
use crate::server::shared::attribution::AttributeSource;
use crate::server::shared::services::factory::ServiceFactory;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::factory::StorageFactory;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::generic::GenericPostgresStorage;
use crate::server::shared::storage::migration_runner::split_statements;
use crate::server::shared::storage::traits::Storage;
use crate::server::subnets::r#impl::base::{SubnetCidr, SubnetCidrValue};

use super::{daemon, host, organization, site, subnet, test_services, user};

const MIGRATION: &str = include_str!("../../migrations/20261012130000_discovery_warnings.sql");

/// What a run's warnings name: a credential, two hosts and a subnet, all on one site.
struct Fixture {
    site_id: Uuid,
    daemon_id: Uuid,
    host_id: Uuid,
    remote_host_id: Uuid,
    subnet_id: Uuid,
    credential_id: Uuid,
}

async fn fixture(storage: &StorageFactory, services: &ServiceFactory) -> Fixture {
    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let site_row = services
        .site_service
        .create(site(&org.id), AuthenticatedEntity::System)
        .await
        .unwrap();
    let near = storage.hosts.create(&host(&site_row.id)).await.unwrap();
    let far = storage.hosts.create(&host(&site_row.id)).await.unwrap();
    let range = storage.subnets.create(&subnet(&site_row.id)).await.unwrap();
    let credential = storage
        .credentials
        .create(&Credential::new(CredentialBase {
            organization_id: org.id,
            ..Default::default()
        }))
        .await
        .unwrap();

    let owner = user(&org.id);
    storage.users.create(&owner).await.unwrap();
    let mut d = daemon(&site_row.id, &near.id);
    d.base.user_id = owner.id;
    storage.daemons.create(&d).await.unwrap();

    Fixture {
        site_id: site_row.id,
        daemon_id: d.id,
        host_id: near.id,
        remote_host_id: far.id,
        subnet_id: range.id,
        credential_id: credential.id,
    }
}

/// One warning carrying each type of id, and one carrying none.
fn warnings(f: &Fixture) -> Vec<DiscoveryWarning> {
    vec![
        DiscoveryWarning::CredentialRejected(CredentialAttempt {
            address: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5)),
            integration: CredentialQueryPayloadDiscriminants::Snmp,
            credential_id: Some(f.credential_id),
            detail: Some("authentication failure".to_string()),
        }),
        DiscoveryWarning::LldpPortNotFound(UnresolvedPort {
            host_id: Some(f.host_id),
            if_descr: "Gi0/1".to_string(),
            remote_host_id: Some(f.remote_host_id),
            port_id: Some("InterfaceName(\"2\")".to_string()),
            port_desc: None,
        }),
        DiscoveryWarning::ProvisionalSubnetInferred(ProvisionalSubnet {
            cidr: "10.20.30.0/24".to_string(),
            subnet_id: Some(f.subnet_id),
            addresses: vec!["10.20.30.11".to_string()],
            sys_names: vec!["offsite-core-01".to_string()],
            seen_by_host_ids: vec![f.host_id],
            widened_by_vlan: false,
        }),
        DiscoveryWarning::SnmpCollectedNothing {
            address: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 9)),
            credential_id: None,
            snmp_version: None,
            failures: Some(Vec::new()),
        },
    ]
}

/// Finish a session carrying `warnings` and return the historical run it recorded.
async fn finish_run(
    storage: &StorageFactory,
    services: &ServiceFactory,
    f: &Fixture,
    warnings: Vec<DiscoveryWarning>,
) -> Discovery {
    let session_id = Uuid::new_v4();
    let mut update = DiscoveryUpdatePayload::new(
        session_id,
        f.daemon_id,
        f.site_id,
        DiscoveryType::SelfReport { host_id: f.host_id },
        None,
    );
    update.phase = DiscoveryPhase::Complete;
    update.progress = 100;
    update.started_at = Some(crate::server::shared::clock::now() - Duration::minutes(1));
    update.finished_at = Some(crate::server::shared::clock::now());
    update.warnings = warnings;
    services.discovery_service.update_session(update).await;

    storage
        .discovery
        .get_all(
            StorableFilter::<Discovery>::new_from_site_ids(&[f.site_id]).historical_discovery(),
        )
        .await
        .unwrap()
        .into_iter()
        .find(|d| {
            d.base
                .run_type
                .historical_results()
                .is_some_and(|r| r.session_id == session_id)
        })
        .expect("the session records a historical run")
}

async fn warnings_of(services: &ServiceFactory, discovery_id: Uuid) -> Vec<DiscoveryWarning> {
    let run = services
        .discovery_service
        .get_by_id(&discovery_id)
        .await
        .unwrap()
        .expect("the run exists");
    match run.base.run_type {
        RunType::Historical { results } => results.warnings,
        _ => panic!("expected a historical run"),
    }
}

async fn records_of(storage: &StorageFactory, discovery_id: Uuid) -> Vec<DiscoveryWarningRecord> {
    GenericPostgresStorage::<DiscoveryWarningRecord>::new(storage.pool().clone())
        .get_all(StorableFilter::new_from_uuids_column(
            "discovery_id",
            &[discovery_id],
        ))
        .await
        .unwrap()
}

fn sorted(mut warnings: Vec<DiscoveryWarning>) -> Vec<String> {
    let mut out: Vec<String> = warnings
        .drain(..)
        .map(|w| serde_json::to_string(&w).unwrap())
        .collect();
    out.sort();
    out
}

async fn stored_json_warnings(storage: &StorageFactory, discovery_id: Uuid) -> serde_json::Value {
    sqlx::query("SELECT run_type->'results'->'warnings' AS w FROM discovery WHERE id = $1")
        .bind(discovery_id)
        .fetch_one(storage.pool())
        .await
        .unwrap()
        .get::<Option<serde_json::Value>, _>("w")
        .unwrap_or(serde_json::Value::Null)
}

#[tokio::test]
async fn a_finished_run_stores_one_row_per_warning_with_its_ids_in_columns() {
    let (storage, services, _container) = test_services().await;
    let f = fixture(&storage, &services).await;
    let sent = warnings(&f);
    let run = finish_run(&storage, &services, &f, sent.clone()).await;

    let records = records_of(&storage, run.id).await;
    assert_eq!(records.len(), sent.len());
    let column = |pick: fn(&DiscoveryWarningRecord) -> Option<Uuid>| {
        records.iter().filter_map(pick).collect::<Vec<_>>()
    };
    assert_eq!(column(|r| r.base.credential_id), vec![f.credential_id]);
    assert_eq!(column(|r| r.base.host_id), vec![f.host_id]);
    assert_eq!(column(|r| r.base.remote_host_id), vec![f.remote_host_id]);
    assert_eq!(column(|r| r.base.subnet_id), vec![f.subnet_id]);
    assert!(records.iter().all(|r| r.base.site_id == f.site_id));

    // The ids live in their columns alone: the stored details carry none of them.
    for record in &records {
        let details = serde_json::to_value(&record.base.details).unwrap();
        for key in ["credential_id", "host_id", "remote_host_id", "subnet_id"] {
            assert!(
                details.get(key).is_none_or(serde_json::Value::is_null),
                "{key} left in details: {details}"
            );
        }
    }

    // The run's own JSON no longer holds them, and reading the run puts every one back.
    assert!(
        stored_json_warnings(&storage, run.id)
            .await
            .as_array()
            .is_none_or(Vec::is_empty)
    );
    assert_eq!(sorted(warnings_of(&services, run.id).await), sorted(sent));
}

#[tokio::test]
async fn deleting_what_a_warning_names_keeps_the_warning_without_the_id() {
    let (storage, services, _container) = test_services().await;
    let f = fixture(&storage, &services).await;
    let run = finish_run(&storage, &services, &f, warnings(&f)).await;

    storage.hosts.delete(&f.host_id).await.unwrap();
    storage.hosts.delete(&f.remote_host_id).await.unwrap();
    storage.subnets.delete(&f.subnet_id).await.unwrap();
    storage.credentials.delete(&f.credential_id).await.unwrap();

    let records = records_of(&storage, run.id).await;
    assert_eq!(records.len(), warnings(&f).len());
    assert!(records.iter().all(|r| r.base.credential_id.is_none()
        && r.base.host_id.is_none()
        && r.base.remote_host_id.is_none()
        && r.base.subnet_id.is_none()));

    let read = warnings_of(&services, run.id).await;
    assert_eq!(read.len(), warnings(&f).len());
    for mut warning in read {
        let slots = warning.entity_id_slots();
        for slot in [
            slots.credential_id,
            slots.host_id,
            slots.remote_host_id,
            slots.subnet_id,
        ]
        .into_iter()
        .flatten()
        {
            assert_eq!(*slot, None, "a deleted entity's id comes back None");
        }
    }
}

#[tokio::test]
async fn deleting_a_run_deletes_its_warnings() {
    let (storage, services, _container) = test_services().await;
    let f = fixture(&storage, &services).await;
    let run = finish_run(&storage, &services, &f, warnings(&f)).await;
    assert!(!records_of(&storage, run.id).await.is_empty());

    storage.discovery.delete(&run.id).await.unwrap();

    assert!(records_of(&storage, run.id).await.is_empty());
}

#[tokio::test]
async fn a_run_reads_only_warnings_on_its_own_site() {
    let (storage, services, _container) = test_services().await;
    let f = fixture(&storage, &services).await;
    let run = finish_run(&storage, &services, &f, warnings(&f)).await;

    // Another site in the same organization, with a row claiming this run's id.
    let org_id = storage
        .sites
        .get_by_id(&f.site_id)
        .await
        .unwrap()
        .unwrap()
        .base
        .organization_id;
    let other_site = services
        .site_service
        .create(site(&org_id), AuthenticatedEntity::System)
        .await
        .unwrap();
    let stray = DiscoveryWarningRecord::from_warning(
        run.id,
        other_site.id,
        &DiscoveryWarning::ReverseDnsTimedOut { count: 3 },
    );
    GenericPostgresStorage::<DiscoveryWarningRecord>::new(storage.pool().clone())
        .create(&stray)
        .await
        .unwrap();

    let read = warnings_of(&services, run.id).await;
    assert_eq!(sorted(read), sorted(warnings(&f)));

    // And a reader limited to the other site never sees the run at all.
    let visible = services
        .discovery_service
        .get_all(
            StorableFilter::<Discovery>::new_from_site_ids(&[other_site.id]).historical_discovery(),
        )
        .await
        .unwrap();
    assert!(visible.is_empty());
}

/// Runs written before this table, or by a container from the previous release during the deploy,
/// hold their warnings only in the JSON. The migration copies them, and a second run of it adds
/// nothing.
#[tokio::test]
async fn the_backfill_copies_stored_json_warnings_once() {
    let (storage, services, _container) = test_services().await;
    let f = fixture(&storage, &services).await;
    let run = finish_run(&storage, &services, &f, Vec::new()).await;
    let deleted_host = Uuid::new_v4();

    let json = serde_json::json!([
        "Scan hit its time limit (4h) — 12 host(s) not scanned.",
        { "code": "SomethingANewerDaemonKnowsAbout", "address": "10.0.0.2" },
        {
            "code": "CredentialRejected",
            "address": "10.0.0.5",
            "integration": "Snmp",
            "credential_id": f.credential_id,
            "detail": null
        },
        {
            "code": "LldpPortNotFound",
            "host_id": deleted_host,
            "if_descr": "Gi0/1",
            "remote_host_id": f.remote_host_id,
            "port_id": null,
            "port_desc": null
        }
    ]);
    sqlx::query(
        "UPDATE discovery SET run_type = jsonb_set(run_type, '{results,warnings}', $2) WHERE id = $1",
    )
    .bind(run.id)
    .bind(&json)
    .execute(storage.pool())
    .await
    .unwrap();

    for _ in 0..2 {
        for statement in split_statements(MIGRATION) {
            sqlx::raw_sql(&statement)
                .execute(storage.pool())
                .await
                .unwrap();
        }
    }

    let records = records_of(&storage, run.id).await;
    assert_eq!(records.len(), 4, "one row per stored warning, copied once");

    let read = warnings_of(&services, run.id).await;
    let unknown: Vec<&str> = read
        .iter()
        .filter_map(|w| match w {
            DiscoveryWarning::Unknown { detail } => Some(detail.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        unknown.len(),
        2,
        "a bare string and an unknown code both read as Unknown"
    );
    assert!(unknown.contains(&"Scan hit its time limit (4h) — 12 host(s) not scanned."));

    assert!(
        read.contains(&DiscoveryWarning::CredentialRejected(CredentialAttempt {
            address: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5)),
            integration: CredentialQueryPayloadDiscriminants::Snmp,
            credential_id: Some(f.credential_id),
            detail: None,
        }))
    );
    // The near host never existed, so its id is NULL rather than dangling; the far one is kept.
    assert!(
        read.contains(&DiscoveryWarning::LldpPortNotFound(UnresolvedPort {
            host_id: None,
            if_descr: "Gi0/1".to_string(),
            remote_host_id: Some(f.remote_host_id),
            port_id: None,
            port_desc: None,
        }))
    );
}

/// An assumed range carries the evidence from the scan that created it, not from the later scans
/// that re-report it empty, and never from a row on another site.
#[tokio::test]
async fn an_assumed_range_carries_the_evidence_of_the_scan_that_created_it() {
    let (storage, services, _container) = test_services().await;
    let f = fixture(&storage, &services).await;

    let mut assumed = subnet(&f.site_id);
    assumed.base.cidr = SubnetCidr::new(
        SubnetCidrValue(IpCidr::V4(
            Ipv4Cidr::new(Ipv4Addr::new(10, 20, 30, 0), 24).unwrap(),
        )),
        AttributeSource::LldpNeighbourAddress,
    );
    // An assumed range is an inferred subnet, the way the server creates one.
    assumed.base.source = crate::server::shared::types::entities::EntitySource::Inferred {
        via: crate::server::shared::types::entities::InferredVia::ReportedAddress,
    };
    let assumed = storage.subnets.create(&assumed).await.unwrap();
    let measured = storage
        .subnets
        .get_by_id(&f.subnet_id)
        .await
        .unwrap()
        .unwrap();

    let report = |seen_by: Vec<Uuid>| {
        DiscoveryWarning::ProvisionalSubnetInferred(ProvisionalSubnet {
            cidr: "10.20.30.0/24".to_string(),
            subnet_id: Some(assumed.id),
            addresses: if seen_by.is_empty() {
                Vec::new()
            } else {
                vec!["10.20.30.11".to_string()]
            },
            sys_names: Vec::new(),
            seen_by_host_ids: seen_by,
            widened_by_vlan: false,
        })
    };
    let creating = finish_run(&storage, &services, &f, vec![report(vec![f.host_id])]).await;
    // Later scans re-report the standing range without evidence.
    finish_run(&storage, &services, &f, vec![report(Vec::new())]).await;

    // A newer row with evidence, on another site, naming this range.
    let org_id = storage
        .sites
        .get_by_id(&f.site_id)
        .await
        .unwrap()
        .unwrap()
        .base
        .organization_id;
    let other_site = services
        .site_service
        .create(site(&org_id), AuthenticatedEntity::System)
        .await
        .unwrap();
    GenericPostgresStorage::<DiscoveryWarningRecord>::new(storage.pool().clone())
        .create(&DiscoveryWarningRecord::from_warning(
            creating.id,
            other_site.id,
            &report(vec![f.remote_host_id]),
        ))
        .await
        .unwrap();

    let responses = services
        .subnet_service
        .with_usage(vec![assumed.clone(), measured], None)
        .await
        .unwrap();
    let evidence_of = |id: Uuid| {
        responses
            .iter()
            .find(|r| r.subnet.id == id)
            .unwrap()
            .range_evidence
            .clone()
    };

    let evidence = evidence_of(assumed.id).expect("the assumed range carries its evidence");
    assert_eq!(evidence.discovery_id, creating.id);
    assert_eq!(evidence.seen_by_host_ids, vec![f.host_id]);
    assert_eq!(evidence.addresses, vec!["10.20.30.11".to_string()]);
    assert_eq!(
        evidence_of(f.subnet_id),
        None,
        "a measured range has no evidence"
    );
}
