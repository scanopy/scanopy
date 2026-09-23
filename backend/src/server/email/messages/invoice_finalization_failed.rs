use super::{Body, Content, Email, EmailCategory, EmailPreference};

/// Stripe refused to finalize a licence invoice, so it was never issued.
///
/// The app has already told the buyer the invoice was sent, and nothing else
/// reports this: no invoice arrives, and no licence period is granted.
///
/// The copy names no cause of its own. Finalization fails when Stripe cannot
/// compute the invoice, and which field is at fault is in Stripe's own reason,
/// so the email quotes that and points at the billing details rather than
/// guessing. A tax ID is *not* a likely cause: Stripe validates those
/// asynchronously for display and does not block finalization on them.
pub struct InvoiceFinalizationFailed<'a> {
    pub plan_name: &'a str,
    /// Stripe's own reason, shown verbatim: it names the field at fault.
    pub reason: &'a str,
}

impl Email for InvoiceFinalizationFailed<'_> {
    fn subject(&self) -> String {
        "We could not issue your Scanopy invoice".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        "invoice_finalization_failed"
    }

    fn body_html(&self) -> String {
        let content = Content::new()
            .heading("We could not issue your invoice")
            .paragraph("Hi there,")
            .paragraph(&format!(
                "Your invoice for {} could not be issued, so nothing has been sent and nothing has been charged.",
                self.plan_name
            ))
            .paragraph(&format!("Stripe reported: {}", self.reason))
            .paragraph(
                "Check your billing details in Settings, then request the invoice again. Reply to this email if the reason above does not point at something you can correct.",
            );

        Body::new()
            .content(content)
            .cta(super::links::SETTINGS_BILLING, "Check your billing details")
            .render()
    }
}
