use super::{BILLING_DETAILS_TAGLINE, Body, Content, Email, EmailCategory, links};
use crate::server::shared::events::types::TrialExtensionSource;

/// What happens when the extended trial ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtendedTrialEnd {
    /// The subscription starts, paid by the card or invoice on file.
    Converts,
    /// Nothing can pay for the subscription, so the trial ends without it.
    NoPaymentMethod,
    /// A cancellation is scheduled, so the trial ends without a charge.
    CancellationScheduled,
}

/// Sent when a trial's end moves later, by the org's own extension or by
/// Scanopy staff. States the new end date and what happens on it, worded as
/// `TrialEnding` and `SelfHostedTrialEnding` word it. The subscriber sends
/// none for a `PlanSwitch`, which the wording below treats as staff.
pub struct TrialExtended<'a> {
    pub source: TrialExtensionSource,
    pub plan_name: &'a str,
    pub billing_period: &'a str,
    pub days_added: u32,
    /// Formatted new trial end date.
    pub new_trial_end: &'a str,
    pub end: ExtendedTrialEnd,
    /// For a self-hosted plan, the formatted date the key stops working if
    /// the license lapses. `None` for a cloud plan.
    pub key_expires: Option<&'a str>,
}

impl Email for TrialExtended<'_> {
    fn subject(&self) -> String {
        match self.source {
            TrialExtensionSource::User => format!("Your trial now ends on {}", self.new_trial_end),
            TrialExtensionSource::Staff | TrialExtensionSource::PlanSwitch => {
                format!("We've extended your trial to {}", self.new_trial_end)
            }
        }
    }

    fn category(&self) -> EmailCategory {
        // It moves the date a card is charged or a license lapses, and it
        // goes to self-hosted plans too.
        EmailCategory::Billing
    }

    fn campaign(&self) -> &'static str {
        match self.source {
            TrialExtensionSource::User => "trial_extended_user",
            TrialExtensionSource::Staff | TrialExtensionSource::PlanSwitch => {
                "trial_extended_staff"
            }
        }
    }

    fn body_html(&self) -> String {
        let days = if self.days_added == 1 {
            "1 day".to_string()
        } else {
            format!("{} days", self.days_added)
        };
        let (heading, who) = match self.source {
            TrialExtensionSource::User => ("Your trial is extended", "You"),
            TrialExtensionSource::Staff | TrialExtensionSource::PlanSwitch => {
                ("We've extended your trial", "We")
            }
        };

        let content = Content::new()
            .heading(heading)
            .paragraph("Hi there,")
            .paragraph(&format!(
                "{who} added {days} to your {} {} trial. It now ends on {}.",
                self.plan_name, self.billing_period, self.new_trial_end
            ));

        let content = match (self.end, self.key_expires) {
            (ExtendedTrialEnd::Converts, key_expires) => {
                let content = content.paragraph(&format!(
                    "When the trial ends, your {0} {1} subscription starts, billed to the payment method on file.",
                    self.plan_name, self.billing_period
                ));
                if key_expires.is_some() {
                    content.paragraph(
                        "Your license key keeps working through the change. There is nothing to update on your server.",
                    )
                } else {
                    content
                }
            }
            (ExtendedTrialEnd::NoPaymentMethod, None) => content.paragraph(
                "There is no payment method on file. Add one before the trial ends to keep your plan. Without one, your organization moves to Free when the trial ends.",
            ),
            (ExtendedTrialEnd::NoPaymentMethod, Some(key_expires)) => content.paragraph(&format!(
                "There is no payment method on file, so your license lapses when the trial ends. Your key keeps working until {key_expires}, then your server stops accepting it and goes read-only. Add a payment method before then to keep the same key working."
            )),
            (ExtendedTrialEnd::CancellationScheduled, None) => content.paragraph(
                "Your subscription is set to cancel, so your organization moves to Free when the trial ends.",
            ),
            (ExtendedTrialEnd::CancellationScheduled, Some(key_expires)) => {
                content.paragraph(&format!(
                    "Your subscription is set to cancel, so your license lapses when the trial ends. Your key keeps working until {key_expires}, then your server stops accepting it and goes read-only."
                ))
            }
        };

        // As in `TrialEnding` and `SelfHostedTrialEnding`: the license plan's
        // details are on its License tab, not Billing.
        let (content, cta_href, cta_label) = match (self.end, self.key_expires.is_some()) {
            (ExtendedTrialEnd::NoPaymentMethod, true) => {
                (content, links::ADD_PAYMENT_METHOD, "Add payment method")
            }
            (ExtendedTrialEnd::NoPaymentMethod, false) => (
                content.paragraph(BILLING_DETAILS_TAGLINE),
                links::ADD_PAYMENT_METHOD,
                "Add Payment Method",
            ),
            (_, true) => (content, links::SETTINGS_LICENSE, "View your license"),
            (_, false) => (
                content.paragraph(BILLING_DETAILS_TAGLINE),
                links::SETTINGS_BILLING,
                "View Billing",
            ),
        };

        Body::new()
            .content(content)
            .cta(cta_href, cta_label)
            .render()
    }
}
