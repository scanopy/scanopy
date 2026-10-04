//! Which billing emails a customer paying nothing still receives.
//!
//! Receipts and "your plan is active" emails say nothing at $0 and are
//! skipped. Receipts read the invoice's own amounts, so a 100%-off coupon on a
//! paid plan counts as $0 there. The plan-active emails read the plan's price,
//! so a paid plan on a coupon still gets its welcome. Emails that change what
//! the customer can do (a lapse, a cancellation, a new card, a new air-gapped
//! key) go out at any amount.

use chrono::{DateTime, Utc};

use super::self_hosted_license_emails::{campaign, create_owner, handle_billing, sent};
use super::self_hosted_licensing::{create_org, reload, test_state_with_email_dir};
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::billing::plans::{
    get_enterprise_plan, get_free_plan, get_purchasable_plans, get_self_hosted_plus_plan,
};
use crate::server::billing::types::base::{
    BillingInvoice, BillingInvoiceLineItem, BillingPlan, BillingReason, InvoiceCollection,
};
use crate::server::license::types::LicenseKeyType;
use crate::server::shared::events::types::BillingOperation;
use crate::server::shared::services::traits::CrudService;

fn pro() -> BillingPlan {
    get_purchasable_plans()
        .into_iter()
        .find(|plan| matches!(plan, BillingPlan::Pro(_)))
        .unwrap()
}

/// A renewal invoice for `plan`. `total_cents` is what the invoice came to
/// after discounts; `amount_paid_cents` is what was taken from the customer
/// after account credit.
fn renewal(plan: BillingPlan, total_cents: i64, amount_paid_cents: i64) -> BillingInvoice {
    let now: DateTime<Utc> = Utc::now();
    let period_end = now + chrono::Duration::days(30);
    BillingInvoice {
        stripe_invoice_id: "in_renewal".to_string(),
        amount_paid_cents,
        currency: "usd".to_string(),
        created_at: now,
        period_start: now,
        period_end,
        billing_reason: BillingReason::SubscriptionCycle,
        line_items: vec![BillingInvoiceLineItem {
            description: None,
            amount_cents: total_cents,
            period_start: now,
            period_end,
            product: Some(plan.stripe_product_id()),
        }],
        invoice_pdf: None,
        hosted_invoice_url: None,
        collection: InvoiceCollection::ChargeAutomatically,
        po_number: None,
        amount_due_cents: amount_paid_cents,
        total_cents,
        due_date: None,
    }
}

fn paid(invoice: BillingInvoice) -> BillingOperation {
    BillingOperation::PaymentSucceeded {
        invoice,
        previous_license_paid_through: None,
    }
}

fn checkout(plan: BillingPlan) -> BillingOperation {
    BillingOperation::CheckoutCompleted {
        plan,
        included_networks: plan.config().included_networks,
        included_seats: plan.config().included_seats,
        mrr_amount_cents: plan.config().base_cents,
        is_trialing: false,
        next_renewal_at: None,
    }
}

/// Sends `operation` to the email subscriber for an org with an owner on
/// `plan`, and returns everything sent.
async fn emails_for(plan: BillingPlan, operation: BillingOperation) -> String {
    let dir = tempfile::tempdir().unwrap();
    let (state, _container) = test_state_with_email_dir(Some(dir.path().to_path_buf())).await;
    let org = create_org(&state, plan, None).await;
    create_owner(&state, org.id).await;
    handle_billing(&state, org.id, operation).await;
    sent(dir.path())
}

#[tokio::test]
async fn a_renewal_receipt_goes_out_only_when_the_invoice_came_to_something() {
    let cases = [
        // Legacy Free: a $0 price.
        (get_free_plan(), renewal(get_free_plan(), 0, 0), false),
        // A paid plan on a 100%-off coupon.
        (pro(), renewal(pro(), 0, 0), false),
        // A paid plan, charged.
        (pro(), renewal(pro(), 9999, 9999), true),
        // A real total the customer's account credit covered in full.
        (pro(), renewal(pro(), 9999, 0), true),
    ];
    for (plan, invoice, expected) in cases {
        let (total, paid_cents) = (invoice.total_cents, invoice.amount_paid_cents);
        let sent = emails_for(plan, paid(invoice)).await;
        assert_eq!(
            sent.contains(&campaign("usage_summary")),
            expected,
            "total {total}, paid {paid_cents}: {sent}"
        );
    }
}

