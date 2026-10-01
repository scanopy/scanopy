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
use uuid::Uuid;

use super::self_hosted_licensing::{reload, test_state_with_email_dir};
use super::{organization, user};
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::billing::service::stripe_client::tests::{client_for, fake_stripe};
use crate::server::billing::service::webhook_envelope::tests::{
    SUBSCRIPTION_CREATED_CLOVER, SUBSCRIPTION_CREATED_DAHLIA, header,
};
use crate::server::billing::service::{BillingService, BillingServiceParams};
use crate::server::billing::types::base::PlanStatus;
use crate::server::config::AppState;
use crate::server::organizations::r#impl::base::Organization;
use crate::server::shared::events::bus::BusChannel;
use crate::server::shared::events::types::BillingOperation;
use crate::server::shared::services::traits::CrudService;
use crate::server::users::r#impl::permissions::UserOrgPermissions;

const WEBHOOK_SECRET: &str = "whsec_test_secret";
/// The org and customer the recorded checkout belongs to.
const FIXTURE_ORG_ID: &str = "758bc122-2c76-459e-83c6-e05267c016c2";
const FIXTURE_CUSTOMER_ID: &str = "cus_VMI18gjRBujIOD";
/// The subscription as Stripe returns it to our client, in our version.
const SUBSCRIPTION_DAHLIA: &str =
    include_str!("../server/billing/service/fixtures/subscription.dahlia.json");

/// A fresh database, plus a billing service whose Stripe is a local stand-in
/// serving the recorded subscription for `org_id`.
async fn billing_against_fake_stripe(
    org_id: Uuid,
) -> (Arc<AppState>, BillingService, ContainerAsync<GenericImage>) {
    let (state, container) = test_state_with_email_dir(None).await;

    let subscription = SUBSCRIPTION_DAHLIA.replace(FIXTURE_ORG_ID, &org_id.to_string());
    let router = Router::new()
        .route(
            "/v1/subscriptions/{id}",
            get(move |Path(_id): Path<String>| {
                let subscription = subscription.clone();
                async move { subscription }
            }),
        )
        .route(
            "/v1/subscriptions",
            get(|| async {
                r#"{"object": "list", "data": [], "has_more": false, "url": "/v1/subscriptions"}"#
            }),
        );
    let (url, _server) = fake_stripe(router).await;

    let services = &state.services;
    let mut billing = BillingService::new(BillingServiceParams {
        stripe_secret: "sk_test_fake".to_string(),
        webhook_secret: WEBHOOK_SECRET.to_string(),
        organization_service: services.organization_service.clone(),
        user_service: services.user_service.clone(),
        network_service: services.network_service.clone(),
        host_service: services.host_service.clone(),
        event_bus: services.event_bus.clone(),
    });
    billing.stripe = client_for(
        &url,
        Quota::per_second(NonZeroU32::new(100).unwrap()),
        Duration::from_secs(1),
    );
    (state, billing, container)
}

/// An org that has just been through checkout: a Stripe customer, an owner,
/// and no plan yet.
async fn checked_out_org(state: &AppState, org_id: Uuid) -> Organization {
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

async fn deliver(billing: &BillingService, payload: &str) {
    let signature = header(payload, WEBHOOK_SECRET, Utc::now().timestamp());
    billing.handle_webhook(payload, &signature).await.unwrap();
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

        let mut published =
            BusChannel::<BillingOperation>::channel(&*state.services.event_bus).subscribe_channel();
        let mut checkouts = || {
            let mut count = 0;
            while let Ok(event) = published.try_recv() {
                if matches!(event.operation, BillingOperation::CheckoutCompleted { .. }) {
                    count += 1;
                }
            }
            count
        };

        deliver(&billing, payload).await;
        let after_first = reload(&state, org.id).await;
        assert!(after_first.base.plan.is_some_and(|plan| !plan.is_free()));
        assert_eq!(after_first.base.plan_status, Some(PlanStatus::Trialing));
        assert_eq!(checkouts(), 1);

        deliver(&billing, payload).await;
        let after_second = reload(&state, org.id).await;
        assert_eq!(after_second.base.plan, after_first.base.plan);
        assert_eq!(after_second.base.plan_status, after_first.base.plan_status);
        assert_eq!(after_second.updated_at, after_first.updated_at);
        assert_eq!(checkouts(), 0);
    }
}
