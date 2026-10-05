use anyhow::{Error, anyhow};
use async_trait::async_trait;
use base64ct::{Base64, Encoding};
use email_address::EmailAddress;
use reqwest::Client;
use serde_json::json;

use super::{messages::Email, transport::EmailTransport};

/// Brevo-based email transport (transactional HTTP API).
pub struct BrevoEmailProvider {
    api_key: String,
    client: Client,
}

impl BrevoEmailProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            // `Client::new()` has no timeout at all, so a hung connection to Brevo held whatever
            // was sending: the digest email is sent from a discovery's completion. Email is
            // best-effort, and a send that has not finished in 30s is better abandoned.
            client: Client::builder()
                .connect_timeout(std::time::Duration::from_secs(10))
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create Brevo HTTP client"),
        }
    }
}

/// Body of a transactional send. Tagged with both the category and the campaign so Brevo's
/// delivery, open and click stats separate emails that share a category (trial_started vs
/// trial_ending_no_payment are both `billing`).
fn send_payload(
    to: &EmailAddress,
    email: &dyn Email,
    base_url: &str,
    self_hosted: bool,
) -> serde_json::Value {
    let mut payload = json!({
        "sender": {
            "name": "Scanopy",
            "email": "no-reply@email.scanopy.net"
        },
        "to": [{ "email": to.to_string() }],
        "subject": email.subject(),
        "htmlContent": email.render_html(base_url, self_hosted),
        "tags": [email.category().as_str(), email.campaign()],
    });

    // Brevo takes attachments as base64 `content` + `name` entries.
    let attachments = email.attachments();
    if !attachments.is_empty() {
        payload["attachment"] = json!(
            attachments
                .iter()
                .map(|a| json!({
                    "content": Base64::encode_string(&a.bytes),
                    "name": a.filename,
                }))
                .collect::<Vec<_>>()
        );
    }

    payload
}

#[async_trait]
impl EmailTransport for BrevoEmailProvider {
    async fn send(
        &self,
        to: EmailAddress,
        email: &dyn Email,
        base_url: &str,
        self_hosted: bool,
    ) -> Result<(), Error> {
        let url = "https://api.brevo.com/v3/smtp/email";
        let payload = send_payload(&to, email, base_url, self_hosted);

        let response = self
            .client
            .post(url)
            .header("api-key", &self.api_key)
            .json(&payload)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!(
                "Failed to send email via Brevo: {}",
                response.text().await?
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::email::messages::{TrialEnding, TrialStarted};

    #[test]
    fn emails_sharing_a_category_get_distinct_tags() {
        let to: EmailAddress = "owner@example.com".parse().unwrap();
        let started = TrialStarted {
            plan_name: "Pro",
            trial_days: 14,
            billing_period: "month",
        };
        let ending = TrialEnding {
            has_payment: false,
            plan_name: "Pro",
            billing_period: "month",
            hosts_count: 0,
            sites_count: 0,
            daemons_count: 0,
            services_count: 0,
            days_into_trial: 11,
        };
        assert_eq!(started.category(), ending.category());

        let tags =
            |email: &dyn Email| send_payload(&to, email, "https://app", false)["tags"].clone();
        assert_ne!(tags(&started), tags(&ending));
    }
}
