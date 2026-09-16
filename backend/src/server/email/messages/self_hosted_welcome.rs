use super::{Body, Content, Email, EmailCategory, EmailPreference};

const INSTALL_DOCS_URL: &str = "https://scanopy.net/docs/self-hosted-server/server-installation/";
const BOOK_DEMO_URL: &str = "https://cal.com/mferrandiz/scanopy-demo";

/// Sent when an organization moves onto a self-hosted plan: says where the
/// license key lives, how to install a server, and (on plans that include
/// deployment assistance) how to book time with the team.
pub struct SelfHostedWelcome<'a> {
    pub plan_name: &'a str,
    /// Trial length when the plan started as a trial; `None` when it was
    /// bought outright or switched to from a cloud plan.
    pub trial_days: Option<u32>,
    /// Whether the plan includes deployment assistance, which adds the demo link.
    pub deployment_assistance: bool,
}

impl Email for SelfHostedWelcome<'_> {
    fn subject(&self) -> String {
        format!("Your Scanopy {} license", self.plan_name)
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        // Carries the license key location and the install guide: without it
        // the customer cannot run the software they just signed up for.
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        "self_hosted_welcome"
    }

    fn body_html(&self) -> String {
        let mut content = Content::new()
            .heading(&format!("Welcome to Scanopy {}", self.plan_name))
            .paragraph("Hi there,")
            .paragraph(
                "Your license is ready. Open Settings and then License in Scanopy Cloud to copy your license key, and set it as <strong>SCANOPY_LICENSE_KEY</strong> on your own server.",
            )
            .paragraph(&format!(
                r#"The <a href="{INSTALL_DOCS_URL}?{{utm}}">self-hosted install guide</a> walks through running the server and pointing a daemon at it."#
            ));

        content = match self.trial_days {
            Some(days) => content.paragraph(&format!(
                "Your {days}-day trial has started, and no card is required during it. Add a payment method before it ends to keep your license key working.",
            )),
            None => content.paragraph(
                "Your subscription is active. Your license key keeps working for as long as the subscription is paid.",
            ),
        };

        if self.deployment_assistance {
            content = content.paragraph(&format!(
                r#"Your plan includes deployment assistance. <a href="{BOOK_DEMO_URL}?{{utm}}">Book time with us</a> and we will help you get it running."#
            ));
        }

        Body::new()
            .content(content)
            .cta(
                "{base_url}/?modal=settings&tab=license&{utm}",
                "Get your license key",
            )
            .render()
    }
}
