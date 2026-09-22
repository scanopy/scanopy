use super::{Body, Content, Email, EmailCategory, EmailPreference, links};

/// Confirms a subscription is scheduled to cancel at the end of the current
/// billing period.
pub struct CancellationInitiated<'a> {
    pub period_end: &'a str,
    /// The subscription is for a self-hosted plan. What runs until the period
    /// ends is the license key, and the billing-plan deep link is overridden
    /// by the license lock, so the button goes to Settings instead.
    pub licensed: bool,
}

impl Email for CancellationInitiated<'_> {
    fn subject(&self) -> String {
        format!("Your Subscription Will End on {}", self.period_end)
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        "cancellation_initiated"
    }

    fn body_html(&self) -> String {
        let content = Content::new()
            .heading("Cancellation Scheduled")
            .paragraph("Hi there,");

        let (content, cta_href) = if self.licensed {
            (
                content
                    .paragraph(&format!(
                        "Your Scanopy license subscription is scheduled to cancel on <strong>{}</strong>. Your license key keeps working until then. After that your server stops accepting it and goes read-only.",
                        self.period_end
                    ))
                    .paragraph("Changed your mind? You can keep the subscription any time before then from your billing settings."),
                links::SETTINGS_LICENSE,
            )
        } else {
            (
                content
                    .paragraph(&format!(
                        "Your Scanopy subscription is scheduled to cancel on <strong>{}</strong>. You'll keep full access until then; after that you'll move to the Free plan.",
                        self.period_end
                    ))
                    .paragraph("Changed your mind? You can resubscribe or switch plans any time from your billing settings."),
                links::PLAN_PICKER,
            )
        };

        Body::new()
            .content(content)
            .cta(cta_href, "Manage Subscription")
            .render()
    }
}
