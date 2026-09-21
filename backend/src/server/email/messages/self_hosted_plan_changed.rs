use super::{Body, Content, Email, EmailCategory, EmailPreference, links};

/// Sent instead of the cloud plan-changed email when an organization moves
/// between two self-hosted plans. The cloud email's "Open Scanopy" button
/// leads into an app these organizations are locked out of, and what they
/// need to know is how the new plan reaches their server.
pub struct SelfHostedPlanChanged<'a> {
    pub plan_name: &'a str,
    /// An online key embeds no plan, so the server picks the change up at its
    /// next check-in. An air-gapped key embeds its entitlement and has to be
    /// copied again.
    pub air_gapped: bool,
}

impl Email for SelfHostedPlanChanged<'_> {
    fn subject(&self) -> String {
        format!("Your Scanopy license is now {}", self.plan_name)
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        "self_hosted_plan_changed"
    }

    fn body_html(&self) -> String {
        let mut content = Content::new()
            .heading("License plan updated")
            .paragraph("Hi there,")
            .paragraph(&format!(
                "Your Scanopy license has been changed to {}.",
                self.plan_name
            ));

        content = if self.air_gapped {
            content.paragraph(
                "You are using an air-gapped key. Copy the new key from Settings and set it on your server: air-gapped keys carry their plan inside them and cannot update themselves.",
            )
        } else {
            content.paragraph(
                "Your license key stays the same. Your server picks up the new plan at its next check-in, within a few hours.",
            )
        };

        Body::new()
            .content(content)
            .cta(links::SETTINGS_LICENSE, "View your license")
            .render()
    }
}