#[tokio::test]
async fn the_welcome_email_goes_out_only_for_a_priced_plan() {
    let free = emails_for(get_free_plan(), checkout(get_free_plan())).await;
    assert!(!free.contains(&campaign("checkout_completed")), "{free}");

    let paid = emails_for(pro(), checkout(pro())).await;
    assert!(paid.contains(&campaign("checkout_completed")), "{paid}");

    // Enterprise has no list price, but it is not free.
    let enterprise = emails_for(get_enterprise_plan(), checkout(get_enterprise_plan())).await;
    assert!(
        enterprise.contains(&campaign("checkout_completed")),
        "{enterprise}"
    );
}

#[tokio::test]
async fn the_trial_converted_email_goes_out_only_for_a_priced_plan() {
    for (plan, expected) in [
        (get_free_plan(), false),
        (pro(), true),
        (get_enterprise_plan(), true),
    ] {
        let sent = emails_for(
            plan,
            BillingOperation::TrialEnded {
                plan,
                next_renewal_at: None,
            },
        )
        .await;
        assert_eq!(
            sent.contains(&campaign("trial_converted")),
            expected,
            "{sent}"
        );
    }
}

/// A free customer still hears about what changes their access or their
/// account's security.
#[tokio::test]
async fn a_free_customer_still_gets_the_emails_that_change_something() {
    let free = get_free_plan();
    let period_end = Utc::now() + chrono::Duration::days(10);
    let cases = [
        (
            BillingOperation::SubscriptionCancelled {
                plan: free,
                reason_code: None,
                stripe_feedback: None,
                stripe_reason: None,
                internal_reason: None,
                comment: None,
                period_end,
                was_trialing: false,
                mrr_amount_cents: 0,
                tenure_days: 200,
                license_key_type: None,
                defaulted: false,
            },
            "subscription_cancelled",
        ),
        (
            BillingOperation::CancellationInitiated {
                plan: Some(free),
                reason_code: None,
                stripe_feedback: None,
                stripe_reason: None,
                comment: None,
                save_offer_shown: vec![],
                save_offer_redeemed: None,
                planned_period_end: period_end,
            },
            "cancellation_initiated",
        ),
        (BillingOperation::PaymentMethodAdded, "payment_method_added"),
    ];
    for (operation, slug) in cases {
        let sent = emails_for(free, operation).await;
        assert!(sent.contains(&campaign(slug)), "{slug}: {sent}");
    }
}

/// An air-gapped key carries its expiry inside it, so a renewal has to be
/// copied by hand whatever it cost.
#[tokio::test]
async fn a_zero_dollar_air_gapped_renewal_still_says_to_copy_the_key() {
    let dir = tempfile::tempdir().unwrap();
    let (state, _container) = test_state_with_email_dir(Some(dir.path().to_path_buf())).await;
    let plus = get_self_hosted_plus_plan();
    let org = create_org(&state, plus, Some(Utc::now())).await;
    let mut air_gapped = reload(&state, org.id).await;
    air_gapped.base.license_key_type = Some(LicenseKeyType::Offline);
    state
        .services
        .organization_service
        .update(&mut air_gapped, AuthenticatedEntity::System)
        .await
        .unwrap();
    create_owner(&state, org.id).await;

    handle_billing(&state, org.id, paid(renewal(plus, 0, 0))).await;

    let sent = sent(dir.path());
    assert!(sent.contains(&campaign("airgap_renewal")), "{sent}");
}

/// PostHog and Brevo drop exactly the events whose email is skipped, so a
/// receipt or welcome nobody was sent leaves no analytics or CRM trace.
/// Self-hosted renewals stay: a $0 one still carries a new license period.
#[test]
fn analytics_drop_only_the_zero_dollar_notices() {
    let plus = get_self_hosted_plus_plan();
    let trial_ended = |plan| BillingOperation::TrialEnded {
        plan,
        next_renewal_at: None,
    };
    let cases = [
        (paid(renewal(get_free_plan(), 0, 0)), true),
        (paid(renewal(pro(), 0, 0)), true),
        (checkout(get_free_plan()), true),
        (trial_ended(get_free_plan()), true),
        (paid(renewal(pro(), 9999, 9999)), false),
        (paid(renewal(pro(), 9999, 0)), false),
        (paid(renewal(plus, 0, 0)), false),
        (checkout(pro()), false),
        (checkout(plus), false),
        (checkout(get_enterprise_plan()), false),
        (trial_ended(pro()), false),
        (trial_ended(get_enterprise_plan()), false),
    ];
    for (operation, expected) in cases {
        assert_eq!(operation.is_zero_dollar_notice(), expected, "{operation:?}");
    }
}
