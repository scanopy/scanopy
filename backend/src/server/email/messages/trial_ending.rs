use super::{
    BILLING_DETAILS_TAGLINE, Body, Content, Email, EmailCategory, EmailPreference,
    PausableCategory, links,
};

/// Sent 3 days before a trial ends: recaps trial value and prompts the user to
/// add a payment method (or confirms upcoming billing if one is on file).
pub struct TrialEnding<'a> {
    pub has_payment: bool,
    pub plan_name: &'a str,
    pub billing_period: &'a str,
    pub hosts_count: u64,
    pub networks_count: u64,
    pub daemons_count: u64,
    pub services_count: u64,
    pub days_into_trial: i64,
}

impl Email for TrialEnding<'_> {
    fn subject(&self) -> String {
        "Your Trial Ends in 3 Days".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Pausable(PausableCategory::TrialAndUsage)
    }

    fn campaign(&self) -> &'static str {
        if self.has_payment {
            "trial_ending_has_payment"
        } else {
            "trial_ending_no_payment"
        }
    }

    fn body_html(&self) -> String {
        let main = if self.has_payment {
            Content::new()
                .heading("Your Trial Ends Soon")
                .paragraph("Hi there,")
                .paragraph(&format!(
                    "Your {0} {1} trial ends in 3 days. At the end of the trial period, your {0} {1} subscription begins automatically.",
                    self.plan_name, self.billing_period
                ))
                .paragraph(BILLING_DETAILS_TAGLINE)
        } else {
            Content::new()
                .heading("Your Trial Ends Soon")
                .paragraph("Hi there,")
                .paragraph(&format!(
                    "Your {} {} trial ends in 3 days. To keep all your features and data, add a payment method before the trial expires.",
                    self.plan_name, self.billing_period
                ))
                .paragraph("If no payment method is added, your account will be downgraded to the Free plan, which includes up to 25 hosts with manual discovery only.")
                .paragraph(BILLING_DETAILS_TAGLINE)
        };

        let recap_table = format!(
            r#"                            <table cellpadding="0" cellspacing="0" border="0" width="100%" style="border-collapse: collapse;">
                                <tr>
                                    <td style="padding: 8px 0; font-size: 14px; color: #4a4a4a;"><strong>{hosts}</strong> hosts discovered</td>
                                    <td style="padding: 8px 0; font-size: 14px; color: #4a4a4a;"><strong>{networks}</strong> networks mapped</td>
                                </tr>
                                <tr>
                                    <td style="padding: 8px 0; font-size: 14px; color: #4a4a4a;"><strong>{daemons}</strong> daemons connected</td>
                                    <td style="padding: 8px 0; font-size: 14px; color: #4a4a4a;"><strong>{services}</strong> services identified</td>
                                </tr>
                                <tr>
                                    <td colspan="2" style="padding: 8px 0; font-size: 14px; color: #4a4a4a;"><strong>{days}</strong> days into your trial</td>
                                </tr>
                            </table>
"#,
            hosts = self.hosts_count,
            networks = self.networks_count,
            daemons = self.daemons_count,
            services = self.services_count,
            days = self.days_into_trial,
        );

        let cta_label = if self.has_payment {
            "View Billing"
        } else {
            "Add Payment Method"
        };

        Body::new()
            .content(main)
            .content_named(
                "Trial Recap",
                Content::new()
                    .subheading("Here's what Scanopy found during your trial")
                    .raw(&recap_table),
            )
            .cta(links::SETTINGS_BILLING, cta_label)
            .render()
    }
}
