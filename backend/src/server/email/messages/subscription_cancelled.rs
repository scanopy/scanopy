use super::{Body, Content, Email, EmailCategory, EmailPreference, links};

/// Sent when a cancelled subscription reaches its period end: the org keeps
/// its plan and is read-only until it chooses a paid plan, with a resubscribe
/// CTA.
pub struct SubscriptionCancelled<'a> {
    pub period_end_date: &'a str,
}

impl Email for SubscriptionCancelled<'_> {
    fn subject(&self) -> String {
        "Your Subscription Has Been Cancelled".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        "subscription_cancelled"
    }

    fn body_html(&self) -> String {
        Body::new()
            .content(
                Content::new()
                    .heading("Subscription Cancelled")
                    .paragraph("Hi there,")
                    .paragraph(&format!(
                        "Your Scanopy subscription ended on {}. Your account is now read-only: you can still see everything Scanopy found, but scans, edits and daemon work are paused.",
                        self.period_end_date
                    ))
                    .paragraph("Choose a paid plan from Settings to resume. Your networks, hosts and schedules are all still there."),
            )
            .cta(links::PLAN_PICKER, "Resubscribe")
            .render()
    }
}
