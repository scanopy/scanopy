use super::{Body, Content, Email, EmailCategory, EmailPreference, links};

/// Tells a self-hosted buyer their licence is back after they settled the
/// invoice we had written off. Fired by the email subscriber on
/// `BillingOperation::LicenseResumed`.
///
/// Air-gapped holders need a second step: their key carries its own expiry, so
/// a resumed date on the server changes nothing until they copy the new key.
pub struct SelfHostedLicenseResumed<'a> {
    pub plan_name: &'a str,
    pub resumes_through: &'a str,
    pub air_gapped: bool,
}

impl Email for SelfHostedLicenseResumed<'_> {
    fn subject(&self) -> String {
        "Your Scanopy License is Active Again".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        "self_hosted_license_resumed"
    }

    fn body_html(&self) -> String {
        let mut content = Content::new()
            .heading("Your license is back")
            .paragraph("Hi there,")
            .paragraph(&format!(
                "Thank you for settling your invoice. Your {} plan is active again, and your license runs through {}. The time your servers were locked has been added on, so you get the full year you paid for.",
                self.plan_name, self.resumes_through
            ));

        content = if self.air_gapped {
            content.paragraph(
                "You are using an air-gapped key, which carries its own expiry date. Copy the new key from Settings and install it on your server, or it will keep running to the old date.",
            )
        } else {
            content.paragraph(
                "Your server picks this up on its own at its next check-in, within a few hours. Nothing to install.",
            )
        };

        content = content.paragraph(&format!(
            "Your next invoice is due when this term ends on {}, payable within 30 days as before.",
            self.resumes_through
        ));

        Body::new()
            .content(content)
            .cta(links::SETTINGS_LICENSE, "View license")
            .render()
    }
}
