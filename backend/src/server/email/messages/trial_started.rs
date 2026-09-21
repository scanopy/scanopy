use super::{
    BILLING_DETAILS_TAGLINE, Body, Content, Email, EmailCategory, EmailPreference,
    PausableCategory, links,
};

/// Sent when a trial begins: welcomes the user and points them at adding a
/// payment method before the trial ends.
pub struct TrialStarted<'a> {
    pub plan_name: &'a str,
    pub trial_days: u32,
    pub billing_period: &'a str,
}

impl Email for TrialStarted<'_> {
    fn subject(&self) -> String {
        "Welcome to Scanopy! Your Trial Has Started".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Pausable(PausableCategory::TrialAndUsage)
    }

    fn campaign(&self) -> &'static str {
        "trial_started"
    }

    fn body_html(&self) -> String {
        Body::new()
            .content(
                Content::new()
                    .heading(&format!(
                        "Welcome to Scanopy {} {}!",
                        self.plan_name, self.billing_period
                    ))
                    .paragraph("Hi there,")
                    .paragraph(&format!(
                        "Your trial of the {} {} plan has started. You have full access to all features for the next {} days.",
                        self.plan_name, self.billing_period, self.trial_days
                    ))
                    .paragraph(
                        "No credit card is required during the trial. To keep your features and data after it ends, add a payment method anytime from your Settings page.",
                    )
                    .paragraph(BILLING_DETAILS_TAGLINE),
            )
            .cta(links::SETTINGS_BILLING, "Add Payment Method")
            .render()
    }
}
