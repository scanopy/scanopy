use super::{Body, Content, Email, EmailCategory, EmailPreference, links};

/// Sent when a trial ends without conversion: the org keeps its plan and is
/// read-only until it chooses a paid plan, with a CTA to the plan picker.
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
                        "Your {} {} trial has ended. Your account is now read-only: you can still see everything Scanopy found, but scans, edits and daemon work are paused.",
                        self.plan_name, self.billing_period
                    ))
                    .paragraph("Choose a paid plan from Settings to pick up where the trial left off. Your networks, hosts and schedules are all still there."),
            )
            .cta(links::PLAN_PICKER, "Choose a plan")
            .render()
    }
}
