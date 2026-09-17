use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::billing::plans::YEARLY_DISCOUNT;
use crate::server::billing::plans::get_enterprise_plan;
use crate::server::billing::plans::get_free_plan;
use crate::server::billing::types::api::{
    CancelSubscriptionRequest, CancelSubscriptionResponse, ChangePlanPreview, PauseDuration,
    SaveOfferCoupon,
};
use crate::server::billing::types::base::{BillingInvoice, BillingPlan, CancelReason, PlanStatus};
use crate::server::billing::types::features::Feature;
use crate::server::billing::types::stripe_metadata::StripeSubscriptionMetadata;
use crate::server::hosts::service::HostService;
use crate::server::networks::r#impl::Network;
use crate::server::networks::service::NetworkService;
use crate::server::organizations::r#impl::base::Organization;
use crate::server::organizations::service::OrganizationService;
use crate::server::shared::events::bus::EventBus;
use crate::server::shared::events::traits::{Event, OrgScope};
use crate::server::shared::events::types::{
    BillingOperation, OnboardingOperation, OnboardingOperationDiscriminants,
};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::types::metadata::TypeMetadataProvider;
use crate::server::users::service::UserService;
use anyhow::Error;
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use std::sync::OnceLock;
use stripe::Client;
use stripe_billing::CancellationDetailsFeedback;
use stripe_billing::billing_portal_session::CreateBillingPortalSession;
use stripe_billing::subscription::CancelSubscription;
use stripe_billing::subscription::CreateSubscription;
use stripe_billing::subscription::CreateSubscriptionItems;
use stripe_billing::subscription::CreateSubscriptionTrialSettings;
use stripe_billing::subscription::CreateSubscriptionTrialSettingsEndBehavior;
use stripe_billing::subscription::CreateSubscriptionTrialSettingsEndBehaviorMissingPaymentMethod;
use stripe_billing::subscription::DiscountsDataParam;
use stripe_billing::subscription::ListSubscription;
use stripe_billing::subscription::UpdateSubscription;
use stripe_billing::subscription::UpdateSubscriptionCancelAt;
use stripe_billing::subscription::UpdateSubscriptionCancellationDetails;
use stripe_billing::subscription::UpdateSubscriptionCancellationDetailsFeedback;
use stripe_billing::subscription::UpdateSubscriptionItems;
use stripe_billing::subscription::UpdateSubscriptionPauseCollection;
use stripe_billing::subscription::UpdateSubscriptionPauseCollectionBehavior;
use stripe_billing::subscription::UpdateSubscriptionProrationBehavior;
use stripe_billing::subscription::UpdateSubscriptionTrialEnd;
use stripe_billing::{Subscription, SubscriptionStatus};
use stripe_checkout::checkout_session::CreateCheckoutSessionCustomerUpdate;
use stripe_checkout::checkout_session::CreateCheckoutSessionCustomerUpdateAddress;
use stripe_checkout::checkout_session::CreateCheckoutSessionCustomerUpdateName;
use stripe_checkout::checkout_session::CreateCheckoutSessionPaymentMethodCollection;
use stripe_checkout::checkout_session::CreateCheckoutSessionSubscriptionData;
use stripe_checkout::checkout_session::{
    CreateCheckoutSession, CreateCheckoutSessionLineItems, CreateCheckoutSessionTaxIdCollection,
};
use stripe_checkout::{
    CheckoutSession, CheckoutSessionBillingAddressCollection, CheckoutSessionMode,
};
use stripe_client_core::{RequestBuilder, StripeMethod, StripeRequest};
use stripe_core::customer::CreateCustomer;
use stripe_core::customer::DeleteCustomer;
use stripe_core::customer::DeleteDiscountCustomer;
use stripe_core::customer::ListPaymentMethodsCustomer;
use stripe_core::customer::UpdateCustomer;
use stripe_core::customer::UpdateCustomerInvoiceSettings;
use stripe_core::customer_balance_transaction::CreateCustomerCustomerBalanceTransaction;
use stripe_core::setup_intent::CreateSetupIntent;
use stripe_core::setup_intent::CreateSetupIntentAutomaticPaymentMethods;
use stripe_core::setup_intent::CreateSetupIntentAutomaticPaymentMethodsAllowRedirects;
use stripe_core::setup_intent::CreateSetupIntentUsage;
use stripe_core::setup_intent::RetrieveSetupIntent;
use stripe_core::{CustomerId, EventType};
use stripe_product::Price;
use stripe_product::Product;
use stripe_product::coupon::RetrieveCoupon;
use stripe_product::price::CreatePriceRecurring;
use stripe_product::price::SearchPrice;
use stripe_product::price::{CreatePrice, CreatePriceRecurringUsageType};
use stripe_product::product::Features;
use stripe_product::product::{CreateProduct, RetrieveProduct};
use stripe_shared::SetupIntentStatus;
use stripe_webhook::{EventObject, Webhook};
use uuid::Uuid;

