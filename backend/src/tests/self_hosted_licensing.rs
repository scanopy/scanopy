//! Self-hosted plans bought through the cloud: the entitlement endpoint, the
//! billing middleware's lock on main-app routes, and the writers of
//! `license_paid_through`.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::middleware::{self, Next};
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Duration, Utc};
use email_address::EmailAddress;
use testcontainers::{ContainerAsync, GenericImage};
use uuid::Uuid;

use super::{organization, setup_test_db, user};
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::auth::middleware::billing::require_billing_for_users;
use crate::server::billing::plans::{
    get_enterprise_plan, get_self_hosted_plus_plan, get_self_hosted_standard_plan,
};
use crate::server::billing::types::base::{
    BillingInvoice, BillingInvoiceLineItem, BillingPlan, BillingReason, InvoiceCollection,
    PlanStatus,
};
use crate::server::config::{AppState, ServerConfig};
use crate::server::license::handlers::{get_current_license_key, get_entitlement};
use crate::server::license::key::LicenseKey;
use crate::server::license::mint::tests::{test_decoding_key, test_issuer};
use crate::server::license::mint::{GRACE_PERIOD_DAYS, MintError, PAID_THROUGH_BUFFER_DAYS};
use crate::server::license::online::{ENTITLEMENT_PATH, EntitlementRequest};
use crate::server::license::types::{LicenseKeyType, LicenseStatus};
use crate::server::organizations::r#impl::base::Organization;
use crate::server::organizations::service::SwitchKeyTypeError;
use crate::server::shared::events::traits::{Event, OrgScope, Subscriber};
use crate::server::shared::events::types::BillingOperation;
use crate::server::shared::services::traits::CrudService;
use crate::server::users::r#impl::permissions::UserOrgPermissions;

/// App state on a fresh database, billing enforced, signing with the test key.
async fn test_state() -> (Arc<AppState>, ContainerAsync<GenericImage>) {
    test_state_with_email_dir(None).await
}

/// As [`test_state`], but with emails written to `email_log_dir` when one is
/// given, so a test can read what a subscriber actually rendered.
async fn test_state_with_email_dir(
    email_log_dir: Option<PathBuf>,
) -> (Arc<AppState>, ContainerAsync<GenericImage>) {
    let (pool, database_url, container) = setup_test_db().await;
    pool.close().await;
    let config = ServerConfig {
        database_url,
        enforce_billing_for_testing: true,
        email_log_dir,
        ..ServerConfig::default()
    };
    let mut state = Arc::try_unwrap(AppState::new(config).await.unwrap())
        .ok()
        .expect("state is not shared yet");
    state.license_issuer = Some(Arc::new(test_issuer()));
    (Arc::new(state), container)
}

async fn create_org(
    state: &AppState,
    plan: BillingPlan,
    paid_through: Option<DateTime<Utc>>,
) -> Organization {
    let mut org = organization();
    org.base.plan = Some(plan);
    org.base.plan_status = Some(PlanStatus::Trialing);
    org.base.license_paid_through = paid_through;
    state
        .services
        .organization_service
        .create(org, AuthenticatedEntity::System)
        .await
        .unwrap()
}

async fn set_plan(state: &AppState, organization_id: Uuid, plan: BillingPlan) {
    let service = &state.services.organization_service;
    let mut org = service.get_by_id(&organization_id).await.unwrap().unwrap();
    org.base.plan = Some(plan);
    service
        .update(&mut org, AuthenticatedEntity::System)
        .await
        .unwrap();
}

async fn reload(state: &AppState, organization_id: Uuid) -> Organization {
    state
        .services
        .organization_service
        .get_by_id(&organization_id)
        .await
        .unwrap()
        .unwrap()
}

/// Postgres stores microseconds; keep test timestamps on whole seconds so
/// they round-trip exactly.
fn whole_seconds_from_now(days: i64) -> DateTime<Utc> {
    DateTime::from_timestamp((Utc::now() + Duration::days(days)).timestamp(), 0).unwrap()
}

async fn request_entitlement(state: &Arc<AppState>, key: String) -> Result<String, StatusCode> {
    get_entitlement(State(state.clone()), Json(EntitlementRequest { key }))
        .await
        .map(|Json(response)| response.into_data().unwrap().entitlement)
        .map_err(|error| error.status)
}

