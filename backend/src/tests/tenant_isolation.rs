//! Requests that name another tenant's rows, through the real router. Each control call shows the
//! same request working with the caller's own row, so a refusal is about the tenant and not a
//! malformed body.

use std::net::SocketAddr;
use std::sync::Arc;

use reqwest::Method;
use secrecy::SecretString;
use serde_json::{Value, json};
use uuid::Uuid;

use super::api_key_site_access::send;
use super::cross_tenant_probe::serve_as;
use super::self_hosted_licensing::{cloud_test_state, create_org};
use super::{daemon, host, ip_address, service, site, subnet, user};
use crate::daemon::discovery::types::base::DiscoveryPhase;
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::billing::plans::get_enterprise_plan;
use crate::server::bindings::r#impl::base::{Binding, BindingBase, BindingType};
use crate::server::config::AppState;
use crate::server::credentials::r#impl::base::{Credential, CredentialBase};
use crate::server::credentials::r#impl::mapping::IntegrationTarget;
use crate::server::credentials::r#impl::types::{
    CredentialAssignment, CredentialType, SecretValue,
};
use crate::server::credentials::service::DaemonPlacement;
use crate::server::daemon_api_keys::r#impl::base::{DaemonApiKey, DaemonApiKeyBase};
use crate::server::daemons::r#impl::api::DiscoveryUpdatePayload;
use crate::server::daemons::r#impl::base::Daemon;
use crate::server::discovery::r#impl::base::{Discovery, DiscoveryBase};
use crate::server::discovery::r#impl::types::{DiscoveryType, RunType};
use crate::server::organizations::r#impl::base::Organization;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::traits::Storable;
use crate::server::shares::r#impl::base::{Share, ShareBase};
use crate::server::sites::r#impl::Site;
use crate::server::topology::types::base::{Topology, TopologyBase};
use crate::server::topology::types::views::TopologyView;
use crate::server::users::r#impl::base::User;
use crate::server::users::r#impl::permissions::UserOrgPermissions;
use crate::server::vlans::r#impl::base::Vlan;

/// An organization, one of its sites, its owner, and the API served as that owner.
struct Tenant {
    org: Organization,
    site: Site,
    owner: User,
    addr: SocketAddr,
}

fn session(user: &User, sites: Vec<Uuid>) -> AuthenticatedEntity {
    AuthenticatedEntity::User {
        user_id: user.id,
        organization_id: user.base.organization_id,
        permissions: UserOrgPermissions::Owner,
        site_ids: sites,
        email: user.base.email.clone(),
        email_verified: true,
        terms_accepted_at: user.base.terms_accepted_at,
    }
}

async fn stored_user(state: &AppState, organization_id: Uuid, email: &str) -> User {
    let mut row = user(&organization_id);
    row.base.email = email.parse().unwrap();
    state
        .services
        .user_service
        .create(row, AuthenticatedEntity::System)
        .await
        .unwrap()
}

async fn stored_site(state: &AppState, organization_id: Uuid) -> Site {
    state
        .services
        .site_service
        .create(site(&organization_id), AuthenticatedEntity::System)
        .await
        .unwrap()
}

async fn tenant(state: &Arc<AppState>, email: &str) -> Tenant {
    let org = create_org(state, get_enterprise_plan(), None).await;
    let owner = stored_user(state, org.id, email).await;
    let own_site = stored_site(state, org.id).await;
    let addr = serve_as(state, session(&owner, vec![own_site.id])).await;
    Tenant {
        org,
        site: own_site,
        owner,
        addr,
    }
}

