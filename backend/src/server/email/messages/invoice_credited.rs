use super::{Body, Content, Email, EmailCategory, EmailPreference, links};

/// Sent when a plan change leaves the customer owing nothing: Stripe credits
/// the unused part of what they already paid, and the credit comes off their
/// next renewal. The invoice exists, so without this they would hear nothing,
/// or worse, be told to pay a negative amount.
pub struct InvoiceCredited<'a> {
    pub plan_name: &'a str,
    /// Formatted credit, e.g. "$1,999.98".
    pub credit: &'a str,
}

impl Email for InvoiceCredited<'_> {
    fn subject(&self) -> String {
        format!("Your Scanopy account has a {} credit", self.credit)
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        // Money owed to the customer: never suppressed.
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        "invoice_credited"
    }

    fn body_html(&self) -> String {
        Body::new()
            .content(
                Content::new()
                    .heading("Nothing to pay")
                    .paragraph("Hi there,")
                    .paragraph(&format!(
                        "Your move to {} left {} unused, held as credit against your next renewal. There is nothing to pay now.",
                        self.plan_name, self.credit
                    )),
            )
            .cta(links::SETTINGS_BILLING, "View Billing")
            .render()
    }
}
