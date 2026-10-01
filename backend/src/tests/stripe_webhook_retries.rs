//! What a webhook that fails partway leaves behind when Stripe redelivers it.
//!
//! Stripe redelivers after any error. A handler that published an email
//! event before failing would send it again, so these pin that each one either
//! fails before its first email or is gated on state the first attempt wrote.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use super::self_hosted_licensing::{reload, test_state_with_email_dir};
use super::stripe_webhooks::{
    EMPTY_LIST, FIXTURE_CUSTOMER_ID, STRIPE_OUTAGE, SUBSCRIPTION_DAHLIA, billing_events,
    billing_with, checked_out_org, is_checkout, published, subscription_for, try_deliver,
};
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::billing::plans::get_self_hosted_standard_plan;
use crate::server::billing::service::webhook_envelope::tests::{
    INVOICE_PAID_DAHLIA, SUBSCRIPTION_CREATED_CLOVER,
};
use crate::server::billing::types::base::PlanStatus;
use crate::server::config::AppState;
use crate::server::shared::events::types::BillingOperation;
use crate::server::shared::services::traits::CrudService;

const INVOICE_DAHLIA: &str = include_str!("../server/billing/service/fixtures/invoice.dahlia.json");
const INVOICE_ID: &str = "in_1ULZRtAkcpHEg5IATS8EybqB";

/// One fake Stripe endpoint: fails with an outage while `failures` is above
/// zero, then answers `body`. Counts its requests and keeps each one's
/// `Idempotency-Key`.
#[derive(Clone)]
struct Endpoint {
    body: Arc<Mutex<String>>,
    failures: Arc<AtomicUsize>,
    hits: Arc<AtomicUsize>,
    idempotency_keys: Arc<Mutex<Vec<Option<String>>>>,
}

