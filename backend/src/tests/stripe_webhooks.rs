//! Stripe webhook deliveries in either API version, end to end: signature,
//! envelope, the object fetched from (a stand-in for) Stripe, and the org row
//! the handlers leave behind.

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::extract::Path;
use axum::routing::get;
use chrono::Utc;
use email_address::EmailAddress;
use governor::Quota;
use std::num::NonZeroU32;
use testcontainers::{ContainerAsync, GenericImage};
use tokio::sync::broadcast::Receiver;
use uuid::Uuid;

use super::self_hosted_licensing::{reload, test_state_with_email_dir};
use super::{organization, user};
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::billing::plans::{get_free_plan, get_purchasable_plans};
use crate::server::billing::service::stripe_client::tests::{client_for, fake_stripe};
use crate::server::billing::service::webhook_envelope::tests::{
    SUBSCRIPTION_CREATED_CLOVER, SUBSCRIPTION_CREATED_DAHLIA, header,
};
use crate::server::billing::service::{BillingService, BillingServiceParams};
use crate::server::billing::types::base::{BillingPlan, PlanStatus};
use crate::server::config::AppState;
use crate::server::organizations::r#impl::base::Organization;
use crate::server::shared::events::bus::BusChannel;
use crate::server::shared::events::traits::Event;
use crate::server::shared::events::types::BillingOperation;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::types::metadata::TypeMetadataProvider;
use crate::server::users::r#impl::permissions::UserOrgPermissions;

pub(super) const WEBHOOK_SECRET: &str = "whsec_test_secret";
/// The org and customer the recorded checkout belongs to.
pub(super) const FIXTURE_ORG_ID: &str = "758bc122-2c76-459e-83c6-e05267c016c2";
pub(super) const FIXTURE_CUSTOMER_ID: &str = "cus_VMI18gjRBujIOD";
/// The subscription as Stripe returns it to our client, in our version.
pub(super) const SUBSCRIPTION_DAHLIA: &str =
    include_str!("../server/billing/service/fixtures/subscription.dahlia.json");
/// An empty Stripe list response.
pub(super) const EMPTY_LIST: &str =
    r#"{"object": "list", "data": [], "has_more": false, "url": "/v1/list"}"#;
/// A Stripe error response, for a stand-in that is failing.
pub(super) const STRIPE_OUTAGE: &str =
    r#"{"error": {"type": "api_error", "message": "fake outage"}}"#;

/// The recorded subscription, belonging to `org_id`.
pub(super) fn subscription_for(org_id: Uuid) -> String {
    SUBSCRIPTION_DAHLIA.replace(FIXTURE_ORG_ID, &org_id.to_string())
}

/// A billing service over `state`'s services whose Stripe is a local
/// stand-in serving `router`.
pub(super) async fn billing_with(state: &AppState, router: Router) -> BillingService {
    let (url, _server) = fake_stripe(router).await;
    let services = &state.services;
    let mut billing = BillingService::new(BillingServiceParams {
        stripe_secret: "sk_test_fake".to_string(),
        webhook_secret: WEBHOOK_SECRET.to_string(),
        organization_service: services.organization_service.clone(),
        user_service: services.user_service.clone(),
        site_service: services.site_service.clone(),
        host_service: services.host_service.clone(),
        event_bus: services.event_bus.clone(),
    });
    billing.stripe = client_for(
        &url,
        Quota::per_second(NonZeroU32::new(100).unwrap()),
        Duration::from_secs(1),
    );
    billing
}

/// A fresh database, plus a billing service whose Stripe serves the recorded
/// subscription for `org_id` and lists no others.
async fn billing_against_fake_stripe(
    org_id: Uuid,
) -> (Arc<AppState>, BillingService, ContainerAsync<GenericImage>) {
    let (state, container) = test_state_with_email_dir(None).await;
    let subscription = subscription_for(org_id);
    let router = Router::new()
        .route(
            "/v1/subscriptions/{id}",
            get(move |Path(_id): Path<String>| async move { subscription }),
        )
        .route("/v1/subscriptions", get(|| async { EMPTY_LIST }));
    let billing = billing_with(&state, router).await;
    (state, billing, container)
}

/// An org that has just been through checkout: a Stripe customer, an owner,
/// and no plan yet.
pub(super) async fn checked_out_org(state: &AppState, org_id: Uuid) -> Organization {
    let mut org = organization();
    org.id = org_id;
    org.base.stripe_customer_id = Some(FIXTURE_CUSTOMER_ID.to_string());
    let org = state
        .services
        .organization_service
        .create(org, AuthenticatedEntity::System)
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
    org
}

