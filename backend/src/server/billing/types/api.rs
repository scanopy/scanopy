use crate::server::billing::types::base::{BillingPlan, BillingRate, CancelReason, SaveOffer};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateCheckoutRequest {
    /// Plan to subscribe to.
    pub plan: BillingPlan,
    /// URL to return the user to after checkout completes.
    pub url: String,
}

/// Pause subscription duration. The cancel modal's `RadioGroup` posts
/// one of these enum variants verbatim — no integer parsing at the API
/// boundary, the type is the contract.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PauseDuration {
    Days30,
    Days60,
    Days90,
}

impl PauseDuration {
    pub fn days(self) -> u32 {
        match self {
            Self::Days30 => 30,
            Self::Days60 => 60,
            Self::Days90 => 90,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PauseSubscriptionRequest {
    /// How long to pause billing for, in days.
    pub duration_days: PauseDuration,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CancelSubscriptionRequest {
    /// Why the customer is cancelling, as picked from the cancel flow.
    pub reason_code: CancelReason,
    /// Free-text detail the customer added to their cancellation reason.
    #[serde(default)]
    pub comment: Option<String>,
    /// Whether the retention discount was offered during this flow.
    #[serde(default)]
    pub save_offer_shown: Vec<SaveOffer>,
    /// Whether the customer accepted the retention discount instead of cancelling.
    #[serde(default)]
    pub save_offer_redeemed: Option<SaveOffer>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CancelSubscriptionResponse {
    /// When the current paid period ends and access drops to the free tier.
    pub period_end: DateTime<Utc>,
}

/// Response for creating a SetupIntent — the client secret the frontend
/// Payment Element uses to collect and confirm a card in-app.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SetupIntentResponse {
    /// Stripe SetupIntent client secret, used to mount the Payment Element.
    pub client_secret: String,
}

/// Request to finalize a client-confirmed SetupIntent (set the collected card
/// as the customer's default payment method).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FinalizePaymentMethodRequest {
    /// Stripe SetupIntent to attach as the organization's payment method.
    pub setup_intent_id: String,
}

/// Postal address of the billing entity, as Stripe's Address Element returns it.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InvoiceBillingAddress {
    pub line1: String,
    #[serde(default)]
    pub line2: Option<String>,
    pub city: String,
    #[serde(default)]
    pub state: Option<String>,
    pub postal_code: String,
    /// Two-letter ISO country code.
    pub country: String,
}

/// Tax ID of the billing entity, as Stripe's Tax ID Element returns it.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InvoiceBillingTaxId {
    /// Stripe tax ID type, e.g. `eu_vat` or `us_ein`.
    pub tax_id_type: String,
    pub value: String,
}

/// Who an invoice is addressed to and what the buyer's finance team matches
/// it against.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InvoiceBillingDetails {
    /// Legal name of the organization being invoiced.
    pub entity_name: String,
    /// Where Stripe emails invoices.
    #[schema(format = "email")]
    pub billing_email: String,
    pub address: InvoiceBillingAddress,
    #[serde(default)]
    pub tax_id: Option<InvoiceBillingTaxId>,
    /// Purchase order number printed on every invoice.
    #[serde(default)]
    pub po_number: Option<String>,
}

/// What to do once the billing entity is recorded.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum InvoiceBillingMode {
    /// Bill the subscription by invoice and issue the first invoice now.
    SendInvoice,
    /// Issue a quote the buyer's procurement raises a purchase order against.
    Quote,
}

/// Switch the organization to paying by invoice.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InvoiceBillingRequest {
    pub details: InvoiceBillingDetails,
    pub mode: InvoiceBillingMode,
    /// Plan to invoice for. Required only when the organization has no live
    /// subscription (a returning customer with no trial left); otherwise the
    /// current plan is used and this is ignored.
    #[serde(default)]
    pub plan: Option<BillingPlan>,
}

/// An open quote waiting for the buyer's purchase order.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PendingQuote {
    /// Quote number printed on the PDF, which the purchase order references.
    pub number: Option<String>,
    /// Total per annual term, in cents.
    pub amount_total_cents: i64,
    pub currency: String,
    pub expires_at: DateTime<Utc>,
}

/// An issued invoice that has not been paid.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InvoiceSummary {
    pub number: Option<String>,
    pub amount_due_cents: i64,
    pub currency: String,
    pub due_date: Option<DateTime<Utc>>,
    /// Stripe-hosted page where the invoice can be viewed and paid.
    pub hosted_invoice_url: Option<String>,
}

impl From<&stripe_billing::Invoice> for InvoiceSummary {
    fn from(invoice: &stripe_billing::Invoice) -> Self {
        Self {
            number: invoice.number.clone(),
            amount_due_cents: invoice.amount_due,
            currency: invoice.currency.to_string(),
            due_date: invoice
                .due_date
                .and_then(|ts| DateTime::<Utc>::from_timestamp(ts, 0)),
            hosted_invoice_url: invoice.hosted_invoice_url.clone(),
        }
    }
}

/// Invoice billing state shown on the License tab.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InvoiceBillingStatus {
    /// The subscription is billed by sent invoice.
    pub bills_by_invoice: bool,
    /// Purchase order number printed on invoices.
    pub po_number: Option<String>,
    pub open_invoice: Option<InvoiceSummary>,
    /// An invoice this customer defaulted on and we wrote off. While one
    /// stands, they cannot take payment terms again.
    pub written_off_invoice: Option<InvoiceSummary>,
    pub pending_quote: Option<PendingQuote>,
}

/// Replace the purchase order number printed on future invoices.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdatePoNumberRequest {
    /// New PO number; empty or absent removes it.
    #[serde(default)]
    pub po_number: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChangePlanRequest {
    /// Plan to move the subscription to.
    pub plan: BillingPlan,
    /// Billing interval to move to.
    pub rate: BillingRate,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChangePlanPreview {
    /// Hosts over the target plan's allowance, which would be billed as overage.
    pub excess_hosts: u64,
    /// Networks over the target plan's allowance.
    pub excess_networks: u64,
    /// Seats over the target plan's allowance.
    pub excess_seats: u64,
}

/// Live terms for the configured save-offer coupon, read directly from
/// Stripe. Used by the cancel modal's Discount panel to render the offer
/// dynamically instead of hard-coding the percent/duration.
///
/// Only returned when the coupon would actually catch the user's next
/// invoice — i.e. `next_renewal_at` falls within the coupon's `duration_in_months`
/// window. Yearly subscribers partway through a cycle whose next renewal
/// lands after the coupon's window get `None` from the endpoint and the
/// cancel modal's Discount panel doesn't render.
///
/// `billing_rate` lets the frontend pick monthly vs yearly copy: a monthly
/// subscriber thinks in terms of "N months of discount"; a yearly subscriber
/// thinks in terms of "my next renewal on {date}."
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SaveOfferCoupon {
    /// Discount applied by the retention offer.
    pub percent_off: i64,
    /// How many months the discount lasts.
    pub duration_in_months: i64,
    /// When the discounted subscription next renews.
    pub next_renewal_at: DateTime<Utc>,
    /// Billing interval the discount applies to.
    pub billing_rate: BillingRate,
}
