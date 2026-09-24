use super::{Body, Content, Email, EmailCategory, EmailPreference, links};

/// Sent instead of the cloud payment-failed email when the organization is on
/// a self-hosted plan. What breaks for these customers is their own server,
/// not their access to Scanopy Cloud, so the copy says when the key stops
/// working rather than talking about account access.
pub struct SelfHostedPaymentFailed<'a> {
    /// The date the licence is paid through, after which keys stop being valid.
    pub key_expires: &'a str,
    /// Air-gapped keys need a fresh copy by hand once payment is fixed; online
    /// keys pick the renewal up on their own.
    pub air_gapped: bool,
}

impl Email for SelfHostedPaymentFailed<'_> {
    fn subject(&self) -> String {
        "Your Scanopy license key will stop working".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        "self_hosted_payment_failed"
    }

    fn body_html(&self) -> String {
        let mut content = Content::new()
            .heading("We could not take your payment")
            .paragraph("Hi there,")
            .paragraph(&format!(
                "The payment for your Scanopy license did not go through. Your license runs until <strong>{}</strong>, and after that your server stops accepting the key and goes read-only.",
                self.key_expires
            ))
            .paragraph(
                "Update your payment method in Settings and we will retry the charge.",
            );

        content = if self.air_gapped {
            content.paragraph(
                "You are using an air-gapped key. Once the payment succeeds, copy the new key from Settings and set it on your server.",
            )
        } else {
            content.paragraph(
                "Once the payment succeeds your server picks up the new license on its own, within a few hours.",
            )
        };

        Body::new()
            .content(content)
            .cta(links::SETTINGS_BILLING, "Update payment method")
            .render()
    }
}
