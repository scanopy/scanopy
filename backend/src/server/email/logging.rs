use std::path::{Path, PathBuf};

use anyhow::Error;
use async_trait::async_trait;
use email_address::EmailAddress;

use super::{
    messages::Email,
    transport::{EmailTransport, SES_MESSAGE_TAGS},
};

/// Maximum length of each slug component in a written filename.
const SLUG_MAX_LEN: usize = 48;

/// Testing transport that never delivers anything.
///
/// Every send is reported at INFO with the recipient, subject and category so
/// email paths (verification, password reset, digests, …) can be exercised
/// locally without SMTP credentials. When an output directory is
/// configured the rendered message is also written there for review — one
/// `.html` (browser-openable) and one `.txt` per send.
///
/// Selected ahead of SMTP whenever `SCANOPY_EMAIL_LOG_DIR` is set, so it
/// deliberately overrides credentials that happen to be present in the
/// environment. Never appropriate for production: the written files contain
/// full bodies, including password-reset and verification tokens.
pub struct LoggingEmailProvider {
    /// `None` once construction settles when no directory is configured, or
    /// when the configured one could not be created — either way the transport
    /// degrades to log-only rather than failing sends.
    output_dir: Option<PathBuf>,
}

impl LoggingEmailProvider {
    pub fn new(output_dir: Option<PathBuf>) -> Self {
        let output_dir = output_dir.and_then(|dir| match std::fs::create_dir_all(&dir) {
            Ok(()) => Some(dir),
            Err(e) => {
                tracing::warn!(
                    dir = %dir.display(),
                    error = %e,
                    "Could not create the email output directory; emails will only be logged"
                );
                None
            }
        });

        Self { output_dir }
    }

    /// Write the rendered message to `dir`, returning the path of the `.html`
    /// file. Errors are the caller's to downgrade — a failed review write must
    /// never break a send.
    fn write_files(
        dir: &Path,
        to: &EmailAddress,
        email: &dyn Email,
        base_url: &str,
        self_hosted: bool,
        unsubscribe_url: Option<&str>,
        ses_tags: Option<&str>,
    ) -> Result<PathBuf, std::io::Error> {
        let subject = email.subject();
        let timestamp = crate::server::shared::clock::now();

        let stem = format!(
            "{}-{}-{}",
            timestamp.format("%Y%m%dT%H%M%S%.6fZ"),
            slugify(to.as_str()),
            slugify(&subject),
        );

        let mut metadata = vec![
            format!("To: {to}"),
            format!("Subject: {subject}"),
            format!("Category: {}", email.category().tag()),
            format!("Date: {}", timestamp.to_rfc3339()),
            format!("Base-URL: {base_url}"),
            format!("Self-Hosted: {self_hosted}"),
        ];
        if let Some(url) = unsubscribe_url {
            metadata.push(format!("List-Unsubscribe: <{url}>"));
        }
        if let Some(tags) = ses_tags {
            metadata.push(format!("{SES_MESSAGE_TAGS}: {tags}"));
        }
        // Attachment bytes are not written out; the header records enough to
        // tell which files a real send would have carried.
        for a in email.attachments() {
            metadata.push(format!(
                "Attachment: {} ({}, {} bytes)",
                a.filename,
                a.content_type,
                a.bytes.len()
            ));
        }
        let metadata = metadata.join("\n");

        // The metadata rides in an HTML comment so the file stays a faithful
        // rendering of what the recipient would have seen.
        let html_path = dir.join(format!("{stem}.html"));
        std::fs::write(
            &html_path,
            format!(
                "<!--\n{metadata}\n-->\n{}",
                email.render_html(base_url, self_hosted)
            ),
        )?;

        std::fs::write(
            dir.join(format!("{stem}.txt")),
            format!("{metadata}\n\n{}", email.render_text(base_url, self_hosted)),
        )?;

        Ok(html_path)
    }
}

#[async_trait]
impl EmailTransport for LoggingEmailProvider {
    async fn send(
        &self,
        to: EmailAddress,
        email: &dyn Email,
        base_url: &str,
        self_hosted: bool,
        unsubscribe_url: Option<&str>,
        ses_tags: Option<&str>,
    ) -> Result<(), Error> {
        // Bodies carry reset/verification tokens, so the log line stays on
        // metadata; the full rendering goes to the review file only.
        let written = match &self.output_dir {
            Some(dir) => {
                match Self::write_files(
                    dir,
                    &to,
                    email,
                    base_url,
                    self_hosted,
                    unsubscribe_url,
                    ses_tags,
                ) {
                    Ok(path) => Some(path),
                    Err(e) => {
                        tracing::warn!(
                            dir = %dir.display(),
                            error = %e,
                            "Could not write the email to the output directory"
                        );
                        None
                    }
                }
            }
            None => None,
        };

        tracing::info!(
            to = %to,
            subject = %email.subject(),
            category = email.category().tag(),
            base_url = base_url,
            self_hosted = self_hosted,
            unsubscribe = unsubscribe_url.is_some(),
            file = written.as_ref().map(|p| p.display().to_string()),
            "Email not delivered: the logging transport is active"
        );

        Ok(())
    }
}

