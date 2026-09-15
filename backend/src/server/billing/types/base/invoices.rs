//! Domain invoice snapshot types and Stripe conversions.
use super::*;
use strum::IntoEnumIterator;

// ===========================================================================
// Domain invoice snapshot — typed projection of `stripe_billing::Invoice` for
// event payloads. Carries exactly the fields the usage-summary email needs to
// render the line-item breakdown without reaching back into Stripe.
// ===========================================================================

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Display, VariantNames,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum BillingReason {
    /// Recurring renewal — triggers the usage-summary email.
    SubscriptionCycle,
    /// Initial subscription creation invoice.
    SubscriptionCreate,
    /// Plan change / proration invoice.
    SubscriptionUpdate,
    /// Manually-issued invoice.
    Manual,
    /// Anything else Stripe sends us.
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct BillingInvoiceLineItem {
    pub description: Option<String>,
    pub amount_cents: i64,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    /// Stripe product the line's price belongs to (`BillingPlan::stripe_product_id`
    /// for plan lines). `None` for lines without a price, and on events
    /// published before this field existed.
    #[serde(default)]
    pub product: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct BillingInvoice {
    pub stripe_invoice_id: String,
    pub amount_paid_cents: i64,
    pub currency: String,
    pub created_at: DateTime<Utc>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub billing_reason: BillingReason,
    pub line_items: Vec<BillingInvoiceLineItem>,
    /// Public link to Stripe's rendered PDF for this invoice. Stripe generates
    /// it lazily, so it can be `None` immediately after payment.
    pub invoice_pdf: Option<String>,
    /// Public link to Stripe's hosted invoice page — the fallback when the PDF
    /// isn't ready in time to attach.
    pub hosted_invoice_url: Option<String>,
}

impl BillingInvoice {
    /// End of the latest service period this invoice pays for on a
    /// self-hosted license plan, or `None` when no line is for one. Read from
    /// the invoice's own lines rather than the org row: `invoice.paid` can
    /// arrive before the subscription webhook that moves the org onto the
    /// plan. Uses line periods, not the invoice-level period, which on a
    /// subscription's first invoice is just its creation time.
    pub fn license_paid_through(&self) -> Option<DateTime<Utc>> {
        let licensed_products: Vec<String> = BillingPlan::iter()
            .filter(|plan| plan.license_plan().is_some())
            .map(|plan| plan.stripe_product_id())
            .collect();

        self.line_items
            .iter()
            .filter(|line| {
                line.product
                    .as_ref()
                    .is_some_and(|product| licensed_products.contains(product))
            })
            .map(|line| line.period_end)
            .max()
    }
}

// Stripe ships unix-epoch i64 timestamps; fall back to `Utc::now()` on a
// malformed value rather than failing the event publish.
fn ts_to_chrono(ts: i64) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
}

impl From<&stripe_billing::Invoice> for BillingInvoice {
    fn from(inv: &stripe_billing::Invoice) -> Self {
        Self {
            stripe_invoice_id: inv.id.as_ref().map(|id| id.to_string()).unwrap_or_default(),
            amount_paid_cents: inv.amount_paid,
            currency: inv.currency.to_string(),
            created_at: ts_to_chrono(inv.created),
            period_start: ts_to_chrono(inv.period_start),
            period_end: ts_to_chrono(inv.period_end),
            billing_reason: inv.billing_reason.into(),
            line_items: inv
                .lines
                .data
                .iter()
                .map(BillingInvoiceLineItem::from)
                .collect(),
            invoice_pdf: inv.invoice_pdf.clone(),
            hosted_invoice_url: inv.hosted_invoice_url.clone(),
        }
    }
}

impl From<&stripe_billing::InvoiceLineItem> for BillingInvoiceLineItem {
    fn from(item: &stripe_billing::InvoiceLineItem) -> Self {
        Self {
            description: item.description.clone(),
            amount_cents: item.amount,
            period_start: ts_to_chrono(item.period.start),
            period_end: ts_to_chrono(item.period.end),
            product: item
                .pricing
                .as_ref()
                .and_then(|pricing| pricing.price_details.as_ref())
                .map(|details| details.product.clone()),
        }
    }
}

impl From<Option<stripe_billing::InvoiceBillingReason>> for BillingReason {
    fn from(reason: Option<stripe_billing::InvoiceBillingReason>) -> Self {
        use stripe_billing::InvoiceBillingReason::*;
        match reason {
            Some(SubscriptionCycle) => Self::SubscriptionCycle,
            Some(SubscriptionCreate) => Self::SubscriptionCreate,
            Some(SubscriptionUpdate) => Self::SubscriptionUpdate,
            Some(Manual) => Self::Manual,
            _ => Self::Other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::billing::plans::{get_enterprise_plan, get_self_hosted_standard_plan};

    fn line(product: Option<String>, period_end: DateTime<Utc>) -> BillingInvoiceLineItem {
        BillingInvoiceLineItem {
            description: None,
            amount_cents: 0,
            period_start: period_end - chrono::Duration::days(365),
            period_end,
            product,
        }
    }

    fn invoice(line_items: Vec<BillingInvoiceLineItem>) -> BillingInvoice {
        let now = Utc::now();
        BillingInvoice {
            stripe_invoice_id: "in_test".to_string(),
            amount_paid_cents: 0,
            currency: "usd".to_string(),
            created_at: now,
            period_start: now,
            period_end: now,
            billing_reason: BillingReason::SubscriptionCreate,
            line_items,
            invoice_pdf: None,
            hosted_invoice_url: None,
        }
    }

    #[test]
    fn license_paid_through_takes_latest_self_hosted_line() {
        let now = Utc::now();
        let self_hosted = Some(get_self_hosted_standard_plan().stripe_product_id());
        let cloud = Some(get_enterprise_plan().stripe_product_id());

        // A proration invoice: credit for the old period, charge for the new.
        let paid = invoice(vec![
            line(self_hosted.clone(), now + chrono::Duration::days(30)),
            line(self_hosted, now + chrono::Duration::days(365)),
            line(cloud, now + chrono::Duration::days(900)),
        ]);
        assert_eq!(
            paid.license_paid_through(),
            Some(now + chrono::Duration::days(365))
        );
    }

    #[test]
    fn license_paid_through_ignores_cloud_and_priceless_lines() {
        let now = Utc::now();
        let paid = invoice(vec![
            line(Some(get_enterprise_plan().stripe_product_id()), now),
            line(None, now),
        ]);
        assert_eq!(paid.license_paid_through(), None);
    }
}