impl Endpoint {
    fn new(body: impl Into<String>) -> Self {
        Self {
            body: Arc::new(Mutex::new(body.into())),
            failures: Arc::new(AtomicUsize::new(0)),
            hits: Arc::new(AtomicUsize::new(0)),
            idempotency_keys: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn failing(self, times: usize) -> Self {
        self.failures.store(times, Ordering::SeqCst);
        self
    }

    fn fail_next(&self, times: usize) {
        self.failures.store(times, Ordering::SeqCst);
    }

    fn answer(&self, body: impl Into<String>) {
        *self.body.lock().unwrap() = body.into();
    }

    fn hits(&self) -> usize {
        self.hits.load(Ordering::SeqCst)
    }

    fn respond(&self, headers: &HeaderMap) -> (StatusCode, String) {
        self.hits.fetch_add(1, Ordering::SeqCst);
        self.idempotency_keys.lock().unwrap().push(
            headers
                .get("idempotency-key")
                .and_then(|value| value.to_str().ok())
                .map(str::to_string),
        );
        let failing = self
            .failures
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
            .is_ok();
        if failing {
            (StatusCode::INTERNAL_SERVER_ERROR, STRIPE_OUTAGE.to_string())
        } else {
            (StatusCode::OK, self.body.lock().unwrap().clone())
        }
    }

    fn get(&self) -> axum::routing::MethodRouter {
        let endpoint = self.clone();
        get(move |headers: HeaderMap| async move { endpoint.respond(&headers) })
    }

    fn post(&self) -> axum::routing::MethodRouter {
        let endpoint = self.clone();
        post(move |headers: HeaderMap| async move { endpoint.respond(&headers) })
    }
}

fn list_of(objects: &[Value]) -> String {
    json!({"object": "list", "data": objects, "has_more": false, "url": "/v1/list"}).to_string()
}

fn whole_seconds(at: DateTime<Utc>) -> DateTime<Utc> {
    DateTime::from_timestamp(at.timestamp(), 0).unwrap()
}

fn is_payment(operation: &BillingOperation) -> bool {
    matches!(operation, BillingOperation::PaymentSucceeded { .. })
}

fn is_resume(operation: &BillingOperation) -> bool {
    matches!(operation, BillingOperation::LicenseResumed { .. })
}

fn is_trial_start(operation: &BillingOperation) -> bool {
    matches!(operation, BillingOperation::TrialStarted { .. })
}

fn is_cancellation(operation: &BillingOperation) -> bool {
    matches!(operation, BillingOperation::SubscriptionCancelled { .. })
}

/// The duplicate-subscription cleanup runs after the checkout emails. When
/// it fails, the redelivery finds the org already on its plan, so it finishes
/// the cleanup and sends neither email again.
#[tokio::test]
async fn a_failure_after_the_checkout_emails_does_not_resend_them() {
    let (state, _container) = test_state_with_email_dir(None).await;
    let org_id = Uuid::new_v4();
    let subscriptions = Endpoint::new(EMPTY_LIST).failing(1);
    let router = Router::new()
        .route(
            "/v1/subscriptions/{id}",
            Endpoint::new(subscription_for(org_id)).get(),
        )
        .route("/v1/subscriptions", subscriptions.get());
    let billing = billing_with(&state, router).await;
    checked_out_org(&state, org_id).await;
    let mut events = billing_events(&state);

    assert!(
        try_deliver(&billing, SUBSCRIPTION_CREATED_CLOVER)
            .await
            .is_err()
    );
    try_deliver(&billing, SUBSCRIPTION_CREATED_CLOVER)
        .await
        .unwrap();

    assert_eq!(
        subscriptions.hits(),
        2,
        "the redelivery finishes the cleanup"
    );
    let mut checkouts = 0;
    let mut trial_starts = 0;
    while let Ok(event) = events.try_recv() {
        checkouts += usize::from(is_checkout(&event.operation));
        trial_starts += usize::from(is_trial_start(&event.operation));
    }
    assert_eq!((checkouts, trial_starts), (1, 1));
    assert_eq!(
        reload(&state, org_id).await.base.plan_status,
        Some(PlanStatus::Trialing)
    );
}

/// A lapsed self-hosted org settling the licence invoice we wrote off, and
/// the Stripe that answers for it.
struct LicenseResumeCase {
    org_id: Uuid,
    resumed_through: DateTime<Utc>,
    /// The subscription the resume creates, as Stripe returns it.
    resumed_subscription: Value,
    subscriptions: Endpoint,
    create_subscription: Endpoint,
}

impl LicenseResumeCase {
    /// `invoice.dahlia.json` edited into a paid, written-off licence invoice:
    /// its line moved onto the self-hosted product, its term ending in 100
    /// days, written off 30 days ago and paid now. 130 days were left when it
    /// was written off, so the licence resumes to 130 days from now.
    async fn new(state: &AppState) -> (Self, crate::server::billing::service::BillingService) {
        let plan = get_self_hosted_standard_plan();
        let now = whole_seconds(Utc::now());
        let term_end = now + Duration::days(100);
        let written_off = now - Duration::days(30);
        let resumed_through = now + Duration::days(130);

        let mut invoice: Value = serde_json::from_str(INVOICE_DAHLIA).unwrap();
        let line = &mut invoice["lines"]["data"][0];
        line["pricing"]["price_details"]["product"] = json!(plan.stripe_product_id());
        line["period"]["end"] = json!(term_end.timestamp());
        invoice["status_transitions"]["marked_uncollectible_at"] = json!(written_off.timestamp());
        invoice["status_transitions"]["paid_at"] = json!(now.timestamp());

        let org_id = Uuid::new_v4();
        let mut resumed_subscription: Value =
            serde_json::from_str(&subscription_for(org_id)).unwrap();
        resumed_subscription["id"] = json!("sub_resumed");
        resumed_subscription["status"] = json!("active");
        resumed_subscription["billing_cycle_anchor"] = json!(resumed_through.timestamp());

        let price = {
            let subscription: Value = serde_json::from_str(SUBSCRIPTION_DAHLIA).unwrap();
            subscription["items"]["data"][0]["price"].clone()
        };
        let price_search = json!({
            "object": "search_result", "data": [price], "has_more": false,
            "next_page": null, "url": "/v1/prices/search"
        });

        let subscriptions = Endpoint::new(EMPTY_LIST);
        let create_subscription = Endpoint::new(resumed_subscription.to_string());
        let router = Router::new()
            .route(
                "/v1/invoices/{id}",
                Endpoint::new(invoice.to_string()).get(),
            )
            .route(
                "/v1/prices/search",
                Endpoint::new(price_search.to_string()).get(),
            )
            .route(
                "/v1/subscriptions",
                subscriptions.get().merge(create_subscription.post()),
            );
        let billing = billing_with(state, router).await;

        checked_out_org(state, org_id).await;
        let service = &state.services.organization_service;
        let mut org = service.get_by_id(&org_id).await.unwrap().unwrap();
        org.base.plan = Some(plan);
        org.base.plan_status = Some(PlanStatus::Cancelled);
        service
            .update(&mut org, AuthenticatedEntity::System)
            .await
            .unwrap();
        assert!(reload(state, org_id).await.is_lapsed());

        (
            Self {
                org_id,
                resumed_through,
                resumed_subscription,
                subscriptions,
                create_subscription,
            },
            billing,
        )
    }
}

/// Creating the resumed subscription fails. No payment email has gone out,
/// and the redelivery resumes the licence and sends it once.
#[tokio::test]
async fn a_failed_licence_resume_sends_nothing_until_it_succeeds() {
    let (state, _container) = test_state_with_email_dir(None).await;
    let (case, billing) = LicenseResumeCase::new(&state).await;
    case.create_subscription.fail_next(1);
    let mut events = billing_events(&state);

    assert!(try_deliver(&billing, INVOICE_PAID_DAHLIA).await.is_err());
    assert_eq!(published(&mut events, is_payment), 0);

    try_deliver(&billing, INVOICE_PAID_DAHLIA).await.unwrap();
    let mut payments = 0;
    let mut resumes = 0;
    while let Ok(event) = events.try_recv() {
        payments += usize::from(is_payment(&event.operation));
        resumes += usize::from(is_resume(&event.operation));
    }
    assert_eq!((payments, resumes), (1, 1));

    let org = reload(&state, case.org_id).await;
    assert_eq!(org.base.license_paid_through, Some(case.resumed_through));
    assert_eq!(org.base.plan_status, Some(PlanStatus::Active));

    // Both attempts asked Stripe for the same subscription.
    let keys = case.create_subscription.idempotency_keys.lock().unwrap();
    assert_eq!(keys.len(), 2);
    assert_eq!(keys[0], keys[1]);
    assert_eq!(
        keys[0].as_deref(),
        Some(format!("resume-license-{INVOICE_ID}").as_str())
    );
}

/// An earlier delivery created the resumed subscription but was not recorded.
/// The redelivery records the resume against it and creates nothing.
#[tokio::test]
async fn a_resume_created_but_not_recorded_is_finished() {
    let (state, _container) = test_state_with_email_dir(None).await;
    let (case, billing) = LicenseResumeCase::new(&state).await;
    case.subscriptions
        .answer(list_of(&[case.resumed_subscription.clone()]));
    let mut events = billing_events(&state);

    try_deliver(&billing, INVOICE_PAID_DAHLIA).await.unwrap();

    assert_eq!(published(&mut events, is_resume), 1);
    assert_eq!(case.create_subscription.hits(), 0);
    let org = reload(&state, case.org_id).await;
    assert_eq!(org.base.license_paid_through, Some(case.resumed_through));
}

/// Stripe failing to list the org's subscriptions is not an answer that it
/// has none, so no second subscription is created.
#[tokio::test]
async fn a_failed_subscription_lookup_creates_nothing() {
    let (state, _container) = test_state_with_email_dir(None).await;
    let (case, billing) = LicenseResumeCase::new(&state).await;
    case.subscriptions.fail_next(1);
    let mut events = billing_events(&state);

    assert!(try_deliver(&billing, INVOICE_PAID_DAHLIA).await.is_err());

    assert_eq!(case.create_subscription.hits(), 0);
    assert_eq!(published(&mut events, is_payment), 0);
    assert!(reload(&state, case.org_id).await.is_lapsed());
}

/// A deletion that fails partway answers with an error, so Stripe redelivers
/// it, instead of answering 200 and leaving the org on a plan it no longer
/// pays for.
#[tokio::test]
async fn a_failed_deletion_is_redelivered_and_lapses_the_org() {
    let (state, _container) = test_state_with_email_dir(None).await;
    let org_id = Uuid::new_v4();
    let subscription = Endpoint::new(subscription_for(org_id));
    let subscriptions = Endpoint::new(EMPTY_LIST);
    let router = Router::new()
        .route("/v1/subscriptions/{id}", subscription.get())
        .route("/v1/subscriptions", subscriptions.get())
        .route("/v1/invoices", Endpoint::new(EMPTY_LIST).get());
    let billing = billing_with(&state, router).await;
    checked_out_org(&state, org_id).await;
    try_deliver(&billing, SUBSCRIPTION_CREATED_CLOVER)
        .await
        .unwrap();
    assert_eq!(
        reload(&state, org_id)
            .await
            .base
            .stripe_customer_id
            .as_deref(),
        Some(FIXTURE_CUSTOMER_ID)
    );

    let mut canceled: Value = serde_json::from_str(&subscription_for(org_id)).unwrap();
    canceled["status"] = json!("canceled");
    canceled["ended_at"] = json!(Utc::now().timestamp());
    subscription.answer(canceled.to_string());
    subscriptions.fail_next(1);
    let deleted = SUBSCRIPTION_CREATED_CLOVER.replace(
        "customer.subscription.created",
        "customer.subscription.deleted",
    );
    let mut events = billing_events(&state);

    assert!(try_deliver(&billing, &deleted).await.is_err());
    assert_eq!(published(&mut events, is_cancellation), 0);
    assert_eq!(
        reload(&state, org_id).await.base.plan_status,
        Some(PlanStatus::Trialing)
    );

    try_deliver(&billing, &deleted).await.unwrap();
    assert_eq!(published(&mut events, is_cancellation), 1);
    assert_eq!(
        reload(&state, org_id).await.base.plan_status,
        Some(PlanStatus::Cancelled)
    );
}