async fn stored_topology(state: &AppState, site_id: Uuid) -> Topology {
    state
        .services
        .topology_service
        .create(
            Topology::new(TopologyBase::new(site_id)),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap()
}

async fn stored_credential(state: &AppState, organization_id: Uuid) -> Credential {
    state
        .services
        .credential_service
        .create(
            Credential::new(CredentialBase {
                organization_id,
                name: format!("credential {}", Uuid::new_v4()),
                credential_type: CredentialType::SnmpCommunity {
                    community: SecretValue::Inline {
                        value: SecretString::from("community".to_string()),
                    },
                },
                ..Default::default()
            }),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap()
}

/// A daemon that has connected, so its version passes the discovery gates.
async fn stored_daemon(state: &AppState, tenant: &Tenant) -> Daemon {
    let site_id = tenant.site.id;
    let daemon_host = state
        .services
        .host_service
        .create(host(&site_id), AuthenticatedEntity::System)
        .await
        .unwrap();
    let mut row = daemon(&site_id, &daemon_host.id);
    row.base.user_id = tenant.owner.id;
    row.base.version = Some(semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap());
    state
        .services
        .daemon_service
        .create(row, AuthenticatedEntity::System)
        .await
        .unwrap()
}

fn share_body(site_id: Uuid, topology_id: Uuid) -> Value {
    let share = Share::new(ShareBase {
        site_id,
        topology_id,
        name: "shared".to_string(),
        is_enabled: true,
        ..ShareBase::default()
    });
    json!({ "share": share })
}

fn scan(daemon: &Daemon, targets: Vec<IntegrationTarget>) -> Discovery {
    let mut row = Discovery::new(DiscoveryBase {
        discovery_type: DiscoveryType::Unified {
            host_id: daemon.base.host_id,
            subnet_ids: None,
            host_naming_fallback: Default::default(),
            scan_settings: Default::default(),
        },
        run_type: RunType::AdHoc { last_run: None },
        name: "sweep".to_string(),
        daemon_id: daemon.id,
        site_id: daemon.base.site_id,
        tags: Vec::new(),
    });
    row.integration_targets = targets;
    row
}

async fn post(addr: SocketAddr, path: &str, body: &Value) -> (u16, Value) {
    send(addr, Method::POST, path, "session", Some(body)).await
}

async fn put(addr: SocketAddr, path: &str, body: &Value) -> (u16, Value) {
    send(addr, Method::PUT, path, "session", Some(body)).await
}

fn created_id(body: &Value) -> Uuid {
    serde_json::from_value(body["data"]["id"].clone()).unwrap()
}

fn is_ok(status: u16) -> bool {
    (200..300).contains(&status)
}

#[tokio::test]
async fn a_share_must_name_a_topology_on_its_own_site() {
    let (state, _container) = cloud_test_state().await;
    let mine = tenant(&state, "a@example.com").await;
    let theirs = tenant(&state, "b@example.com").await;
    let other_site = stored_site(&state, mine.org.id).await;
    let addr = serve_as(
        &state,
        session(&mine.owner, vec![mine.site.id, other_site.id]),
    )
    .await;

    let own = stored_topology(&state, mine.site.id).await;
    let elsewhere = stored_topology(&state, other_site.id).await;
    let foreign = stored_topology(&state, theirs.site.id).await;

    let (status, body) = post(addr, "/api/v1/shares", &share_body(mine.site.id, own.id)).await;
    assert_eq!(status, 200, "{body}");
    let share_id = created_id(&body);

    for topology in [&elsewhere, &foreign] {
        let body = share_body(mine.site.id, topology.id);
        let (status, _) = post(addr, "/api/v1/shares", &body).await;
        assert_eq!(status, 400, "create with topology {}", topology.id);
        let (status, _) = put(addr, &format!("/api/v1/shares/{share_id}"), &body).await;
        assert_eq!(status, 400, "update with topology {}", topology.id);
    }
}

#[tokio::test]
async fn a_share_already_pointing_off_its_site_is_not_served() {
    let (state, _container) = cloud_test_state().await;
    let mine = tenant(&state, "a@example.com").await;
    let theirs = tenant(&state, "b@example.com").await;
    let foreign = stored_topology(&state, theirs.site.id).await;

    let mut row = Share::new(ShareBase {
        site_id: mine.site.id,
        topology_id: foreign.id,
        created_by: mine.owner.id,
        name: "stored before the check existed".to_string(),
        is_enabled: true,
        ..ShareBase::default()
    });
    row.id = Uuid::new_v4();
    let share = state
        .services
        .share_service
        .create(row, AuthenticatedEntity::System)
        .await
        .unwrap();

    let path = format!("/api/v1/shares/public/{}", share.id);
    let (status, _) = send(mine.addr, Method::GET, &path, "", None).await;
    assert_eq!(status, 404, "metadata");
    let view = json!({ "view": TopologyView::default() });
    let (status, _) = post(mine.addr, &format!("{path}/topology"), &view).await;
    assert_eq!(status, 404, "topology");
}

#[tokio::test]
async fn a_share_keeps_its_creator() {
    let (state, _container) = cloud_test_state().await;
    let mine = tenant(&state, "a@example.com").await;
    let colleague = stored_user(&state, mine.org.id, "c@example.com").await;
    let own = stored_topology(&state, mine.site.id).await;

    let body = share_body(mine.site.id, own.id);
    let (status, created) = post(mine.addr, "/api/v1/shares", &body).await;
    assert_eq!(status, 200, "{created}");
    let share_id = created_id(&created);

    let mut update = body.clone();
    update["share"]["created_by"] = json!(colleague.id);
    let (status, body) = put(mine.addr, &format!("/api/v1/shares/{share_id}"), &update).await;
    assert_eq!(status, 200, "{body}");
    let stored = state
        .services
        .share_service
        .get_by_id(&share_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.base.created_by, mine.owner.id);
}

#[tokio::test]
async fn a_discovery_cannot_name_another_organizations_credential() {
    let (state, _container) = cloud_test_state().await;
    let mine = tenant(&state, "a@example.com").await;
    let theirs = tenant(&state, "b@example.com").await;
    let own_daemon = stored_daemon(&state, &mine).await;
    let own = stored_credential(&state, mine.org.id).await;
    let foreign = stored_credential(&state, theirs.org.id).await;
    let target = |credential_id| IntegrationTarget::Site { credential_id };

    let body = scan(&own_daemon, vec![target(own.id)]);
    let (status, created) = post(mine.addr, "/api/v1/discoveries", &json!(body)).await;
    assert_eq!(status, 200, "{created}");
    let discovery_id = created_id(&created);

    let body = scan(&own_daemon, vec![target(foreign.id)]);
    let (status, _) = post(mine.addr, "/api/v1/discoveries", &json!(body)).await;
    assert_eq!(status, 400, "create");
    let path = format!("/api/v1/discoveries/{discovery_id}");
    let (status, _) = put(mine.addr, &path, &json!(body)).await;
    assert!(!is_ok(status), "update got {status}");
}

#[tokio::test]
async fn dispatch_withholds_a_credential_from_another_organization() {
    let (state, _container) = cloud_test_state().await;
    let mine = tenant(&state, "a@example.com").await;
    let theirs = tenant(&state, "b@example.com").await;
    let own = stored_credential(&state, mine.org.id).await;
    let foreign = stored_credential(&state, theirs.org.id).await;
    let service = &state.services.credential_service;
    let placement = DaemonPlacement::default();

    let (with_own, _) = service
        .build_all_credential_mappings(
            mine.site.id,
            &[IntegrationTarget::Site {
                credential_id: own.id,
            }],
            None,
            &placement,
        )
        .await
        .unwrap();
    let (without, _) = service
        .build_all_credential_mappings(mine.site.id, &[], None, &placement)
        .await
        .unwrap();
    assert_eq!(with_own.len(), without.len() + 1, "the control dispatches");

    let (with_foreign, _) = service
        .build_all_credential_mappings(
            mine.site.id,
            &[IntegrationTarget::Site {
                credential_id: foreign.id,
            }],
            None,
            &placement,
        )
        .await
        .unwrap();
    assert_eq!(with_foreign.len(), without.len());
}

#[tokio::test]
async fn legacy_snmp_dispatch_withholds_a_credential_from_another_organization() {
    let (state, _container) = cloud_test_state().await;
    let mine = tenant(&state, "a@example.com").await;
    let theirs = tenant(&state, "b@example.com").await;
    let own_subnet = state
        .services
        .subnet_service
        .create(subnet(&mine.site.id), AuthenticatedEntity::System)
        .await
        .unwrap();
    let own_host = state
        .services
        .host_service
        .create(host(&mine.site.id), AuthenticatedEntity::System)
        .await
        .unwrap();
    let mut address = ip_address(&mine.site.id, &own_subnet.id);
    address.base.host_id = own_host.id;
    state
        .services
        .ip_address_service
        .create(address, AuthenticatedEntity::System)
        .await
        .unwrap();
    let foreign = stored_credential(&state, theirs.org.id).await;
    state
        .services
        .credential_service
        .set_host_credentials(
            &own_host.id,
            &[CredentialAssignment {
                credential_id: foreign.id,
                ip_address_ids: None,
            }],
        )
        .await
        .unwrap();

    let mapping = state
        .services
        .credential_service
        .build_snmp_credentials_for_discovery(mine.site.id)
        .await
        .unwrap();
    assert!(mapping.ip_overrides.is_empty());
}

#[tokio::test]
async fn provisioning_cannot_seed_another_organizations_credential() {
    let (state, _container) = cloud_test_state().await;
    let mine = tenant(&state, "a@example.com").await;
    let theirs = tenant(&state, "b@example.com").await;
    let own = stored_credential(&state, mine.org.id).await;
    let foreign = stored_credential(&state, theirs.org.id).await;
    let body = |credential_id: Uuid| {
        json!({
            "name": "edge",
            "site_id": mine.site.id,
            "seed_credential_refs": [{ "scope": "Site", "credential_id": credential_id }],
        })
    };

    let (status, created) = post(mine.addr, "/api/v1/daemons/provision", &body(own.id)).await;
    assert!(is_ok(status), "{status} {created}");
    let (status, _) = post(mine.addr, "/api/v1/daemons/provision", &body(foreign.id)).await;
    assert!(!is_ok(status), "got {status}");
}

#[tokio::test]
async fn an_admin_cannot_grant_a_site_in_another_organization() {
    let (state, _container) = cloud_test_state().await;
    let mine = tenant(&state, "a@example.com").await;
    let theirs = tenant(&state, "b@example.com").await;
    let mut row = user(&mine.org.id);
    row.base.email = "m@example.com".parse().unwrap();
    row.base.permissions = UserOrgPermissions::Member;
    let mut member = state
        .services
        .user_service
        .create(row, AuthenticatedEntity::System)
        .await
        .unwrap();
    let path = format!("/api/v1/users/{}/admin", member.id);

    member.base.site_ids = vec![mine.site.id];
    let (status, body) = put(mine.addr, &path, &json!(member)).await;
    assert_eq!(status, 200, "{body}");

    member.base.site_ids = vec![theirs.site.id];
    let (status, _) = put(mine.addr, &path, &json!(member)).await;
    assert_eq!(status, 400);
    let held = state.services.user_service.get_site_ids(&member.id).await;
    assert_eq!(held.unwrap(), vec![mine.site.id]);
}

#[tokio::test]
async fn a_host_cannot_name_another_organizations_credential() {
    let (state, _container) = cloud_test_state().await;
    let mine = tenant(&state, "a@example.com").await;
    let theirs = tenant(&state, "b@example.com").await;
    let own = stored_credential(&state, mine.org.id).await;
    let foreign = stored_credential(&state, theirs.org.id).await;
    let create = |credential_id: Uuid| {
        json!({
            "name": "router",
            "site_id": mine.site.id,
            "hostname": null,
            "description": null,
            "virtualization_metadata": null,
            "credential_assignments": [{ "credential_id": credential_id }],
        })
    };

    let (status, created) = post(mine.addr, "/api/v1/hosts", &create(own.id)).await;
    assert_eq!(status, 200, "{created}");
    let id = created_id(&created);
    let (status, _) = post(mine.addr, "/api/v1/hosts", &create(foreign.id)).await;
    assert_eq!(status, 400, "create");

    let update = |credential_id: Uuid| {
        json!({
            "id": id,
            "name": "router",
            "hostname": null,
            "description": null,
            "virtualization_metadata": null,
            "virtualization_service_id": null,
            "virtualization_interface_id": null,
            "tags": [],
            "credential_assignments": [{ "credential_id": credential_id }],
        })
    };
    let path = format!("/api/v1/hosts/{id}");
    let (status, body) = put(mine.addr, &path, &update(own.id)).await;
    assert_eq!(status, 200, "{body}");
    let (status, _) = put(mine.addr, &path, &update(foreign.id)).await;
    assert_eq!(status, 400, "update");
}

#[tokio::test]
async fn a_discovery_must_use_a_daemon_on_its_site() {
    let (state, _container) = cloud_test_state().await;
    let mine = tenant(&state, "a@example.com").await;
    let theirs = tenant(&state, "b@example.com").await;
    let own = stored_daemon(&state, &mine).await;
    let foreign = stored_daemon(&state, &theirs).await;

    let mut body = json!(scan(&own, vec![]));
    let (status, created) = post(mine.addr, "/api/v1/discoveries", &body).await;
    assert_eq!(status, 200, "{created}");
    let path = format!("/api/v1/discoveries/{}", created_id(&created));

    body["daemon_id"] = json!(foreign.id);
    let (status, _) = post(mine.addr, "/api/v1/discoveries", &body).await;
    assert_eq!(status, 400, "create");
    let (status, _) = put(mine.addr, &path, &body).await;
    assert_eq!(status, 400, "update");
}

#[tokio::test]
async fn a_daemon_key_cannot_bind_itself_to_a_daemon() {
    let (state, _container) = cloud_test_state().await;
    let mine = tenant(&state, "a@example.com").await;
    let theirs = tenant(&state, "b@example.com").await;
    let foreign = stored_daemon(&state, &theirs).await;

    let mut body = json!(DaemonApiKey::new(DaemonApiKeyBase {
        name: "bound".to_string(),
        site_id: mine.site.id,
        is_enabled: true,
        ..Default::default()
    }));
    body["daemon_id"] = json!(foreign.id);
    let (status, created) = post(mine.addr, "/api/v1/daemon-api-keys", &body).await;
    assert_eq!(status, 200, "{created}");
    let id: Uuid = serde_json::from_value(created["data"]["api_key"]["id"].clone()).unwrap();
    let stored = state
        .services
        .daemon_api_key_service
        .get_by_id(&id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.base.daemon_id, None);
}

/// The API served to a daemon authenticated on `site_id` as `daemon_id`.
async fn serve_as_daemon(state: &Arc<AppState>, site_id: Uuid, daemon_id: Uuid) -> SocketAddr {
    serve_as(
        state,
        AuthenticatedEntity::Daemon {
            site_id,
            api_key_id: Uuid::new_v4(),
            daemon_id,
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
        },
    )
    .await
}

fn registration(site_id: Uuid, daemon_id: Uuid) -> Value {
    json!({
        "daemon_id": daemon_id,
        "site_id": site_id,
        "name": "reporting",
        "mode": "daemon_poll",
        "capabilities": {},
        "user_id": Uuid::nil(),
        "version": env!("CARGO_PKG_VERSION"),
    })
}

#[tokio::test]
async fn registering_cannot_touch_a_daemon_on_another_site() {
    let (state, _container) = cloud_test_state().await;
    let mine = tenant(&state, "a@example.com").await;
    let theirs = tenant(&state, "b@example.com").await;
    let foreign = stored_daemon(&state, &theirs).await;

    let addr = serve_as_daemon(&state, mine.site.id, foreign.id).await;
    let body = registration(mine.site.id, foreign.id);
    let (status, _) = post(addr, "/api/daemons/register", &body).await;
    assert!(!is_ok(status), "got {status}");

    let stored = state
        .services
        .daemon_service
        .get_by_id(&foreign.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.base.last_seen, foreign.base.last_seen);
    assert_eq!(stored.base.name, foreign.base.name);
}

fn progress(
    session_id: Uuid,
    daemon_id: Uuid,
    site_id: Uuid,
    progress: u8,
) -> DiscoveryUpdatePayload {
    DiscoveryUpdatePayload {
        session_id,
        daemon_id,
        site_id,
        phase: DiscoveryPhase::Scanning,
        discovery_type: DiscoveryType::default(),
        progress,
        error: None,
        warnings: Vec::new(),
        credential_results: Vec::new(),
        started_at: None,
        finished_at: None,
        hosts_discovered: None,
        estimated_remaining_secs: None,
        discovery_id: None,
        scanned: None,
        reason: None,
        last_update_at: None,
        daemon_version: None,
    }
}

#[tokio::test]
async fn a_daemon_cannot_overwrite_a_session_on_another_site() {
    let (state, _container) = cloud_test_state().await;
    let mine = tenant(&state, "a@example.com").await;
    let theirs = tenant(&state, "b@example.com").await;
    let own_daemon = stored_daemon(&state, &mine).await;
    let foreign_daemon = stored_daemon(&state, &theirs).await;

    let session_id = Uuid::new_v4();
    state
        .services
        .discovery_service
        .update_session(progress(session_id, foreign_daemon.id, theirs.site.id, 10))
        .await;

    let addr = serve_as_daemon(&state, mine.site.id, own_daemon.id).await;
    let body = json!(progress(session_id, own_daemon.id, mine.site.id, 90));
    let path = format!("/api/v1/discovery/{session_id}/update");
    let (status, _) = post(addr, &path, &body).await;
    assert!(!is_ok(status), "got {status}");

    let stored = state
        .services
        .discovery_service
        .get_session(&session_id)
        .await
        .unwrap();
    assert_eq!(stored.site_id, theirs.site.id);
    assert_eq!(stored.progress, 10);
}

#[tokio::test]
async fn generic_writes_refuse_ids_from_another_organization() {
    let (state, _container) = cloud_test_state().await;
    let mine = tenant(&state, "a@example.com").await;
    let theirs = tenant(&state, "b@example.com").await;

    let services = &state.services;
    let system = || AuthenticatedEntity::System;
    let own_host = services
        .host_service
        .create(host(&mine.site.id), system())
        .await
        .unwrap();
    let own_service = services
        .service_service
        .create(service(&mine.site.id, &own_host.id), system())
        .await
        .unwrap();
    let foreign_host = services
        .host_service
        .create(host(&theirs.site.id), system())
        .await
        .unwrap();
    let foreign_service = services
        .service_service
        .create(service(&theirs.site.id, &foreign_host.id), system())
        .await
        .unwrap();
    let foreign_subnet = services
        .subnet_service
        .create(subnet(&theirs.site.id), system())
        .await
        .unwrap();

    // A binding on the caller's site naming a service on theirs.
    let binding = Binding::new(BindingBase::new(
        foreign_service.id,
        mine.site.id,
        BindingType::IPAddress {
            ip_address_id: Uuid::new_v4(),
        },
    ));
    let (status, _) = post(mine.addr, "/api/v1/bindings", &json!(binding)).await;
    assert_eq!(status, 400, "binding service_id");

    // A service containerized by a service on theirs.
    let mut containerized = json!(service(&mine.site.id, &own_host.id));
    containerized["bindings"] = json!([]);
    containerized["virtualization_service_id"] = json!(own_service.id);
    let (status, body) = post(mine.addr, "/api/v1/services", &containerized).await;
    assert_eq!(status, 200, "{body}");
    containerized["virtualization_service_id"] = json!(foreign_service.id);
    let (status, _) = post(mine.addr, "/api/v1/services", &containerized).await;
    assert_eq!(status, 400, "service virtualization_service_id");

    // A subnet managed by a service on theirs.
    let mut managed = subnet(&mine.site.id);
    managed.base.cidr = theirs_cidr();
    managed.base.virtualization_service_id = Some(foreign_service.id);
    let (status, _) = post(mine.addr, "/api/v1/subnets", &json!(managed)).await;
    assert_eq!(status, 400, "subnet virtualization_service_id");
    managed.base.virtualization_service_id = Some(own_service.id);
    let (status, body) = post(mine.addr, "/api/v1/subnets", &json!(managed)).await;
    assert_eq!(status, 200, "{body}");

    // A scan of a subnet on theirs.
    let own_daemon = stored_daemon(&state, &mine).await;
    let own_scan = json!(scan(&own_daemon, vec![]));
    let (status, created) = post(mine.addr, "/api/v1/discoveries", &own_scan).await;
    assert_eq!(status, 200, "{created}");
    let mut foreign_scan = own_scan.clone();
    foreign_scan["discovery_type"]["subnet_ids"] = json!([foreign_subnet.id]);
    let path = format!("/api/v1/discoveries/{}", created_id(&created));
    let (status, _) = put(mine.addr, &path, &foreign_scan).await;
    assert_eq!(status, 400, "discovery subnet_ids");

    // A daemon maintained by a user from theirs.
    let mut maintained = json!(own_daemon);
    maintained["user_id"] = json!(theirs.owner.id);
    let path = format!("/api/v1/daemons/{}", own_daemon.id);
    let (status, _) = put(mine.addr, &path, &maintained).await;
    assert_eq!(status, 400, "daemon user_id");
}

#[tokio::test]
async fn a_daemon_cannot_claim_a_subnet_on_another_site() {
    let (state, _container) = cloud_test_state().await;
    let mine = tenant(&state, "a@example.com").await;
    let theirs = tenant(&state, "b@example.com").await;
    let own_daemon = stored_daemon(&state, &mine).await;
    let own_subnet = state
        .services
        .subnet_service
        .create(subnet(&mine.site.id), AuthenticatedEntity::System)
        .await
        .unwrap();
    let foreign_subnet = state
        .services
        .subnet_service
        .create(subnet(&theirs.site.id), AuthenticatedEntity::System)
        .await
        .unwrap();

    let addr = serve_as_daemon(&state, mine.site.id, own_daemon.id).await;
    let body = json!({ "interfaced_subnet_ids": [own_subnet.id, foreign_subnet.id] });
    let path = format!("/api/daemons/{}/update-capabilities", own_daemon.id);
    let (status, reply) = post(addr, &path, &body).await;
    assert_eq!(status, 200, "{reply}");

    let interfaced = state
        .services
        .daemon_service
        .get_interfaced_subnet_ids(&own_daemon.id)
        .await;
    assert_eq!(interfaced, vec![own_subnet.id]);
}

#[tokio::test]
async fn a_discovered_host_cannot_rewrite_an_address_on_another_site() {
    let (state, _container) = cloud_test_state().await;
    let mine = tenant(&state, "a@example.com").await;
    let theirs = tenant(&state, "b@example.com").await;
    let own_daemon = stored_daemon(&state, &mine).await;
    let services = &state.services;
    let system = || AuthenticatedEntity::System;
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
    let mut foreign_address = ip_address(&theirs.site.id, &foreign_subnet.id);
    foreign_address.base.host_id = foreign_host.id;
    let foreign_address = services
        .ip_address_service
        .create(foreign_address, system())
        .await
        .unwrap();

    let reported_host = host(&mine.site.id);
    let mut reported = json!(foreign_address);
    reported["site_id"] = json!(mine.site.id);
    reported["host_id"] = json!(reported_host.id);
    reported["subnet_id"] = json!(own_subnet.id);
    reported["last_seen_at"] = json!(chrono::Utc::now() + chrono::Duration::hours(1));
    let body = json!({
        "host": reported_host,
        "ip_addresses": [reported],
        "ports": [],
        "services": [],
    });

    let addr = serve_as_daemon(&state, mine.site.id, own_daemon.id).await;
    post(addr, "/api/v1/hosts/discovery", &body).await;

    let stored = services
        .ip_address_service
        .get_by_id(&foreign_address.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.base.site_id, theirs.site.id);
    assert_eq!(stored.last_seen_at, foreign_address.last_seen_at);
}

#[tokio::test]
async fn saving_a_host_cannot_name_rows_from_another_organization() {
    let (state, _container) = cloud_test_state().await;
    let mine = tenant(&state, "a@example.com").await;
    let theirs = tenant(&state, "b@example.com").await;
    let services = &state.services;
    let system = || AuthenticatedEntity::System;
    let foreign_host = services
        .host_service
        .create(host(&theirs.site.id), system())
        .await
        .unwrap();
    let foreign_service = services
        .service_service
        .create(service(&theirs.site.id, &foreign_host.id), system())
        .await
        .unwrap();
    let foreign_subnet = services
        .subnet_service
        .create(subnet(&theirs.site.id), system())
        .await
        .unwrap();

    let create = json!({
        "name": "vm",
        "site_id": mine.site.id,
        "hostname": null,
        "description": null,
        "virtualization_metadata": null,
    });
    let (status, created) = post(mine.addr, "/api/v1/hosts", &create).await;
    assert_eq!(status, 200, "{created}");
    let id = created_id(&created);

    let mut virtualized = create.clone();
    virtualized["virtualization_service_id"] = json!(foreign_service.id);
    let (status, _) = post(mine.addr, "/api/v1/hosts", &virtualized).await;
    assert_eq!(status, 400, "create virtualization_service_id");

    let update = json!({
        "id": id,
        "name": "vm",
        "hostname": null,
        "description": null,
        "virtualization_metadata": null,
        "virtualization_service_id": null,
        "virtualization_interface_id": null,
        "tags": [],
    });
    let path = format!("/api/v1/hosts/{id}");
    let (status, body) = put(mine.addr, &path, &update).await;
    assert_eq!(status, 200, "{body}");

    let mut virtualized = update.clone();
    virtualized["virtualization_service_id"] = json!(foreign_service.id);
    let (status, _) = put(mine.addr, &path, &virtualized).await;
    assert_eq!(status, 400, "update virtualization_service_id");

    let foreign_address = {
        let mut row = ip_address(&theirs.site.id, &foreign_subnet.id);
        row.base.host_id = foreign_host.id;
        services
            .ip_address_service
            .create(row, system())
            .await
            .unwrap()
    };
    let mut interfaced = update.clone();
    interfaced["interfaces"] = json!([{
        "id": Uuid::new_v4(),
        "if_index": 1,
        "if_descr": "eth0",
        "ip_address_id": foreign_address.id,
    }]);
    let (status, _) = put(mine.addr, &path, &interfaced).await;
    assert!(
        !is_ok(status),
        "update interfaces ip_address_id got {status}"
    );

    let mut addressed = update.clone();
    addressed["ip_addresses"] = json!([{
        "id": Uuid::new_v4(),
        "subnet_id": foreign_subnet.id,
        "ip_address": "192.168.1.9",
    }]);
    let (status, _) = put(mine.addr, &path, &addressed).await;
    assert_eq!(status, 400, "update ip_addresses subnet_id");
}

fn theirs_cidr() -> crate::server::subnets::r#impl::base::SubnetCidr {
    crate::server::subnets::r#impl::base::SubnetCidr::new(
        crate::server::subnets::r#impl::base::SubnetCidrValue("10.88.0.0/24".parse().unwrap()),
        crate::server::shared::attribution::AttributeSource::DaemonSelfReport,
    )
}

#[tokio::test]
async fn an_interface_cannot_name_a_vlan_from_another_organization() {
    let (state, _container) = cloud_test_state().await;
    let mine = tenant(&state, "a@example.com").await;
    let theirs = tenant(&state, "b@example.com").await;
    let own_host = state
        .services
        .host_service
        .create(host(&mine.site.id), AuthenticatedEntity::System)
        .await
        .unwrap();
    let foreign_vlan: Vlan = {
        let mut row = Vlan::default();
        row.id = Uuid::new_v4();
        row.base.vlan_number = 42;
        row.base.site_id = theirs.site.id;
        row.base.organization_id = theirs.org.id;
        state
            .services
            .vlan_service
            .create(row, AuthenticatedEntity::System)
            .await
            .unwrap()
    };

    let body = json!({
        "host_id": own_host.id,
        "site_id": mine.site.id,
        "native_vlan_id": foreign_vlan.id,
    });
    let (status, _) = post(mine.addr, "/api/v1/interfaces", &body).await;
    assert_eq!(status, 400);
}
