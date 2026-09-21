use super::{Body, Content, Email, EmailCategory, EmailPreference, links};

/// Sent when a payment fails: prompts the user to update their payment method to
/// avoid service interruption.
pub struct PaymentFailed;

impl Email for PaymentFailed {
    fn subject(&self) -> String {
        "Payment Failed - Action Required".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        "payment_failed"
    }

    fn body_html(&self) -> String {
        Body::new()
            .content(
                Content::new()
                    .heading("Payment Failed")
                    .paragraph("Hi there,")
                    .paragraph("Your recent payment for Scanopy failed. Please update your payment method to avoid service interruption.")
                    .paragraph("If you believe this is an error, check with your bank or try a different payment method."),
            )
            .cta(links::SETTINGS_BILLING, "Update Payment Method")
            .render()
    }
}
