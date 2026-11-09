//! What the create-daemon wizard reads before its first step: every limit on a new daemon, and,
//! while one applies, the caller's daemons that have never connected.

use uuid::Uuid;

use super::email_verification::{
    create_org_on, create_site, create_user, provision_request, session_entity, state_with_email,
};
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::billing::plans::{get_enterprise_plan, get_free_plan};
use crate::server::config::AppState;
use crate::server::daemons::r#impl::base::Daemon;
use crate::server::shared::events::types::UpdateReason;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::types::error_codes::ErrorCode;
use crate::server::users::r#impl::base::User;
use crate::server::users::r#impl::permissions::UserOrgPermissions;

/// Provision a daemon on `site_id` as `owner`. It has not connected yet.
async fn provision(state: &AppState, owner: &User, site_id: Uuid, name: &str) -> Daemon {
    state
        .services
        .daemon_service
        .provision(
            &provision_request(site_id, name),
            None,
            session_entity(owner, vec![site_id]),
        )
        .await
        .expect("the first daemon is always allowed")
        .0
}

async fn mark_connected(state: &AppState, mut daemon: Daemon) {
    daemon.base.last_seen = Some(crate::server::shared::clock::now());
    state
        .services
        .daemon_service
        .update(
            &mut daemon,
            AuthenticatedEntity::System,
            UpdateReason::EntityRequested,
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn refused_org_lists_its_daemon_that_never_connected() {
    let (state, _container, _dir) = state_with_email().await;
    let org = create_org_on(&state, get_free_plan()).await;
    let owner = create_user(
        &state,
        org.id,
        "o@example.com",
        UserOrgPermissions::Owner,
        true,
    )
    .await;
    let site = create_site(&state, org.id).await;
    let daemon = provision(&state, &owner, site.id, "first").await;

    let eligibility = state
        .services
        .daemon_service
        .provision_eligibility(org.id, &[site.id])
        .await
        .unwrap();

    assert!(eligibility.plan_required);
    let listed: Vec<Uuid> = eligibility
        .unconnected_daemons
        .iter()
        .map(|d| d.id)
        .collect();
    assert_eq!(listed, vec![daemon.id]);
}

#[tokio::test]
async fn list_is_empty_when_no_limit_applies() {
    let (state, _container, _dir) = state_with_email().await;
    let org = create_org_on(&state, get_enterprise_plan()).await;
    let owner = create_user(
        &state,
        org.id,
        "o@example.com",
        UserOrgPermissions::Owner,
        true,
    )
    .await;
    let site = create_site(&state, org.id).await;
    provision(&state, &owner, site.id, "first").await;

    let eligibility = state
        .services
        .daemon_service
        .provision_eligibility(org.id, &[site.id])
        .await
        .unwrap();

    assert!(!eligibility.plan_required && !eligibility.email_verification_required);
    assert!(eligibility.unconnected_daemons.is_empty());
}

#[tokio::test]
async fn a_daemon_that_has_connected_is_not_listed() {
    let (state, _container, _dir) = state_with_email().await;
    let org = create_org_on(&state, get_free_plan()).await;
    let owner = create_user(
        &state,
        org.id,
        "o@example.com",
        UserOrgPermissions::Owner,
        true,
    )
    .await;
    let site = create_site(&state, org.id).await;
    let daemon = provision(&state, &owner, site.id, "first").await;
    mark_connected(&state, daemon).await;

    let eligibility = state
        .services
        .daemon_service
        .provision_eligibility(org.id, &[site.id])
        .await
        .unwrap();

    assert!(eligibility.plan_required);
    assert!(eligibility.unconnected_daemons.is_empty());
}

#[tokio::test]
async fn a_daemon_outside_the_callers_sites_is_not_listed() {
    let (state, _container, _dir) = state_with_email().await;
    let org = create_org_on(&state, get_free_plan()).await;
    let owner = create_user(
        &state,
        org.id,
        "o@example.com",
        UserOrgPermissions::Owner,
        true,
    )
    .await;
    let daemon_site = create_site(&state, org.id).await;
    let member_site = create_site(&state, org.id).await;
    provision(&state, &owner, daemon_site.id, "first").await;

    let eligibility = state
        .services
        .daemon_service
        .provision_eligibility(org.id, &[member_site.id])
        .await
        .unwrap();

    assert!(eligibility.plan_required);
    assert!(eligibility.unconnected_daemons.is_empty());
}

/// A Free org with no verified owner learns about both steps before it upgrades. The server
/// still refuses with the plan code first, as it did before.
#[tokio::test]
async fn free_org_with_no_verified_owner_reports_both_limits() {
    let (state, _container, _dir) = state_with_email().await;
    let org = create_org_on(&state, get_free_plan()).await;
    let owner = create_user(
        &state,
        org.id,
        "o@example.com",
        UserOrgPermissions::Owner,
        false,
    )
    .await;
    let site = create_site(&state, org.id).await;
    provision(&state, &owner, site.id, "first").await;

    let daemons = &state.services.daemon_service;
    let eligibility = daemons
        .provision_eligibility(org.id, &[site.id])
        .await
        .unwrap();
    assert!(eligibility.plan_required && eligibility.email_verification_required);

    let refused = daemons.check_daemon_limit(org.id).await.unwrap_err();
    assert_eq!(
        refused.error_code.as_ref().map(ErrorCode::code),
        Some(ErrorCode::DaemonPlanRequired.code())
    );
}
