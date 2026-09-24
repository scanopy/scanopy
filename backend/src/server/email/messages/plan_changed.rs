use super::{Body, Content, Email, EmailCategory, EmailPreference, links};

/// Sent when an organization's plan changes: confirms the new plan and that the
/// change is effective immediately.
pub struct PlanChanged<'a> {
    pub plan_name: &'a str,
    /// The organization moved off a self-hosted plan, so the license key on
    /// its own server is refused from the next check-in.
    pub license_key_stops: bool,
}

impl Email for PlanChanged<'_> {
    fn subject(&self) -> String {
        "Your Plan Has Changed".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        "plan_changed"
    }

    fn body_html(&self) -> String {
        let mut content = Content::new()
            .heading("Plan Updated")
            .paragraph("Hi there,")
            .paragraph(&format!(
                "Your Scanopy plan has been changed to {}. The change takes effect immediately.",
                self.plan_name
            ));

        if self.license_key_stops {
            content = content.paragraph(
                "Your self-hosted license ended with this change. Your own server stops accepting its license key at the next check-in, within a few hours, and goes read-only.",
            );
        }

        Body::new()
            .content(content)
            .cta(links::APP_HOME, "Open Scanopy")
            .render()
    }
}