#[test]
fn entitlement_route_is_served_at_the_shared_contract_path() {
    let openapi = crate::server::shared::handlers::factory::collect_all_openapi_routes();
    assert!(openapi.paths.paths.contains_key(ENTITLEMENT_PATH));
}

#[tokio::test]
async fn online_keys_are_stable_until_rotated() {
    let (state, _container) = test_state().await;
    let issuer = state.license_issuer.clone().unwrap();
    let service = &state.services.organization_service;
    let org = create_org(&state, get_self_hosted_standard_plan(), None).await;

    // What two clicks of Copy do: one issued-at stamp, one key string.
    let first_stamp = service.license_key_issued_at(org.id).await.unwrap();
    let second_stamp = service.license_key_issued_at(org.id).await.unwrap();
    assert_eq!(first_stamp, second_stamp);
    let key = issuer
        .mint_online_key(&reload(&state, org.id).await, first_stamp)
        .unwrap();
    assert_eq!(
        issuer
            .mint_online_key(&reload(&state, org.id).await, second_stamp)
            .unwrap(),
        key
    );

    // Rotating bumps the key version, so the next copy is a different key.
    // The stamp is whole seconds, so it can still read the same within one.
    service
        .rotate_license_key(org.id, AuthenticatedEntity::System)
        .await
        .unwrap();
    let rotated_stamp = service.license_key_issued_at(org.id).await.unwrap();
    assert!(rotated_stamp >= first_stamp);
    assert_ne!(
        issuer
            .mint_online_key(&reload(&state, org.id).await, rotated_stamp)
            .unwrap(),
        key
    );
}

#[tokio::test]
async fn air_gapped_orgs_cannot_return_to_an_online_key_until_the_period_ends() {
    let (state, _container) = test_state().await;
    let service = &state.services.organization_service;
    let org = create_org(
        &state,
        get_self_hosted_plus_plan(),
        Some(whole_seconds_from_now(365)),
    )
    .await;

    service
        .switch_license_key_type(
            org.id,
            LicenseKeyType::Offline,
            AuthenticatedEntity::System,
            |_| Ok(()),
        )
        .await
        .unwrap();

    // The key already installed runs to its embedded expiry, so going back now
    // would leave two live keys.
    assert!(matches!(
        service
            .switch_license_key_type(
                org.id,
                LicenseKeyType::Online,
                AuthenticatedEntity::System,
                |_| Ok(())
            )
            .await,
        Err(SwitchKeyTypeError::AirGappedStillCurrent { .. })
    ));
    assert_eq!(
        reload(&state, org.id).await.base.license_key_type,
        Some(LicenseKeyType::Offline)
    );

    // Once the period is up, the switch is allowed.
    let mut expired = reload(&state, org.id).await;
    expired.base.license_paid_through = Some(whole_seconds_from_now(-1));
    service
        .update(&mut expired, AuthenticatedEntity::System)
        .await
        .unwrap();
    service
        .switch_license_key_type(
            org.id,
            LicenseKeyType::Online,
            AuthenticatedEntity::System,
            |_| Ok(()),
        )
        .await
        .unwrap();
    assert_eq!(
        reload(&state, org.id).await.base.license_key_type,
        Some(LicenseKeyType::Online)
    );
}