pub struct BillingService {
    pub stripe: stripe::Client,
    pub webhook_secret: String,
    pub organization_service: Arc<OrganizationService>,
    pub user_service: Arc<UserService>,
    pub network_service: Arc<NetworkService>,
    pub host_service: Arc<HostService>,
    pub plans: OnceLock<Vec<BillingPlan>>,
    pub event_bus: Arc<EventBus>,
    /// Secret key for Stripe endpoints the SDK cannot reach: rendered quote
    /// PDFs are binary and served from Stripe's files host.
    stripe_secret: String,
    files_http: reqwest::Client,
}

const SEAT_PRODUCT_ID: &str = "extra_seats";
const SEAT_PRODUCT_NAME: &str = "Extra Seats";
const NETWORK_PRODUCT_ID: &str = "extra_networks";
const NETWORK_PRODUCT_NAME: &str = "Extra Networks";

pub struct BillingServiceParams {
    pub stripe_secret: String,
    pub webhook_secret: String,
    pub organization_service: Arc<OrganizationService>,
    pub user_service: Arc<UserService>,
    pub network_service: Arc<NetworkService>,
    pub host_service: Arc<HostService>,
    pub event_bus: Arc<EventBus>,
}

mod checkout;
mod invoicing;
mod lifecycle;
mod plan_changes;
mod setup;
mod webhooks;

fn extract_cancellation_details(
    details: Option<&stripe_billing::CancellationDetails>,
) -> (
    Option<stripe_billing::CancellationDetailsFeedback>,
    Option<String>,
    Option<stripe_billing::CancellationDetailsReason>,
) {
    let feedback = details.and_then(|d| d.feedback);
    let comment = details.and_then(|d| d.comment.clone());
    let reason = details.and_then(|d| d.reason);
    (feedback, comment, reason)
}

/// Monthly equivalent (in cents) for a single line: unit * qty, divided by 12
/// when yearly. Weekly/daily are not sold so collapse into the monthly bucket.
fn line_monthly_cents(unit_amount: Option<i64>, quantity: Option<u64>, is_yearly: bool) -> i64 {
    let line = unit_amount.unwrap_or(0) * (quantity.unwrap_or(0) as i64);
    if is_yearly { line / 12 } else { line }
}

/// Sum monthly recurring revenue (in cents) across all line items of a Stripe
/// subscription.
fn mrr_from_subscription(sub: &stripe_billing::Subscription) -> i64 {
    sub.items
        .data
        .iter()
        .map(|item| {
            let is_yearly = item
                .price
                .recurring
                .as_ref()
                .map(|r| matches!(r.interval, stripe_product::RecurringInterval::Year))
                .unwrap_or(false);
            line_monthly_cents(item.price.unit_amount, item.quantity, is_yearly)
        })
        .sum()
}

/// `sub.items.data[0].current_period_end` decoded to a chrono timestamp,
/// or `None` if the subscription has no items / no period. This is the
/// canonical "next renewal" timestamp surfaced on `org.next_renewal_at`
/// and on `BillingOperation::*::next_renewal_at` payload fields.
fn next_renewal_from_subscription(sub: &stripe_billing::Subscription) -> Option<DateTime<Utc>> {
    sub.items
        .data
        .first()
        .map(|i| i.current_period_end)
        .and_then(|ts| DateTime::<Utc>::from_timestamp(ts, 0))
}

