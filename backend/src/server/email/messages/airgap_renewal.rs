use super::{Body, Content, Email, EmailCategory, EmailPreference, links};

/// Sent when a self-hosted licence renews and the organization holds an
/// air-gapped key. That key carries its expiry inside it and the customer's
/// server never contacts Scanopy, so a renewal reaches them only if someone
/// copies the new key by hand. An online key needs no such email: it picks the
/// renewal up at its next check-in.
pub struct AirgapRenewal<'a> {
    pub plan_name: &'a str,
    /// The date the key now installed on their server stops working.
    pub current_key_expires: &'a str,
    /// The date the replacement key runs to.
    pub renewed_through: &'a str,
}

impl Email for AirgapRenewal<'_> {
    fn subject(&self) -> String {
        "Copy your renewed Scanopy license key".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        // Skipping this one takes the customer's server read-only.
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        "airgap_renewal"
    }

    fn body_html(&self) -> String {
        Body::new()
            .content(
                Content::new()
                    .heading("Your license has renewed")
                    .paragraph("Hi there,")
                    .paragraph(&format!(
                        "Your Scanopy {} subscription renewed and is now paid through {}.",
                        self.plan_name, self.renewed_through
                    ))
                    .paragraph(&format!(
                        "Air-gapped keys carry their expiry inside them, so the key on your server still ends on <strong>{}</strong>. Copy the new key from Settings and set it as <strong>SCANOPY_LICENSE_KEY</strong> before then, or the server goes read-only.",
                        self.current_key_expires
                    )),
            )
            .cta(links::SETTINGS_LICENSE, "Copy your new key")
            .render()
    }
}
