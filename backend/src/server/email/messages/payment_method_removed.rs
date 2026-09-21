use super::{Body, Content, Email, EmailCategory, EmailPreference, links};

/// Notifies the user that a payment method was removed from their account.
pub struct PaymentMethodRemoved;

impl Email for PaymentMethodRemoved {
    fn subject(&self) -> String {
        "Payment Method Removed".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        "payment_method_removed"
    }

    fn body_html(&self) -> String {
        Body::new()
            .content(
                Content::new()
                    .heading("Payment Method Removed")
                    .paragraph("Hi there,")
                    .paragraph("A payment method has been removed from your Scanopy account.")
                    .paragraph("If this wasn't you, sign in to your account, add a payment method back, and review your active sessions in Settings."),
            )
            .cta(links::SETTINGS_BILLING, "Manage Payment Method")
            .render()
    }
}
