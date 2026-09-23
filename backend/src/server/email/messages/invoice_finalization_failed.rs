use super::{Body, Content, Email, EmailCategory, EmailPreference};

/// Stripe refused to finalize a licence invoice, so it was never issued.
///
/// The app has already told the buyer the invoice was sent, and nothing else
/// reports this: no invoice arrives, and no licence period is granted. A
/// rejected tax ID is the usual cause, so the copy points at the billing
/// details rather than at payment.
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
                "Check your billing details in Settings, then request the invoice again. A tax ID that fails validation is the most common cause.",
            );

        Body::new()
            .content(content)
            .cta(super::links::SETTINGS_BILLING, "Check your billing details")
            .render()
    }
}
