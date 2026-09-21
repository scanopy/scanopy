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
        let mut payload = json!({
            "sender": {
                "name": "Scanopy",
                "email": "no-reply@email.scanopy.net"
            },
            "to": [{ "email": to.to_string() }],
            "subject": email.subject(),
            "htmlContent": email.render_html(base_url, self_hosted),
            "tags": [email.category().as_str()],
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
