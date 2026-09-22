use super::{Body, Content, Email, EmailCategory, EmailPreference, links};

/// Sent before a self-hosted licence renews, to an organization holding an
/// air-gapped key. Their server never contacts Scanopy, so without this the
/// first they hear of the renewal is the charge. It is also the only moment
/// their plan is about to become changeable: leaving an air-gapped key is
/// refused while the current key still has time to run.
pub struct AirgapExpiring<'a> {
    pub plan_name: &'a str,
    /// The date the key installed on their server stops working.
    pub key_expires: &'a str,
    /// The date the subscription renews and bills.
    pub renews_at: &'a str,
    /// Formatted renewal amount, e.g. "$6,000.00".
    pub amount: &'a str,
    /// Whether a lower self-hosted tier exists to move down to.
    pub can_move_down: bool,
}

impl Email for AirgapExpiring<'_> {
    fn subject(&self) -> String {
        format!("Your Scanopy license key expires {}", self.key_expires)
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        // Missing this one means an unexpected charge, or a server that goes
        // read-only with no warning.
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        "airgap_expiring"
    }

    fn body_html(&self) -> String {
        let mut content = Content::new()
            .heading("Your license key expires soon")
            .paragraph("Hi there,")
            .paragraph(&format!(
                "The air-gapped key on your server stops working on <strong>{}</strong>. Your {} subscription renews on {} for {}, and a renewed key is ready to copy from Settings once it does.",
                self.key_expires, self.plan_name, self.renews_at, self.amount
            ))
            .paragraph(&format!(
                "From {} you can also switch to an online key, which picks up renewals by itself, so there is nothing to copy by hand again.",
                self.renews_at
            ));

        if self.can_move_down {
            content = content.paragraph(&format!(
                "Want a smaller plan, or one without air-gapped keys? That change is available from {} too, once the current key's period ends. To stop the renewal altogether, cancel before {}.",
                self.renews_at, self.renews_at
            ));
        } else {
            content = content.paragraph(&format!(
                "To stop the renewal, cancel before {}.",
                self.renews_at
            ));
        }

        Body::new()
            .content(content)
            .cta(links::SETTINGS_LICENSE, "Open your license settings")
            .render()
    }
}
