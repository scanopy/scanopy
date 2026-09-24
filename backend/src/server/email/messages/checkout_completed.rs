use super::{Body, Content, Email, EmailCategory, EmailPreference, links};

/// Sent when a checkout completes: welcomes the user to their newly active paid
/// plan.
pub struct CheckoutCompleted<'a> {
    pub plan_name: &'a str,
}

impl Email for CheckoutCompleted<'_> {
    fn subject(&self) -> String {
        format!("Welcome to Scanopy {}", self.plan_name)
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        "checkout_completed"
    }

    fn body_html(&self) -> String {
        Body::new()
            .content(
                Content::new()
                    .heading(&format!("You're On Scanopy {}", self.plan_name))
                    .paragraph("Hi there,")
                    .paragraph(&format!(
                        "Your subscription to <strong>{}</strong> is active. Thanks for picking Scanopy — we're glad to have you.",
                        self.plan_name
                    )),
            )
            .cta(links::APP_HOME, "Open Scanopy")
            .render()
    }
}