#[tokio::test]
async fn entitlement_endpoint_accepts_current_keys_and_rejects_the_rest() {
    let (state, _container) = test_state().await;
    let issuer = state.license_issuer.clone().unwrap();
    let service = &state.services.organization_service;
    let paid_through = whole_seconds_from_now(30);
    let org = create_org(&state, get_self_hosted_standard_plan(), Some(paid_through)).await;
    let key = issuer.mint_online_key(&org, Utc::now()).unwrap();

    // 200: an entitlement for the org's plan and paid-through date, and the
    // check-in is recorded.
    let entitlement = request_entitlement(&state, key.clone()).await.unwrap();
    let LicenseStatus::Valid(claims) =
        LicenseKey::new(entitlement).validate_with(&test_decoding_key())
    else {
        panic!("entitlement should validate");
    };
    assert_eq!(claims.org_id, Some(org.id.to_string()));
    assert_eq!(
        claims.intended_exp,
        (paid_through + Duration::days(PAID_THROUGH_BUFFER_DAYS)).timestamp()
    );
    assert!(
        reload(&state, org.id)
            .await
            .base
            .license_checkin_at
            .is_some()
    );

    // 400: not a key at all, and an offline-format token presented as an
    // online key.
    assert_eq!(
        request_entitlement(&state, "not-a-key".to_string()).await,
        Err(StatusCode::BAD_REQUEST)
    );
    let offline_format = issuer.mint_entitlement(&org, Utc::now()).unwrap();
    assert_eq!(
        request_entitlement(&state, offline_format).await,
        Err(StatusCode::BAD_REQUEST)
    );

    // 403: the key was rotated. The new key works.
    service
        .rotate_license_key(org.id, AuthenticatedEntity::System)
        .await
        .unwrap();
    assert_eq!(
        request_entitlement(&state, key).await,
        Err(StatusCode::FORBIDDEN)
    );
    let current_key = issuer
        .mint_online_key(&reload(&state, org.id).await, Utc::now())
        .unwrap();
    assert!(
        request_entitlement(&state, current_key.clone())
            .await
            .is_ok()
    );

    // 403: the org moved to a cloud plan.
    set_plan(&state, org.id, get_enterprise_plan()).await;
    assert_eq!(
        request_entitlement(&state, current_key).await,
        Err(StatusCode::FORBIDDEN)
    );
}

async fn get_status(addr: SocketAddr, path: &str) -> (u16, String) {
    let response = reqwest::get(format!("http://{addr}{path}")).await.unwrap();
    (response.status().as_u16(), response.text().await.unwrap())
}

#[tokio::test]
async fn billing_middleware_locks_main_app_routes_for_self_hosted_plans() {
    let (state, _container) = test_state().await;
    let org = create_org(&state, get_self_hosted_standard_plan(), None).await;
    let owner = AuthenticatedEntity::User {
        user_id: Uuid::new_v4(),
        organization_id: org.id,
        permissions: UserOrgPermissions::Owner,
        network_ids: vec![],
        email: EmailAddress::new_unchecked("owner@example.com"),
        email_verified: true,
    };

    // A main-app route and a Settings route behind the billing middleware,
    // with the owner already authenticated (the extractor reads the cached
    // entity from request extensions).
    let app = Router::new()
        .route("/api/v1/hosts", get(|| async { "ok" }))
        .route("/api/v1/organizations", get(|| async { "ok" }))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_billing_for_users,
        ))
        .layer(middleware::from_fn(
            move |mut request: Request<Body>, next: Next| {
                let owner = owner.clone();
                async move {
                    request.extensions_mut().insert(owner);
                    next.run(request).await
                }
            },
        ))
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let (status, body) = get_status(addr, "/api/v1/hosts").await;
    assert_eq!(status, 403);
    assert!(body.contains("billing_self_hosted_plan_locked"));
    assert_eq!(get_status(addr, "/api/v1/organizations").await.0, 200);

    // Switching to a cloud plan unlocks the main app; switching back locks it.
    set_plan(&state, org.id, get_enterprise_plan()).await;
    assert_eq!(get_status(addr, "/api/v1/hosts").await.0, 200);
    set_plan(&state, org.id, get_self_hosted_standard_plan()).await;
    assert_eq!(get_status(addr, "/api/v1/hosts").await.0, 403);
}

#[tokio::test]
async fn switching_key_type_retires_the_previous_key() {
    let (state, _container) = test_state().await;
    let issuer = state.license_issuer.clone().unwrap();
    let service = &state.services.organization_service;
    let org = create_org(
        &state,
        get_self_hosted_plus_plan(),
        Some(whole_seconds_from_now(365)),
    )
    .await;

    let issued_at = service.license_key_issued_at(org.id).await.unwrap();
    let online_key = issuer
        .mint_online_key(&reload(&state, org.id).await, issued_at)
        .unwrap();
    assert!(
        request_entitlement(&state, online_key.clone())
            .await
            .is_ok()
    );

    // Switching to air-gapped retires the online key.
    service
        .switch_license_key_type(
            org.id,
            LicenseKeyType::Offline,
            AuthenticatedEntity::System,
            |_| Ok(()),
        )
        .await
        .unwrap();
    assert_eq!(
        reload(&state, org.id).await.base.license_key_type,
        Some(LicenseKeyType::Offline)
    );
    assert_eq!(
        request_entitlement(&state, online_key).await,
        Err(StatusCode::FORBIDDEN)
    );

    // Asking for the type already issued is a re-read, not another switch.
    let version = reload(&state, org.id).await.base.license_key_version;
    service
        .switch_license_key_type(
            org.id,
            LicenseKeyType::Offline,
            AuthenticatedEntity::System,
            |_| Ok(()),
        )
        .await
        .unwrap();
    assert_eq!(
        reload(&state, org.id).await.base.license_key_version,
        version
    );
}

