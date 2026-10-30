//! What a daemon reports about its own work stays on its own site, on both daemon modes. A
//! DaemonPoll daemon posts its reports; a ServerPoll daemon answers a poll from an address its
//! owner chose, so its reports are no more trusted than a request body.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;
use uuid::Uuid;

use super::self_hosted_licensing::{cloud_test_state, test_state_from};
use super::tenant_isolation::{
    Tenant, is_ok, post, progress, scan, serve_as_daemon, stored_credential, stored_daemon, tenant,
};
use super::{host, subnet};
use crate::daemon::discovery::types::base::DiscoveryPhase;
use crate::daemon::discovery::types::warnings::DiscoveryWarning;
use crate::daemon::runtime::state::{BufferedEntities, DiscoveryPollResponse};
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::config::{AppState, ServerConfig};
use crate::server::credentials::r#impl::run_results::{CredentialRunOutcome, CredentialRunResult};
use crate::server::daemons::r#impl::api::{DiscoveryUpdatePayload, ScannedEntityIds};
use crate::server::daemons::r#impl::base::Daemon;
use crate::server::discovery::r#impl::types::DiscoveryType;
use crate::server::hosts::r#impl::api::DiscoveryHostRequest;
use crate::server::hosts::r#impl::base::Host;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::types::api::ApiResponse;
use crate::server::subnets::r#impl::base::Subnet;

fn system() -> AuthenticatedEntity {
    AuthenticatedEntity::System
}

