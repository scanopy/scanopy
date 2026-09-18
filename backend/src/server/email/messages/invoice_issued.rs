use super::{Body, Content, Email, EmailCategory, EmailPreference};

/// Confirms that a self-hosted licence invoice has been sent, and that the
/// licence key keeps working while the buyer's finance team pays it. Stripe
/// mails the invoice document itself; this one carries the licence context and
/// links to the same hosted invoice.
pub struct InvoiceIssued<'a> {
    pub plan_name: &'a str,
    /// Formatted invoice total, e.g. "$4,000.00".
    pub amount: &'a str,
    /// Formatted due date, e.g. "October 18, 2026".
    pub due_date: &'a str,
    /// Purchase order number printed on the invoice, when the buyer gave one.
    pub po_number: Option<&'a str>,
    /// Stripe's hosted invoice page, or the billing tab as a fallback.
    pub cta_href: &'a str,
}

impl Email for InvoiceIssued<'_> {
    fn subject(&self) -> String {
        format!(
            "Your Scanopy invoice: {} due {}",
            self.amount, self.due_date
        )
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        "invoice_issued"
    }

    fn body_html(&self) -> String {
        let mut content = Content::new()
            .heading("Your invoice is on its way")
            .paragraph("Hi there,")
            .paragraph(&format!(
                "We've sent an invoice for {} of {}, payable by {}.",
                self.plan_name, self.amount, self.due_date
            ));

        if let Some(po_number) = self.po_number {
            content = content.paragraph(&format!(
                "It carries your purchase order number, {}.",
                po_number
            ));
        }

        content = content.paragraph(
            "Your license key works throughout, so your servers keep running while the invoice is paid.",
        );

        Body::new()
            .content(content)
            .cta(self.cta_href, "View Invoice")
            .render()
    }
}