/// Sign `payload` as Stripe would and hand it to the webhook handler.
pub(super) async fn try_deliver(billing: &BillingService, payload: &str) -> anyhow::Result<()> {
    let signature = header(payload, WEBHOOK_SECRET, Utc::now().timestamp());
    billing.handle_webhook(payload, &signature).await
}

async fn deliver(billing: &BillingService, payload: &str) {
    try_deliver(billing, payload).await.unwrap();
}

/// Billing events published from now on.
pub(super) fn billing_events(state: &AppState) -> Receiver<Event<BillingOperation>> {
    BusChannel::<BillingOperation>::channel(&*state.services.event_bus).subscribe_channel()
}

/// How many events published since the last call match `operation`.
pub(super) fn published(
    events: &mut Receiver<Event<BillingOperation>>,
    operation: fn(&BillingOperation) -> bool,
) -> usize {
    let mut count = 0;
    while let Ok(event) = events.try_recv() {
        if operation(&event.operation) {
            count += 1;
        }
    }
    count
}

pub(super) fn is_checkout(operation: &BillingOperation) -> bool {
    matches!(operation, BillingOperation::CheckoutCompleted { .. })
}

/// The recorded subscription as it stands after a renewal: active, on
/// `plan`, belonging to `org_id`.
fn renewed_subscription(org_id: Uuid, plan: BillingPlan) -> String {
    let mut sub: serde_json::Value = serde_json::from_str(&subscription_for(org_id)).unwrap();
    sub["status"] = "active".into();
    sub["metadata"]["plan"] = serde_json::to_string(&plan).unwrap().into();
    sub.to_string()
}

/// A renewal moves the billing period, and Stripe reports that as
/// `customer.subscription.updated` on a subscription whose plan and status
/// are what the org already holds. That is no checkout, whatever the plan
/// costs: a legacy Free org got "Welcome to Scanopy Free" on each renewal.
#[tokio::test]
async fn a_renewal_on_an_unchanged_plan_is_not_a_checkout() {
    let pro = get_purchasable_plans()
        .into_iter()
        .find(|plan| matches!(plan, BillingPlan::Pro(_)))
        .unwrap();
    for plan in [get_free_plan(), pro] {
        let org_id = Uuid::new_v4();
        let (state, _container) = test_state_with_email_dir(None).await;
        let subscription = renewed_subscription(org_id, plan);
        let router = Router::new()
            .route(
                "/v1/subscriptions/{id}",
                get(move |Path(_id): Path<String>| async move { subscription }),
            )
            .route("/v1/subscriptions", get(|| async { EMPTY_LIST }));
        let billing = billing_with(&state, router).await;

        let mut org = checked_out_org(&state, org_id).await;
        org.base.plan = Some(plan);
        org.base.plan_status = Some(PlanStatus::Active);
        state
            .services
            .organization_service
            .update(&mut org, AuthenticatedEntity::System)
            .await
            .unwrap();
        let mut events = billing_events(&state);

        let renewal = SUBSCRIPTION_CREATED_DAHLIA.replace(
            "\"customer.subscription.created\"",
            "\"customer.subscription.updated\"",
        );
        deliver(&billing, &renewal).await;

        assert_eq!(published(&mut events, is_checkout), 0, "{}", plan.name());
    }
}

/// `evt_1ULZRvAkcpHEg5IAyzQrl0Oa` is a Pro checkout's
/// `customer.subscription.created`, recorded in clover, which the
/// dahlia-built SDK could not parse. In either rendering it now puts the org
/// on its plan, and a second delivery of it changes nothing.
#[tokio::test]
async fn a_checkout_in_either_api_version_sets_the_plan_once() {
    for payload in [SUBSCRIPTION_CREATED_CLOVER, SUBSCRIPTION_CREATED_DAHLIA] {
        let org_id = Uuid::new_v4();
        let (state, billing, _container) = billing_against_fake_stripe(org_id).await;
        let org = checked_out_org(&state, org_id).await;
        assert_eq!(org.base.plan_status, None);
        let mut events = billing_events(&state);

        deliver(&billing, payload).await;
        let after_first = reload(&state, org.id).await;
        assert!(after_first.base.plan.is_some_and(|plan| !plan.is_free()));
        assert_eq!(after_first.base.plan_status, Some(PlanStatus::Trialing));
        assert_eq!(published(&mut events, is_checkout), 1);

        deliver(&billing, payload).await;
        let after_second = reload(&state, org.id).await;
        assert_eq!(after_second.base.plan, after_first.base.plan);
        assert_eq!(after_second.base.plan_status, after_first.base.plan_status);
        assert_eq!(after_second.updated_at, after_first.updated_at);
        assert_eq!(published(&mut events, is_checkout), 0);
    }
}
