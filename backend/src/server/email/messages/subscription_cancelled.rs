use super::{Body, Content, Email, EmailCategory, EmailPreference, links};

/// Sent when a subscription is cancelled and access has ended: account moved to
/// Free, with a resubscribe CTA.
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
                        "Your Scanopy subscription was cancelled and your access ended on {}. Your account has been moved to the Free plan.",
                        self.period_end_date
                    ))
                    .paragraph("You can continue using Scanopy with up to 25 hosts and manual discovery. Resubscribe anytime from your Settings page."),
            )
            .cta(links::PLAN_PICKER, "Resubscribe")
            .render()
    }
}