/// Map our canonical `CancelReason` to the Stripe-side feedback enum.
/// Variant names match by string identity in both crates.
fn map_cancel_reason_to_stripe(
    reason: CancelReason,
) -> Option<UpdateSubscriptionCancellationDetailsFeedback> {
    use UpdateSubscriptionCancellationDetailsFeedback as F;
    Some(match reason {
        CancelReason::TooExpensive => F::TooExpensive,
        CancelReason::MissingFeatures => F::MissingFeatures,
        CancelReason::SwitchedService => F::SwitchedService,
        CancelReason::Unused => F::Unused,
        CancelReason::CustomerService => F::CustomerService,
        CancelReason::LowQuality => F::LowQuality,
        CancelReason::TooComplex => F::TooComplex,
        CancelReason::Other => F::Other,
    })
}

/// Computed pause credit + the clamped paused duration that produced it.
/// The caller wants both so the Stripe balance-transaction description
/// can show the same day count the credit math used (otherwise a 35-day
/// elapsed clamped to a 30-day requested duration would label the credit
/// "35 days" while the amount reflects 30 — a confusing reconcile mismatch
/// for anyone reading the customer's balance history).
struct PauseCredit {
    credit_cents: i64,
    actual_paused_secs: i64,
}

/// Pure read-only calculation of the prorated pause credit. Called from
/// the webhook Resumed arm (which actually posts the balance transaction).
/// Returns `None` when there's nothing to credit (metadata missing, no
/// items on the sub, non-positive period, or zero/negative computed
/// credit).
///
/// The math:
/// - `actual_paused_secs = clamp(now - scanopy_paused_at, 0, requested_secs)`
/// - `effective_per_period = base × (1 − active_discount_pct)` (use the
///   post-discount rate so we don't over-credit by the coupon amount)
/// - `credit_cents = effective_per_period × actual_paused_secs / period_secs`
fn compute_pause_credit(sub: &Subscription, organization: &Organization) -> Option<PauseCredit> {
    let meta = StripeSubscriptionMetadata::from_stripe(&sub.metadata);
    let paused_at_ts = meta.scanopy_paused_at?;
    let item = sub.items.data.first()?;

    let now_ts = Utc::now().timestamp();
    let raw_elapsed = (now_ts - paused_at_ts).max(0);
    let cap_secs = meta
        .scanopy_pause_duration_days
        .map(|d| i64::from(d) * 86_400)
        .unwrap_or(raw_elapsed);
    let actual_paused_secs = raw_elapsed.min(cap_secs);

    let period_secs = item.current_period_end - item.current_period_start;
    if period_secs <= 0 {
        return None;
    }

    let gross_per_period = item.price.unit_amount.unwrap_or(0);
    let effective_per_period = match (
        organization.base.discount_save_offer_percent_off,
        organization.base.discount_save_offer_active_until,
    ) {
        (Some(percent_off), Some(active_until)) if active_until > Utc::now() => {
            (gross_per_period as f64 * (1.0 - percent_off as f64 / 100.0)).round() as i64
        }
        _ => gross_per_period,
    };

    let credit_cents = i64::try_from(
        i128::from(effective_per_period) * i128::from(actual_paused_secs) / i128::from(period_secs),
    )
    .unwrap_or(0);

    if credit_cents > 0 {
        Some(PauseCredit {
            credit_cents,
            actual_paused_secs,
        })
    } else {
        None
    }
}

/// Form body for clearing `pause_collection` via the Stripe REST API.
///
/// Stripe accepts an empty form value (`pause_collection=`) as the documented
/// "clear this field" convention. The SDK's `UpdateSubscription::pause_collection`
/// setter takes a typed struct and can't produce that wire representation.
#[derive(serde::Serialize)]
struct ClearPauseCollectionForm {
    pause_collection: &'static str,
}

