use super::{Body, Content, Email, EmailCategory, EmailPreference, links};

/// Sent when a trial ends without conversion: account moved to Free, with an
/// upgrade CTA to restore higher limits and scheduled discovery.
pub struct TrialExpired<'a> {
    pub plan_name: &'a str,
    pub billing_period: &'a str,
}

impl Email for TrialExpired<'_> {
    fn subject(&self) -> String {
        "Your Trial Has Ended".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        "trial_expired"
    }

    fn body_html(&self) -> String {
        Body::new()
            .content(
                Content::new()
                    .heading("Your Trial Has Ended")
                    .paragraph("Hi there,")
                    .paragraph(&format!(
                        "Your {} {} trial has ended and your account has been moved to the Free plan.",
                        self.plan_name, self.billing_period
                    ))
                    .paragraph("You can still use Scanopy with up to 25 hosts and manual discovery. Upgrade anytime to restore scheduled discovery and higher limits."),
            )
            .cta(links::PLAN_PICKER, "Upgrade Plan")
            .render()
    }
}
