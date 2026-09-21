use super::{Body, Content, Email, EmailCategory, EmailPreference, links};

/// Sent instead of the cloud trial-ending email when the trial is for a
/// self-hosted plan. The cloud email recaps hosts, networks, daemons and
/// services found during the trial, and those all live on the customer's own
/// server, so the cloud org's counts are zero. What ends with this trial is
/// the license key, so the copy is about the key.
pub struct SelfHostedTrialEnding<'a> {
    pub plan_name: &'a str,
    pub billing_period: &'a str,
    pub has_payment: bool,
}

impl Email for SelfHostedTrialEnding<'_> {
    fn subject(&self) -> String {
        "Your Scanopy license trial ends in 3 days".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        // Without a payment method the key stops working when the trial ends,
        // which takes the customer's server read-only.
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        if self.has_payment {
            "self_hosted_trial_ending_has_payment"
        } else {
            "self_hosted_trial_ending_no_payment"
        }
    }

    fn body_html(&self) -> String {
        let content = Content::new()
            .heading("Your license trial ends soon")
            .paragraph("Hi there,");

        let (content, cta_label) = if self.has_payment {
            (
                content
                    .paragraph(&format!(
                        "Your {0} {1} trial ends in 3 days. Your {0} {1} subscription starts then, billed to the payment method on file.",
                        self.plan_name, self.billing_period
                    ))
                    .paragraph(
                        "Your license key keeps working through the change. There is nothing to update on your server.",
                    ),
                "View your license",
            )
        } else {
            (
                content
                    .paragraph(&format!(
                        "Your {} {} trial ends in 3 days, and there is no payment method on file.",
                        self.plan_name, self.billing_period
                    ))
                    .paragraph(
                        "When the trial ends your license key stops working: your server stops accepting it and goes read-only. Add a payment method in Settings before then to keep the same key working.",
                    ),
                "Add payment method",
            )
        };

        Body::new()
            .content(content)
            .cta(links::SETTINGS_LICENSE, cta_label)
            .render()
    }
}