/// Custom Stripe request used by [`BillingService::resume_subscription`].
///
/// Implements [`StripeRequest`] directly so it reuses the existing
/// `stripe::Client` (auth, retries, response decoding) without dropping to raw
/// HTTP or stashing the Stripe secret on `BillingService`.
struct ClearPauseCollection {
    sub_id: stripe_billing::SubscriptionId,
    body: ClearPauseCollectionForm,
}

impl ClearPauseCollection {
    fn new(sub_id: stripe_billing::SubscriptionId) -> Self {
        Self {
            sub_id,
            body: ClearPauseCollectionForm {
                pause_collection: "",
            },
        }
    }
}

impl StripeRequest for ClearPauseCollection {
    type Output = stripe_billing::Subscription;

    fn build(&self) -> RequestBuilder {
        RequestBuilder::new(
            StripeMethod::Post,
            format!("/subscriptions/{}", self.sub_id),
        )
        .form(&self.body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_cancellation_details_none_input() {
        assert_eq!(extract_cancellation_details(None), (None, None, None));
    }

    #[test]
    fn extract_cancellation_details_all_inner_none() {
        let details = stripe_billing::CancellationDetails {
            comment: None,
            feedback: None,
            reason: None,
        };
        assert_eq!(
            extract_cancellation_details(Some(&details)),
            (None, None, None)
        );
    }

    #[test]
    fn extract_cancellation_details_fully_populated() {
        let details = stripe_billing::CancellationDetails {
            comment: Some("too pricey for our team".to_string()),
            feedback: Some(stripe_billing::CancellationDetailsFeedback::TooExpensive),
            reason: Some(stripe_billing::CancellationDetailsReason::CancellationRequested),
        };
        assert_eq!(
            extract_cancellation_details(Some(&details)),
            (
                Some(CancellationDetailsFeedback::TooExpensive),
                Some("too pricey for our team".to_string()),
                Some(stripe_billing::CancellationDetailsReason::CancellationRequested),
            )
        );
    }

    #[test]
    fn line_monthly_cents_monthly_passthrough() {
        assert_eq!(line_monthly_cents(Some(2900), Some(1), false), 2900);
    }

    #[test]
    fn line_monthly_cents_yearly_divides_by_12() {
        // $290.00/yr * 1 = $290.00/yr -> 29000 cents -> 2416 cents/mo (truncated)
        assert_eq!(line_monthly_cents(Some(29000), Some(1), true), 2416);
    }

    #[test]
    fn line_monthly_cents_quantity_multiplies() {
        assert_eq!(line_monthly_cents(Some(500), Some(7), false), 3500);
    }

    #[test]
    fn line_monthly_cents_missing_fields_zero() {
        assert_eq!(line_monthly_cents(None, Some(3), false), 0);
        assert_eq!(line_monthly_cents(Some(500), None, false), 0);
        assert_eq!(line_monthly_cents(None, None, true), 0);
    }

    #[test]
    fn pause_duration_days_mapping() {
        assert_eq!(PauseDuration::Days30.days(), 30);
        assert_eq!(PauseDuration::Days60.days(), 60);
        assert_eq!(PauseDuration::Days90.days(), 90);
    }

    #[test]
    fn cancel_reason_maps_to_stripe_feedback() {
        use UpdateSubscriptionCancellationDetailsFeedback as F;
        let cases = [
            (CancelReason::TooExpensive, F::TooExpensive),
            (CancelReason::MissingFeatures, F::MissingFeatures),
            (CancelReason::SwitchedService, F::SwitchedService),
            (CancelReason::Unused, F::Unused),
            (CancelReason::CustomerService, F::CustomerService),
            (CancelReason::LowQuality, F::LowQuality),
            (CancelReason::TooComplex, F::TooComplex),
            (CancelReason::Other, F::Other),
        ];
        for (reason, expected) in cases {
            assert_eq!(map_cancel_reason_to_stripe(reason), Some(expected));
        }
    }
}
