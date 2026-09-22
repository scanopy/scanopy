use super::{Body, Content, Email, EmailCategory, EmailPreference, PausableCategory, links};

/// Sent when an organization crosses 80% of a plan limit (hosts/networks/seats).
pub struct PlanLimitApproaching<'a> {
    pub first_name: Option<&'a str>,
    pub limit_type: &'a str,
    pub current_count: u64,
    pub limit: u64,
    pub plan_name: &'a str,
    pub has_overage: bool,
}

impl Email for PlanLimitApproaching<'_> {
    fn subject(&self) -> String {
        format!("You're Approaching Your {} Limit", self.limit_type)
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Pausable(PausableCategory::TrialAndUsage)
    }

    fn campaign(&self) -> &'static str {
        "plan_limit_approaching"
    }

    fn body_html(&self) -> String {
        let (limit_message, cta_href, cta_label) = if self.has_overage {
            (
                format!(
                    "Additional {} beyond your included amount will be billed automatically.",
                    self.limit_type
                ),
                links::SETTINGS_BILLING,
                "View Billing",
            )
        } else {
            (
                "Upgrade your plan to increase your limits and keep growing.".to_string(),
                links::PLAN_PICKER,
                "Upgrade Plan",
            )
        };
        Body::new()
            .content(
                Content::new()
                    .heading("Approaching Plan Limit")
                    .paragraph(&format!("Hi {},", self.first_name.unwrap_or("there")))
                    .paragraph(&format!(
                        "You're using <strong>{}</strong> of your <strong>{}</strong> included {} on the {} plan.",
                        self.current_count, self.limit, self.limit_type, self.plan_name
                    ))
                    .paragraph(&limit_message),
            )
            .cta(cta_href, cta_label)
            .render()
    }
}
