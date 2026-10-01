use lettre::{
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
    message::{Attachment, Mailbox, MultiPart, SinglePart, header::ContentType},
    transport::smtp::{authentication::Credentials, client::Tls},
};

use anyhow::{Error, anyhow};
use async_trait::async_trait;
use email_address::EmailAddress;

use super::{messages::Email, transport::EmailTransport};
use crate::server::shared::trusted_ca::{TrustedCaBundle, is_untrusted_certificate};

/// Extra guidance for a failure where we never got a reply out of the server.
///
/// Port 465 means implicit TLS, and it is what we fall back to when
/// `SCANOPY_SMTP_PORT` is unset. Providers that accept submission only on 587 with
/// STARTTLS — Microsoft 365 among them — do not listen on 465 at all, so the connection
/// dies before authentication is ever attempted and the operator sees a failure that
/// looks nothing like a port mismatch. Say so, but only when the shoe fits: on 587 the
/// hint would send them the wrong way.
fn implicit_tls_hint(port: Option<u16>, got_reply: bool) -> &'static str {
    if got_reply || !matches!(port, None | Some(465)) {
        return "";
    }
    " The configured port is 465, which means implicit TLS (this is the default when \
     SCANOPY_SMTP_PORT is unset). Servers that accept submission only on 587 with STARTTLS will \
     not answer there — if yours is one of them, set SCANOPY_SMTP_PORT=587."
}

/// SMTP-based email transport (lettre), used as the fallback when Brevo is
/// not configured.
pub struct SmtpEmailProvider {
    mailer: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
    /// Kept for diagnostics only: a send failure is useless to an operator without
    /// knowing which server and port it was talking to.
    relay: String,
    port: Option<u16>,
}

impl SmtpEmailProvider {
    pub fn new(
        smtp_username: String,
        smtp_password: String,
        smtp_email: String,
        smtp_relay: String,
        smtp_port: Option<u16>,
        trusted_ca: Option<&TrustedCaBundle>,
    ) -> Result<Self, Error> {
        let creds = Credentials::new(smtp_username, smtp_password);

        // Port 465 (or unset) uses implicit TLS (SMTPS) via `relay`, preserving
        // the historical default. Any other port uses STARTTLS, which is what
        // submission ports like 587 and 25 expect.
        let mut builder = match smtp_port {
            None | Some(465) => AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp_relay)
                .map_err(|e| anyhow!("Failed to create SMTP transport: {}", e))?,
            Some(port) => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&smtp_relay)
                .map_err(|e| anyhow!("Failed to create SMTP transport: {}", e))?
                .port(port),
        };

        // Same TLS mode as above, with the operator's CA roots added to the default ones.
        if let Some(bundle) = trusted_ca {
            let params = bundle.smtp_tls_parameters(smtp_relay.clone())?;
            builder = builder.tls(match smtp_port {
                None | Some(465) => Tls::Wrapper(params),
                Some(_) => Tls::Required(params),
            });
        }

        let mailer = builder.credentials(creds).build();

        let from = Mailbox::new(
            Some("Scanopy".to_string()),
            smtp_email
                .parse()
                .map_err(|e| anyhow!("Invalid from email address: {}", e))?,
        );

        Ok(Self {
            mailer,
            from,
            relay: smtp_relay,
            port: smtp_port,
        })
    }
}