#[tokio::test]
async fn a_refused_key_type_leaves_the_organization_where_it_was() {
    let (state, _container) = test_state().await;
    let service = &state.services.organization_service;
    let org = create_org(
        &state,
        get_self_hosted_plus_plan(),
        Some(whole_seconds_from_now(365)),
    )
    .await;
    let before = reload(&state, org.id).await;

    // The switch retires the key the org is running and the version bump
    // cannot be undone, so a refusal has to abandon the switch rather than
    // persist a type the org cannot be issued. Otherwise every later read
    // mints that stored type and fails.
    let refused = service
        .switch_license_key_type(
            org.id,
            LicenseKeyType::Offline,
            AuthenticatedEntity::System,
            |_| Err(MintError::OfflineRequiresPayment),
        )
        .await;
    assert!(matches!(refused, Err(SwitchKeyTypeError::Mint(_))));

    let after = reload(&state, org.id).await;
    assert_eq!(after.base.license_key_type, before.base.license_key_type);
    assert_eq!(
        after.base.license_key_version,
        before.base.license_key_version
    );
}

#[tokio::test]
async fn license_paid_through_follows_self_hosted_trials_and_invoices() {
    let (state, _container) = test_state().await;
    let service = &state.services.organization_service;
    let self_hosted = create_org(&state, get_self_hosted_standard_plan(), None).await;
    let cloud = create_org(&state, get_enterprise_plan(), None).await;
    let trial_end = whole_seconds_from_now(30);

    let trial_started = |organization_id: Uuid, plan: BillingPlan| {
        Event::new(
            OrgScope { organization_id },
            BillingOperation::TrialStarted {
                plan,
                trial_end,
                trial_days: 14,
            },
            AuthenticatedEntity::System,
        )
    };
    service
        .handle(vec![
            trial_started(self_hosted.id, get_self_hosted_standard_plan()),
            trial_started(cloud.id, get_enterprise_plan()),
        ])
        .await
        .unwrap();
    assert_eq!(
        reload(&state, self_hosted.id)
            .await
            .base
            .license_paid_through,
        Some(trial_end)
    );
    assert_eq!(
        reload(&state, cloud.id).await.base.license_paid_through,
        None
    );

    // The first annual invoice after the trial moves it to the period end.
    let period_end = whole_seconds_from_now(395);
    let invoice = BillingInvoice {
        stripe_invoice_id: "in_test".to_string(),
        amount_paid_cents: 400_000,
        currency: "usd".to_string(),
        created_at: trial_end,
        period_start: trial_end,
        period_end: trial_end,
        billing_reason: BillingReason::SubscriptionCycle,
        line_items: vec![BillingInvoiceLineItem {
            description: None,
            amount_cents: 400_000,
            period_start: trial_end,
            period_end,
            product: Some(get_self_hosted_standard_plan().stripe_product_id()),
        }],
        invoice_pdf: None,
        hosted_invoice_url: None,
        collection: InvoiceCollection::ChargeAutomatically,
        due_date: None,
    };
    service
        .handle(vec![Event::new(
            OrgScope {
                organization_id: self_hosted.id,
            },
            BillingOperation::PaymentSucceeded {
                invoice,
                previous_license_paid_through: None,
            },
            AuthenticatedEntity::System,
        )])
        .await
        .unwrap();
    assert_eq!(
        reload(&state, self_hosted.id)
            .await
            .base
            .license_paid_through,
        Some(period_end)
    );
}

