use super::{Body, Content, Email, EmailCategory, EmailPreference, links};

/// Sent instead of the cloud cancelled / trial-expired emails when the
/// subscription that ended was for a self-hosted plan. Those emails describe
/// life on the Free cloud plan (25 hosts, manual discovery). This customer's
/// hosts are on their own server, and what they lose is the license key.
pub struct SelfHostedLicenseEnded<'a> {
    pub plan_name: &'a str,
    /// The subscription ended because a trial ran out with no payment method,
    /// as opposed to a paid subscription being cancelled.
    pub was_trial: bool,
    /// An online key is refused at the server's next check-in. An air-gapped
    /// key carries its expiry inside it and runs until then.
    pub air_gapped: bool,
}

impl Email for SelfHostedLicenseEnded<'_> {
    fn subject(&self) -> String {
        if self.was_trial {
            "Your Scanopy license trial has ended".to_string()
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
        } else {
            content.paragraph(
                "Your server stops accepting the license key at its next check-in, within a few hours, and goes read-only. Your data stays on your server.",
            )
        };

        content = content.paragraph(
            "Pick a self-hosted plan again to restore your license. Your Scanopy Cloud account is on the Free plan in the meantime.",
        );

        Body::new()
            .content(content)
            .cta(links::PLAN_PICKER, "Renew license")
            .render()
    }
}
