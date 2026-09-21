use super::{BILLING_DETAILS_TAGLINE, Body, Content, Email, EmailCategory, EmailPreference, links};

/// Sent when a trial converts to a paid subscription: confirms the active plan.
pub struct TrialConverted<'a> {
    pub plan_name: &'a str,
    pub billing_period: &'a str,
}

impl Email for TrialConverted<'_> {
    fn subject(&self) -> String {
        "Your Subscription is Now Active".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        "trial_converted"
    }

    fn body_html(&self) -> String {
        Body::new()
            .content(
                Content::new()
                    .heading("Your Subscription is Active!")
                    .paragraph("Hi there,")
                    .paragraph(&format!(
                        "Your {} {} trial has ended and your subscription is now active.",
                        self.plan_name, self.billing_period
                    ))
                    .paragraph(BILLING_DETAILS_TAGLINE),
            )
            .cta(links::SETTINGS_BILLING, "View Billing")
            .render()
    }
}