#[tokio::test]
async fn a_stranded_air_gapped_organization_heals_on_read() {
    let (state, _container) = test_state().await;
    let service = &state.services.organization_service;
    // Trialing with no card, so an air-gapped key is refused with
    // `OfflineRequiresPayment` — the state a declined charge leaves behind.
    let org = create_org(
        &state,
        get_self_hosted_plus_plan(),
        Some(whole_seconds_from_now(365)),
    )
    .await;
    let mut stranded = reload(&state, org.id).await;
    stranded.base.license_key_type = Some(LicenseKeyType::Offline);
    service
        .update(&mut stranded, AuthenticatedEntity::System)
        .await
        .unwrap();

    let owner = AuthenticatedEntity::User {
        user_id: Uuid::new_v4(),
        organization_id: org.id,
        permissions: UserOrgPermissions::Owner,
        network_ids: vec![],
        email: EmailAddress::new_unchecked("owner@example.com"),
        email_verified: true,
    };
    // The extractor takes the entity cached in request extensions, so the
    // route can be served without a session.
    let app = Router::new()
        .route("/keys/current", get(get_current_license_key))
        .layer(middleware::from_fn(
            move |mut request: Request<Body>, next: Next| {
                let owner = owner.clone();
                async move {
                    request.extensions_mut().insert(owner);
                    next.run(request).await
                }
            },
        ))
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    // Minting the stored type would 403 forever, and the switch back is
    // refused while paid-through is ahead. The read repairs the row instead.
    let (status, body) = get_status(addr, "/keys/current").await;
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("Online"), "{body}");
    assert_eq!(
        reload(&state, org.id).await.base.license_key_type,
        Some(LicenseKeyType::Online)
    );
}

#[tokio::test]
async fn the_airgap_renewal_email_dates_the_key_already_installed() {
    let dir = tempfile::tempdir().unwrap();
    let (state, _container) = test_state_with_email_dir(Some(dir.path().to_path_buf())).await;
    let service = &state.services.organization_service;

    // An air-gapped org part-way through a licence period, with an owner for
    // the email to reach.
    let paid_through_before = whole_seconds_from_now(5);
    let org = create_org(
        &state,
        get_self_hosted_plus_plan(),
        Some(paid_through_before),
    )
    .await;
    let mut air_gapped = reload(&state, org.id).await;
    air_gapped.base.license_key_type = Some(LicenseKeyType::Offline);
    service
        .update(&mut air_gapped, AuthenticatedEntity::System)
        .await
        .unwrap();

    let mut owner = user(&org.id);
    owner.base.email = EmailAddress::new_unchecked("owner@example.test");
    owner.base.permissions = UserOrgPermissions::Owner;
    owner.base.email_verified = true;
    state
        .services
        .user_service
        .create(owner, AuthenticatedEntity::System)
        .await
        .unwrap();

    // The renewal moves paid-through a year out. The key on their server
    // predates it, so the email has to name the old date plus the buffer and
    // grace, never the renewed one.
    let renewed_through = whole_seconds_from_now(370);
    let invoice = BillingInvoice {
        stripe_invoice_id: "in_renewal".to_string(),
        amount_paid_cents: 400_000,
        currency: "usd".to_string(),
        created_at: paid_through_before,
        period_start: paid_through_before,
        period_end: renewed_through,
        billing_reason: BillingReason::SubscriptionCycle,
        line_items: vec![BillingInvoiceLineItem {
            description: None,
            amount_cents: 400_000,
            period_start: paid_through_before,
            period_end: renewed_through,
            product: Some(get_self_hosted_plus_plan().stripe_product_id()),
        }],
        invoice_pdf: None,
        hosted_invoice_url: None,
        collection: InvoiceCollection::ChargeAutomatically,
        due_date: None,
    };

    // Driven straight at the email subscriber. The date rides on the event
    // precisely so it no longer depends on which subscriber ran first, and
    // the organization subscriber is deliberately not run here.
    let email_service = state.services.email_service.clone().unwrap();
    email_service
        .handle(vec![Event::new(
            OrgScope {
                organization_id: org.id,
            },
            BillingOperation::PaymentSucceeded {
                invoice,
                previous_license_paid_through: Some(paid_through_before),
            },
            AuthenticatedEntity::System,
        )])
        .await
        .unwrap();

    let buffer_and_grace = Duration::days(PAID_THROUGH_BUFFER_DAYS + GRACE_PERIOD_DAYS);
    let sent = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| std::fs::read_to_string(entry.unwrap().path()).unwrap())
        .collect::<Vec<_>>()
        .join("\n");

    let installed_key_expiry = (paid_through_before + buffer_and_grace)
        .format("%B %-d, %Y")
        .to_string();
    assert!(
        sent.contains(&installed_key_expiry),
        "the installed key should be dated from the previous paid-through ({installed_key_expiry}): {sent}"
    );
    let renewed_expiry = (renewed_through + buffer_and_grace)
        .format("%B %-d, %Y")
        .to_string();
    assert!(
        !sent.contains(&renewed_expiry),
        "the installed key's expiry must not be computed from the renewed period ({renewed_expiry}): {sent}"
    );
}

