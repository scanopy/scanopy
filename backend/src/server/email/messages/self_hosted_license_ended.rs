use super::{Body, Content, Email, EmailCategory, EmailPreference, links};

/// Sent instead of the cloud cancelled / trial-expired emails when the
/// subscription that ended was for a self-hosted plan. Those emails describe
/// a read-only cloud account. This customer's hosts are on their own server,
/// and what they lose is the license key.
pub struct SelfHostedLicenseEnded<'a> {
    pub plan_name: &'a str,
    /// The subscription ended because a trial ran out with no payment method,
    /// as opposed to a paid subscription being cancelled.
    pub was_trial: bool,
    /// An online key keeps working until `key_expires`, the end of the period
    /// the org paid (or trialled) for plus the usual buffer, because the org
    /// keeps its plan and the cloud keeps serving its entitlement until then.
    /// An air-gapped key carries its expiry inside it and runs until then.
    pub air_gapped: bool,
    /// Formatted date the online key stops working. Unused when `defaulted`:
    /// nothing was paid for, so there is no date to run to.
    pub key_expires: &'a str,
    /// The invoice went unpaid and was written off, so the licence stopped
    /// with it. An air-gapped key is unaffected either way — one is only ever
    /// minted against a period already settled, so the key the customer holds
    /// was paid for and runs to its own expiry regardless.
    pub defaulted: bool,
}

impl Email for SelfHostedLicenseEnded<'_> {
    fn subject(&self) -> String {
        if self.was_trial {
            "Your Scanopy license trial has ended".to_string()
        } else if self.defaulted {
            "Your Scanopy license has stopped".to_string()
        } else {
            "Your Scanopy license has ended".to_string()
        }
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        "self_hosted_license_ended"
    }

    fn body_html(&self) -> String {
        let (heading, opening) = if self.was_trial {
            (
                "Your license trial has ended",
                format!(
                    "Your {} trial ended without a payment method on file, so the license was not renewed.",
                    self.plan_name
                ),
            )
        } else if self.defaulted {
            (
                "Your license has stopped",
                format!(
                    "Your invoice for {} went unpaid past its due date, so we have written it off and the license stopped with it.",
                    self.plan_name
                ),
            )
        } else {
            (
                "Your license has ended",
                format!(
                    "Your {} subscription has been cancelled, and the license ended with it.",
                    self.plan_name
                ),
            )
        };

        let mut content = Content::new()
            .heading(heading)
            .paragraph("Hi there,")
            .paragraph(&opening);

        content = if self.air_gapped {
            content.paragraph(
                "You are using an air-gapped key. It keeps working until the expiry date it was issued with, and after that your server stops accepting it and goes read-only.",
            )
        } else if self.defaulted {
            content.paragraph(
                "Your license key has stopped, so your server is read-only from its next check-in. Your data stays on your server.",
            )
        } else {
            content.paragraph(&format!(
                "Your license key keeps working until {}. After that your server stops accepting it and goes read-only. Your data stays on your server.",
                self.key_expires
            ))
        };

        content = if self.defaulted {
            content.paragraph(&format!(
                "Pay the invoice from Settings to start your {} plan and license again, or contact billing@scanopy.net if this is wrong.",
                self.plan_name
            ))
        } else {
            content.paragraph(&format!(
                "Your Scanopy Cloud account stays on {}. Renew from Settings to keep the same key working, or choose another self-hosted plan.",
                self.plan_name
            ))
        };

        Body::new()
            .content(content)
            .cta(
                if self.defaulted {
                    links::SETTINGS_BILLING
                } else {
                    links::SETTINGS_LICENSE
                },
                if self.defaulted {
                    "Pay invoice"
                } else {
                    "Renew license"
                },
            )
            .render()
    }
}