/// A ServerPoll daemon that answers every poll with `response`. Returns its address.
async fn polled_daemon(response: DiscoveryPollResponse) -> SocketAddr {
    let app = Router::new().route(
        "/api/poll",
        get(move || {
            let response = response.clone();
            async move { Json(ApiResponse::success(response)) }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    addr
}

/// Poll `daemon` once, as the server's poll loop does, with `response` as its answer.
async fn poll(state: &AppState, daemon: &Daemon, response: DiscoveryPollResponse) {
    let addr = polled_daemon(response).await;
    let mut daemon = daemon.clone();
    daemon.base.url = format!("http://{addr}");
    assert!(
        state
            .services
            .daemon_service
            .drain_discovery(&daemon, "unused")
            .await,
        "the poll reached the daemon"
    );
}

/// Self-hosted, because the SSRF guard refuses a loopback daemon on cloud.
async fn polling_state() -> (
    Arc<AppState>,
    testcontainers::ContainerAsync<testcontainers::GenericImage>,
) {
    test_state_from(ServerConfig::default()).await
}

fn discovered(host: Host) -> DiscoveryHostRequest {
    serde_json::from_value(json!({
        "host": host,
        "ip_addresses": [],
        "ports": [],
        "services": [],
    }))
    .unwrap()
}

async fn hosts_on(state: &AppState, tenant: &Tenant) -> usize {
    state
        .services
        .host_service
        .get_all(StorableFilter::<Host>::new_from_site_ids(&[tenant.site.id]))
        .await
        .unwrap()
        .len()
}

async fn subnets_on(state: &AppState, tenant: &Tenant) -> usize {
    state
        .services
        .subnet_service
        .get_all(StorableFilter::<Subnet>::new_from_site_ids(&[tenant
            .site
            .id]))
        .await
        .unwrap()
        .len()
}

#[tokio::test]
async fn a_polled_daemon_cannot_open_a_session_on_another_site() {
    let (state, _container) = polling_state().await;
    let mine = tenant(&state, "a@example.com").await;
    let theirs = tenant(&state, "b@example.com").await;
    let own_daemon = stored_daemon(&state, &mine).await;
    let foreign_daemon = stored_daemon(&state, &theirs).await;

    // Its own report goes through, so a refusal below is about the site.
    let own_session = Uuid::new_v4();
    poll(
        &state,
        &own_daemon,
        DiscoveryPollResponse {
            progress: Some(progress(own_session, own_daemon.id, mine.site.id, 10)),
            entities: BufferedEntities::default(),
        },
    )
    .await;
    assert!(
        state
            .services
            .discovery_service
            .get_session(&own_session)
            .await
            .is_some()
    );

    for (daemon_id, site_id) in [
        (own_daemon.id, theirs.site.id),
        (foreign_daemon.id, theirs.site.id),
        (foreign_daemon.id, mine.site.id),
    ] {
        let session_id = Uuid::new_v4();
        poll(
            &state,
            &own_daemon,
            DiscoveryPollResponse {
                progress: Some(progress(session_id, daemon_id, site_id, 10)),
                entities: BufferedEntities::default(),
            },
        )
        .await;
        assert!(
            state
                .services
                .discovery_service
                .get_session(&session_id)
                .await
                .is_none(),
            "a session for daemon {daemon_id} on site {site_id} was opened"
        );
    }
}

#[tokio::test]
async fn a_polled_daemon_cannot_store_entities_on_another_site() {
    let (state, _container) = polling_state().await;
    let mine = tenant(&state, "a@example.com").await;
    let theirs = tenant(&state, "b@example.com").await;
    let own_daemon = stored_daemon(&state, &mine).await;
    let (own_hosts, own_subnets) = (
        hosts_on(&state, &mine).await,
        subnets_on(&state, &mine).await,
    );
    let (foreign_hosts, foreign_subnets) = (
        hosts_on(&state, &theirs).await,
        subnets_on(&state, &theirs).await,
    );

    poll(
        &state,
        &own_daemon,
        DiscoveryPollResponse {
            progress: None,
            entities: BufferedEntities {
                hosts: vec![
                    discovered(host(&mine.site.id)),
                    discovered(host(&theirs.site.id)),
                ],
                subnets: vec![subnet(&mine.site.id), subnet(&theirs.site.id)],
                vlans: Vec::new(),
            },
        },
    )
    .await;

    assert_eq!(hosts_on(&state, &mine).await, own_hosts + 1, "own host");
    assert_eq!(
        subnets_on(&state, &mine).await,
        own_subnets + 1,
        "own subnet"
    );
    assert_eq!(hosts_on(&state, &theirs).await, foreign_hosts, "their host");
    assert_eq!(
        subnets_on(&state, &theirs).await,
        foreign_subnets,
        "their subnet"
    );
}

/// A report on its way to the session, with every id it can carry naming `theirs` beside `mine`.
fn report_naming(
    daemon: &Daemon,
    site_id: Uuid,
    own: [Uuid; 2],
    foreign: [Uuid; 3],
) -> DiscoveryUpdatePayload {
    let [own_host, own_subnet] = own;
    let [foreign_host, foreign_subnet, foreign_credential] = foreign;
    let mut report = progress(Uuid::new_v4(), daemon.id, site_id, 50);
    report.discovery_type = DiscoveryType::Unified {
        host_id: daemon.base.host_id,
        subnet_ids: Some(vec![own_subnet, foreign_subnet]),
        host_naming_fallback: Default::default(),
        scan_settings: Default::default(),
    };
    report.scanned = Some(ScannedEntityIds {
        host_ids: vec![own_host, foreign_host],
        subnet_ids: vec![own_subnet, foreign_subnet],
        found_subnet_ids: vec![foreign_subnet],
        ..ScannedEntityIds::default()
    });
    report.credential_results = vec![CredentialRunResult {
        credential_id: foreign_credential,
        outcome: CredentialRunOutcome::Collected {
            hosts: 0,
            hosts_with_guests_listed: Vec::new(),
        },
    }];
    report.warnings = vec![DiscoveryWarning::IntegrationWithoutCredential {
        host_id: Some(foreign_host),
        integration: serde_json::from_value(json!("Snmp")).unwrap(),
    }];
    report
}

#[tokio::test]
async fn a_report_keeps_only_what_is_on_the_daemons_site() {
    let (state, _container) = cloud_test_state().await;
    let mine = tenant(&state, "a@example.com").await;
    let theirs = tenant(&state, "b@example.com").await;
    let own_daemon = stored_daemon(&state, &mine).await;
    let services = &state.services;
    let own_subnet = services
        .subnet_service
        .create(subnet(&mine.site.id), system())
        .await
        .unwrap();
    let foreign_subnet = services
        .subnet_service
        .create(subnet(&theirs.site.id), system())
        .await
        .unwrap();
    let foreign_host = services
        .host_service
        .create(host(&theirs.site.id), system())
        .await
        .unwrap();
    let foreign_credential = stored_credential(&state, theirs.org.id).await;

    let report = report_naming(
        &own_daemon,
        mine.site.id,
        [own_daemon.base.host_id, own_subnet.id],
        [foreign_host.id, foreign_subnet.id, foreign_credential.id],
    );
    let session_id = report.session_id;
    let addr = serve_as_daemon(&state, mine.site.id, own_daemon.id).await;
    let path = format!("/api/v1/discovery/{session_id}/update");
    let (status, reply) = post(addr, &path, &json!(report)).await;
    assert!(is_ok(status), "{reply}");

    let stored = services
        .discovery_service
        .get_session(&session_id)
        .await
        .unwrap();
    let scanned = stored.scanned.unwrap();
    assert_eq!(scanned.host_ids, vec![own_daemon.base.host_id]);
    assert_eq!(scanned.subnet_ids, vec![own_subnet.id]);
    assert!(scanned.found_subnet_ids.is_empty(), "the server fills this");
    assert!(stored.credential_results.is_empty(), "their credential");
    assert!(stored.warnings.is_empty(), "the server raises this warning");
    let DiscoveryType::Unified { subnet_ids, .. } = stored.discovery_type else {
        panic!("the report's discovery type changed");
    };
    assert_eq!(subnet_ids, Some(vec![own_subnet.id]));
}

#[tokio::test]
async fn a_report_cannot_name_a_discovery_on_another_site() {
    let (state, _container) = cloud_test_state().await;
    let mine = tenant(&state, "a@example.com").await;
    let theirs = tenant(&state, "b@example.com").await;
    let own_daemon = stored_daemon(&state, &mine).await;
    let foreign_daemon = stored_daemon(&state, &theirs).await;
    let foreign = state
        .services
        .discovery_service
        .create(scan(&foreign_daemon, Vec::new()), system())
        .await
        .unwrap();

    let session_id = Uuid::new_v4();
    let mut report = progress(session_id, own_daemon.id, mine.site.id, 100);
    report.phase = DiscoveryPhase::Complete;
    report.started_at = Some(chrono::Utc::now());
    report.finished_at = Some(chrono::Utc::now());
    report.discovery_id = Some(foreign.id);

    let addr = serve_as_daemon(&state, mine.site.id, own_daemon.id).await;
    let path = format!("/api/v1/discovery/{session_id}/update");
    post(addr, &path, &json!(report)).await;

    let stored = state
        .services
        .discovery_service
        .get_by_id(&foreign.id)
        .await
        .unwrap()
        .expect("their discovery is still there");
    assert_eq!(stored.scan_count, foreign.scan_count, "scan_count moved");
}