#[tokio::test]
async fn sent_invoices_license_until_due_and_give_back_on_void() {
    let (state, _container) = test_state().await;
    let service = &state.services.organization_service;
    let trial_end = whole_seconds_from_now(3);
    let org = create_org(&state, get_self_hosted_standard_plan(), Some(trial_end)).await;
    let due = whole_seconds_from_now(33);

    let sent = |period_start: DateTime<Utc>, due_date: DateTime<Utc>| BillingInvoice {
        stripe_invoice_id: "in_sent".to_string(),
        amount_paid_cents: 0,
        currency: "usd".to_string(),
        created_at: period_start,
        period_start,
        period_end: period_start,
        billing_reason: BillingReason::SubscriptionCycle,
        line_items: vec![BillingInvoiceLineItem {
            description: None,
            amount_cents: 400_000,
            period_start,
            period_end: period_start + Duration::days(365),
            product: Some(get_self_hosted_standard_plan().stripe_product_id()),
        }],
        invoice_pdf: None,
        hosted_invoice_url: None,
        collection: InvoiceCollection::SendInvoice,
        due_date: Some(due_date),
    };
    let publish = |operation: BillingOperation| {
        service.handle(vec![Event::new(
            OrgScope {
                organization_id: org.id,
            },
            operation,
            AuthenticatedEntity::System,
        )])
    };

    // Issued: licensed past the due date, and the org counts as able to pay.
    let issued = sent(trial_end, due);
    let provisional = issued.provisional_paid_through().unwrap();
    publish(BillingOperation::InvoiceIssued {
        invoice: issued.clone(),
    })
    .await
    .unwrap();
    let reloaded = reload(&state, org.id).await;
    assert_eq!(reloaded.base.license_paid_through, Some(provisional));
    // Paying against a PO is a way to pay, so nothing asks for a card.
    assert!(reloaded.base.bills_by_invoice);
    assert!(!reloaded.base.has_payment_method);
    assert!(reloaded.can_pay());

    // An invoice due earlier never shortens the period already granted.
    publish(BillingOperation::InvoiceIssued {
        invoice: sent(trial_end, whole_seconds_from_now(5)),
    })
    .await
    .unwrap();
    assert_eq!(
        reload(&state, org.id).await.base.license_paid_through,
        Some(provisional)
    );

    // Voided: back to where the unpaid period started.
    publish(BillingOperation::InvoiceVoided {
        invoice: issued.clone(),
    })
    .await
    .unwrap();
    assert_eq!(
        reload(&state, org.id).await.base.license_paid_through,
        Some(trial_end)
    );

    // Paid: the term end replaces the provisional date, and a late void of the
    // same invoice leaves the paid term alone.
    publish(BillingOperation::InvoiceIssued {
        invoice: issued.clone(),
    })
    .await
    .unwrap();
    publish(BillingOperation::PaymentSucceeded {
        invoice: issued.clone(),
        previous_license_paid_through: None,
    })
    .await
    .unwrap();
    let term_end = issued.license_paid_through();
    assert_eq!(
        reload(&state, org.id).await.base.license_paid_through,
        term_end
    );
    publish(BillingOperation::InvoiceVoided { invoice: issued })
        .await
        .unwrap();
    assert_eq!(
        reload(&state, org.id).await.base.license_paid_through,
        term_end
    );

    // The subscription that billed by invoice is gone, so the org is back to
    // having no way to pay and the card prompts return.
    publish(BillingOperation::SubscriptionCancelled {
        plan: get_self_hosted_standard_plan(),
        reason_code: None,
        stripe_feedback: None,
        stripe_reason: None,
        internal_reason: None,
        comment: None,
        period_end: Utc::now(),
        was_trialing: false,
        mrr_amount_cents: 0,
        tenure_days: 30,
    })
    .await
    .unwrap();
    let cancelled = reload(&state, org.id).await;
    assert!(!cancelled.base.bills_by_invoice);
    assert!(!cancelled.can_pay());
}