/// Reduce arbitrary text to a filename-safe, readable slug.
fn slugify(input: &str) -> String {
    let mut out = String::with_capacity(input.len().min(SLUG_MAX_LEN));
    let mut last_was_sep = false;

    for c in input.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_was_sep = false;
        } else if !last_was_sep && !out.is_empty() {
            out.push('-');
            last_was_sep = true;
        }
        if out.len() >= SLUG_MAX_LEN {
            break;
        }
    }

    let trimmed = out.trim_end_matches('-');
    if trimmed.is_empty() {
        "email".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::email::messages::EmailCategory;

    struct TestEmail;

    impl Email for TestEmail {
        fn subject(&self) -> String {
            "Reset your password".to_string()
        }

        fn body_html(&self) -> String {
            "<tr><td>Marker: click {base_url}/reset to continue.</td></tr>".to_string()
        }

        fn category(&self) -> EmailCategory {
            EmailCategory::Auth
        }

        fn campaign(&self) -> &'static str {
            "test"
        }
    }

    fn recipient() -> EmailAddress {
        "user@example.test".parse().unwrap()
    }

    fn files_in(dir: &Path) -> Vec<PathBuf> {
        let mut paths: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        paths.sort();
        paths
    }

    #[tokio::test]
    async fn writes_the_rendered_email_to_the_configured_dir() {
        let dir = tempfile::tempdir().unwrap();
        let provider = LoggingEmailProvider::new(Some(dir.path().to_path_buf()));

        provider
            .send(
                recipient(),
                &TestEmail,
                "https://app.example.test",
                false,
                None,
                None,
            )
            .await
            .unwrap();

        let paths = files_in(dir.path());
        assert_eq!(paths.len(), 2, "expected an .html and a .txt: {paths:?}");

        let html_path = paths
            .iter()
            .find(|p| p.extension().unwrap() == "html")
            .unwrap();
        let html = std::fs::read_to_string(html_path).unwrap();
        assert!(html.contains("To: user@example.test"));
        assert!(html.contains("Subject: Reset your password"));
        assert!(html.contains("Category: auth"));
        // The body is the real rendering: the marker, the expanded token, and
        // the shared chrome the transport never assembles itself.
        assert!(html.contains("Marker: click https://app.example.test/reset to continue."));
        assert!(html.contains("<!DOCTYPE html>"));

        let text_path = paths
            .iter()
            .find(|p| p.extension().unwrap() == "txt")
            .unwrap();
        let text = std::fs::read_to_string(text_path).unwrap();
        assert!(text.contains("Subject: Reset your password"));
        assert!(text.contains("Marker: click"));
        assert!(!text.contains("<!DOCTYPE html>"), "should be plaintext");
    }

    #[tokio::test]
    async fn succeeds_without_an_output_dir() {
        let provider = LoggingEmailProvider::new(None);

        provider
            .send(
                recipient(),
                &TestEmail,
                "https://app.example.test",
                true,
                None,
                None,
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn an_unwritable_dir_does_not_break_the_send() {
        // A regular file where a directory is expected: creation fails at
        // construction, and the send still has to succeed.
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("not-a-dir");
        std::fs::write(&blocker, "").unwrap();

        let provider = LoggingEmailProvider::new(Some(blocker.join("emails")));

        provider
            .send(
                recipient(),
                &TestEmail,
                "https://app.example.test",
                false,
                None,
                None,
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn back_to_back_sends_do_not_overwrite_each_other() {
        let dir = tempfile::tempdir().unwrap();
        let provider = LoggingEmailProvider::new(Some(dir.path().to_path_buf()));

        for _ in 0..2 {
            provider
                .send(
                    recipient(),
                    &TestEmail,
                    "https://app.example.test",
                    false,
                    None,
                    None,
                )
                .await
                .unwrap();
        }

        assert_eq!(files_in(dir.path()).len(), 4);
    }

    #[test]
    fn slugs_stay_filename_safe() {
        assert_eq!(slugify("user@example.test"), "user-example-test");
        assert_eq!(slugify("Reset your password!"), "reset-your-password");
        assert_eq!(slugify("  @@  "), "email");
        assert!(slugify(&"a b".repeat(200)).len() <= SLUG_MAX_LEN);
    }
}