#[async_trait]
impl EmailTransport for SmtpEmailProvider {
    async fn send(
        &self,
        to: EmailAddress,
        email: &dyn Email,
        base_url: &str,
        self_hosted: bool,
    ) -> Result<(), Error> {
        let to_mbox = Mailbox::new(
            None,
            to.email()
                .parse()
                .map_err(|e| anyhow!("Invalid recipient email address: {}", e))?,
        );

        let html = email.render_html(base_url, self_hosted);
        let text = email.render_text(base_url, self_hosted);

        let body_alternative = MultiPart::alternative()
            .singlepart(SinglePart::plain(text))
            .singlepart(SinglePart::html(html));

        // With no attachments, send the plain/HTML alternative directly. With
        // attachments, wrap it in a `mixed` part and append each file.
        let attachments = email.attachments();
        let body = if attachments.is_empty() {
            body_alternative
        } else {
            let mut mixed = MultiPart::mixed().multipart(body_alternative);
            for a in attachments {
                let content_type = ContentType::parse(&a.content_type)
                    .map_err(|e| anyhow!("Invalid attachment content type: {}", e))?;
                mixed = mixed.singlepart(Attachment::new(a.filename).body(a.bytes, content_type));
            }
            mixed
        };

        let message = lettre::Message::builder()
            .from(self.from.clone())
            .to(to_mbox)
            .subject(email.subject())
            .multipart(body)?;

        // Log here rather than at the ~8 call sites: they each know *which* email failed
        // but have no idea why, and this is the only point that still holds the reply
        // code and the server's own words. lettre's `Display` carries the enhanced status
        // and its explanation ("535 5.7.139 … basic authentication is disabled"), which is
        // what separates a rejected password from a tenant policy — so it goes out whole
        // rather than being summarised. Deliberately omits the credentials and the body.
        //
        // The error is passed on as itself, not stringified, so `ApiError` can recognise
        // it upstream instead of leaking this text to an end user.
        self.mailer.send(message).await.map_err(|e| {
            // lettre reports a rejected certificate as a connection error, not a TLS one. A
            // rejected certificate also means the relay answered, so the port is not the
            // problem and the implicit-TLS hint would send the operator the wrong way.
            let cert_untrusted = is_untrusted_certificate(&e);
            tracing::error!(
                relay = %self.relay,
                port = self.port.unwrap_or(465),
                smtp_code = e.status().map(u16::from),
                permanent = e.is_permanent(),
                transient = e.is_transient(),
                tls = e.is_tls() || cert_untrusted,
                timeout = e.is_timeout(),
                "Failed to send email: {e}.{}{}",
                implicit_tls_hint(self.port, e.status().is_some() || cert_untrusted),
                if cert_untrusted {
                    " The relay's certificate is not trusted. If it is signed by a private CA, \
                     set SCANOPY_TRUSTED_CA_BUNDLE to that CA's PEM file."
                } else {
                    ""
                }
            );
            Error::new(e)
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::email::messages::{EmailCategory, EmailPreference};
    use crate::server::shared::types::api::ApiError;
    use crate::server::shared::types::error_codes::ErrorCode;

    struct TestEmail;

    impl Email for TestEmail {
        fn subject(&self) -> String {
            "Reset your password".to_string()
        }

        fn body_html(&self) -> String {
            "<tr><td>Click {base_url}/reset to continue.</td></tr>".to_string()
        }

        fn category(&self) -> EmailCategory {
            EmailCategory::Auth
        }

        fn preference(&self) -> EmailPreference {
            EmailPreference::Required
        }

        fn campaign(&self) -> &'static str {
            "test"
        }
    }

    /// A port with nothing behind it, so a send fails at connect without any network
    /// access or a real mail server.
    fn closed_port() -> u16 {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        drop(listener);
        port
    }

    /// A send failure must reach the API layer as a *typed* lettre error, not as text.
    /// When this was stringified, the mail server's reply — which carries the relay
    /// hostname and the provider's per-request diagnostics — was returned in the response
    /// body and shown to any member in a toast.
    #[tokio::test]
    async fn a_send_failure_reaches_the_client_coded_and_without_the_servers_reply() {
        let relay = "mail.internal.example";
        let provider = SmtpEmailProvider::new(
            "scanopy@example.test".to_string(),
            "hunter2".to_string(),
            "scanopy@example.test".to_string(),
            relay.to_string(),
            Some(closed_port()),
            None,
        )
        .expect("provider should build");

        let err = provider
            .send(
                "user@example.test".parse().unwrap(),
                &TestEmail,
                "https://app.example.test",
                true,
            )
            .await
            .expect_err("sending to a closed port must fail");

        // The typed error survives, which is what lets the API layer recognise it.
        assert!(
            err.downcast_ref::<lettre::transport::smtp::Error>()
                .is_some(),
            "the lettre error was flattened into text: {err}"
        );

        let api_error = ApiError::from(err);
        assert!(matches!(
            api_error.error_code,
            Some(ErrorCode::EmailDeliveryFailed)
        ));
        // Nothing about our mail infrastructure goes to the client.
        assert!(
            !api_error.message.contains(relay),
            "the relay host leaked to the client: {}",
            api_error.message
        );
    }

    #[test]
    fn the_implicit_tls_default_is_only_blamed_when_it_could_be_the_cause() {
        // Never reached the server, and on the implicit-TLS default: this is the trap
        // Microsoft 365 self-hosters fall into, so point at 587.
        assert!(implicit_tls_hint(None, false).contains("SCANOPY_SMTP_PORT=587"));
        assert!(implicit_tls_hint(Some(465), false).contains("SCANOPY_SMTP_PORT=587"));

        // Already on a STARTTLS port — suggesting 587 would send them the wrong way.
        assert!(implicit_tls_hint(Some(587), false).is_empty());
        assert!(implicit_tls_hint(Some(25), false).is_empty());

        // The server answered, so the port is fine and the problem is what it said.
        assert!(implicit_tls_hint(None, true).is_empty());
        assert!(implicit_tls_hint(Some(465), true).is_empty());
    }

    /// lettre wraps a rejected relay certificate as a connection error. The send-failure log
    /// relies on `is_untrusted_certificate` still finding it through that wrapping.
    #[tokio::test]
    async fn an_untrusted_relay_certificate_is_recognised_through_lettre() {
        use crate::server::shared::trusted_ca::test_support::serve_test_tls;

        let port = serve_test_tls().await;
        let mailer: AsyncSmtpTransport<Tokio1Executor> =
            AsyncSmtpTransport::<Tokio1Executor>::relay("localhost")
                .unwrap()
                .port(port)
                .build();
        let err = mailer.test_connection().await.unwrap_err();
        assert!(is_untrusted_certificate(&err), "{err}");
    }
}
