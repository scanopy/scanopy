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
    /// for plan lines). `None` for a line without a price.
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
    /// How Stripe collects this invoice. Defaulted for events published
    /// before this field existed, all of which were card charges.
    #[serde(default)]
    pub collection: InvoiceCollection,
    /// When a sent invoice is due. `None` for invoices charged automatically.
    #[serde(default)]
    pub due_date: Option<DateTime<Utc>>,
    /// Purchase order number printed on this invoice, stamped from the
    /// customer's invoice custom fields when Stripe finalized it.
    #[serde(default)]
    pub po_number: Option<String>,
    /// What the customer actually owes on this invoice, as Stripe computed it.
    /// A proration that credits more than it charges comes to zero here, with
    /// the difference going to the customer's balance rather than a refund.
    #[serde(default)]
    pub amount_due_cents: i64,
    /// The invoice's total, which is negative when it is net a credit. Summing
    /// the line items is not the same thing: a proration carries a credit line
    /// beside a charge line, and Stripe applies its own rounding.
    #[serde(default)]
    pub total_cents: i64,
}

/// Name of the invoice custom field carrying the buyer's purchase order.
pub const PO_NUMBER_FIELD: &str = "PO Number";

/// How Stripe collects payment for an invoice.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum InvoiceCollection {
    /// Charged to the customer's saved payment method.
    #[default]
    ChargeAutomatically,
    /// Emailed to the customer, who pays it by its due date.
    SendInvoice,
}

impl From<stripe_shared::InvoiceCollectionMethod> for InvoiceCollection {
    fn from(method: stripe_shared::InvoiceCollectionMethod) -> Self {
        match method {
            stripe_shared::InvoiceCollectionMethod::SendInvoice => Self::SendInvoice,
            _ => Self::ChargeAutomatically,
        }
    }
}

/// Days a sent, unpaid self-hosted invoice keeps the license valid past its
/// due date. Public-sector payment runs routinely miss net-30, and minted keys
/// add their own buffer and grace window on top.
pub const INVOICE_PAYMENT_GRACE_DAYS: i64 = 30;

impl BillingInvoice {
    /// End of the latest service period this invoice pays for on a
    /// self-hosted license plan, or `None` when no line is for one. Read from
    /// the invoice's own lines rather than the org row: `invoice.paid` can
    /// arrive before the subscription webhook that moves the org onto the
    /// plan. Uses line periods, not the invoice-level period, which on a
    /// subscription's first invoice is just its creation time.
    pub fn license_paid_through(&self) -> Option<DateTime<Utc>> {
        self.licensed_lines().map(|line| line.period_end).max()
    }

    /// How long a sent, unpaid invoice for a self-hosted license keeps that
    /// license valid: its due date plus [`INVOICE_PAYMENT_GRACE_DAYS`]. `None`
    /// for invoices charged automatically, invoices with no due date, and
    /// invoices with no self-hosted license line.
    pub fn provisional_paid_through(&self) -> Option<DateTime<Utc>> {
        if self.collection != InvoiceCollection::SendInvoice {
            return None;
        }
        self.licensed_lines().next()?;
        self.due_date
            .map(|due| due + chrono::Duration::days(INVOICE_PAYMENT_GRACE_DAYS))
    }

    /// Start of the earliest service period this invoice bills on a
    /// self-hosted license plan. Everything before it was already paid (or was
    /// a trial), so an unpaid or voided invoice licenses nothing past it.
    pub fn license_unpaid_from(&self) -> Option<DateTime<Utc>> {
        self.licensed_lines().map(|line| line.period_start).min()
    }

    fn licensed_lines(&self) -> impl Iterator<Item = &BillingInvoiceLineItem> {
        let licensed_products: Vec<String> = BillingPlan::iter()
            .filter(|plan| plan.license_plan().is_some())
            .map(|plan| plan.stripe_product_id())
            .collect();

        self.line_items.iter().filter(move |line| {
            line.product
                .as_ref()
                .is_some_and(|product| licensed_products.contains(product))
        })
    }
}

// Stripe ships unix-epoch i64 timestamps; fall back to `Utc::now()` on a
// malformed value rather than failing the event publish.
pub(crate) fn ts_to_chrono(ts: i64) -> DateTime<Utc> {
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
            collection: inv.collection_method.into(),
            due_date: inv.due_date.map(ts_to_chrono),
            po_number: inv
                .custom_fields
                .as_ref()
                .and_then(|fields| fields.iter().find(|field| field.name == PO_NUMBER_FIELD))
                .map(|field| field.value.clone()),
            amount_due_cents: inv.amount_due,
            total_cents: inv.total,
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
            collection: InvoiceCollection::ChargeAutomatically,
            po_number: None,
            amount_due_cents: 0,
            total_cents: 0,
            due_date: None,
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

    #[test]
    fn provisional_paid_through_needs_a_sent_self_hosted_invoice_with_a_due_date() {
        let due = Utc::now() + chrono::Duration::days(30);
        let self_hosted = Some(get_self_hosted_standard_plan().stripe_product_id());
        let sent = |line_items, due_date| BillingInvoice {
            collection: InvoiceCollection::SendInvoice,
            due_date,
            ..invoice(line_items)
        };

        let open = sent(vec![line(self_hosted.clone(), due)], Some(due));
        assert!(open.provisional_paid_through().is_some_and(|d| d > due));

        let charged = BillingInvoice {
            due_date: Some(due),
            ..invoice(vec![line(self_hosted.clone(), due)])
        };
        assert_eq!(charged.provisional_paid_through(), None);

        let cloud_only = sent(
            vec![line(Some(get_enterprise_plan().stripe_product_id()), due)],
            Some(due),
        );
        assert_eq!(cloud_only.provisional_paid_through(), None);

        assert_eq!(
            sent(vec![line(self_hosted, due)], None).provisional_paid_through(),
            None
        );
    }
}
