use super::{Body, Content, Email, EmailCategory, EmailPreference};

/// A sent licence invoice passed its due date unpaid.
///
/// Distinct from [`super::SelfHostedPaymentFailed`], which tells a card
/// customer we could not take their payment and will retry: nobody attempted
/// a charge here, and the buyer's finance team is the one holding the bill.
/// The key keeps working for a grace period, so this names the date it stops
/// rather than treating the due date as a cut-off.
pub struct InvoiceOverdue<'a> {
    pub plan_name: &'a str,
    /// Formatted amount outstanding, e.g. "$4,000.00".
    pub amount: &'a str,
    /// Formatted due date that has passed, e.g. "October 18, 2026".
    pub due_date: &'a str,
    /// Formatted date the licence key stops working.
    pub key_expires: &'a str,
    /// Purchase order number printed on the invoice, when the buyer gave one.
    pub po_number: Option<&'a str>,
    /// Stripe's hosted invoice page, or the billing tab as a fallback.
    pub cta_href: &'a str,
}

impl Email for InvoiceOverdue<'_> {
    fn subject(&self) -> String {
        format!("Your Scanopy invoice of {} is overdue", self.amount)
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        "invoice_overdue"
    }

    fn body_html(&self) -> String {
        let mut content = Content::new()
            .heading("Your invoice is overdue")
            .paragraph("Hi there,")
            .paragraph(&format!(
                "The invoice for {} of {} was due on {} and we have not received payment.",
                self.plan_name, self.amount, self.due_date
            ));

        if let Some(po_number) = self.po_number {
            content = content.paragraph(&format!(
                "It was raised against your purchase order number, {}, if that helps your finance team find it.",
                po_number
            ));
        }

        content = content
            .paragraph(&format!(
                "Your license key still works until <strong>{}</strong>. After that your servers stop accepting it and go read-only.",
                self.key_expires
            ))
            .paragraph(
                "If the invoice is already with your finance team and simply awaiting a payment run, no action is needed beyond making sure it lands before that date.",
            );

        Body::new()
            .content(content)
            .cta(self.cta_href, "Pay Invoice")
            .render()
    }
}
