//! Transactional email messages.
//!
//! Each email lives in its own file as a plain data struct holding that
//! message's template variables, plus an `impl Email`. Adding a new email is:
//! create a file here, define the struct, `impl Email` — no central registry
//! to edit. The producers ([`super::brevo`] / [`super::smtp`]) dispatch on
//! `&dyn Email` and never need to know which concrete email they're sending.

mod cancellation_initiated;
mod checkout_completed;
mod compose;
mod daemon_standby;
mod daemon_sunset;
mod daemon_unreachable;
mod discovery_digest;
mod discovery_guide;
mod email_changed_old;
mod install_command;
mod invite;
mod oidc_linked;
mod oidc_unlinked;
mod organization_deleted;
mod password_changed;
mod password_reset;
mod payment_action_required;
mod payment_failed;
mod payment_method_added;
mod payment_method_removed;
mod payment_recovered;
mod plan_changed;
mod plan_limit_approaching;
mod plan_limit_reached;
mod self_hosted_welcome;
mod subscription_cancelled;
mod subscription_paused;
mod subscription_reactivated;
mod subscription_resumed;
mod trial_converted;
mod trial_ending;
mod trial_expired;
mod trial_started;
mod usage_summary;
mod verification;

pub use cancellation_initiated::CancellationInitiated;
pub use checkout_completed::CheckoutCompleted;
pub use compose::{BILLING_DETAILS_TAGLINE, Body, Content};
pub use daemon_standby::DaemonStandby;
pub use daemon_sunset::DaemonSunset;
pub use daemon_unreachable::DaemonUnreachable;
pub use discovery_digest::DiscoveryDigest;
pub use discovery_guide::DiscoveryGuide;
pub use email_changed_old::EmailChangedOld;
pub use install_command::InstallCommand;
pub use invite::Invite;
pub use oidc_linked::OidcLinked;
pub use oidc_unlinked::OidcUnlinked;
pub use organization_deleted::OrganizationDeleted;
pub use password_changed::PasswordChanged;
pub use password_reset::PasswordReset;
pub use payment_action_required::PaymentActionRequired;
pub use payment_failed::PaymentFailed;
pub use payment_method_added::PaymentMethodAdded;
pub use payment_method_removed::PaymentMethodRemoved;
pub use payment_recovered::PaymentRecovered;
pub use plan_changed::PlanChanged;
pub use plan_limit_approaching::PlanLimitApproaching;
pub use plan_limit_reached::PlanLimitReached;
pub use self_hosted_welcome::SelfHostedWelcome;
pub use subscription_cancelled::SubscriptionCancelled;
pub use subscription_paused::SubscriptionPaused;
pub use subscription_reactivated::SubscriptionReactivated;
pub use subscription_resumed::SubscriptionResumed;
pub use trial_converted::TrialConverted;
pub use trial_ending::TrialEnding;
pub use trial_expired::TrialExpired;
pub use trial_started::TrialStarted;
pub use usage_summary::UsageSummary;
pub use verification::Verification;

/// Broad classification for a transactional email, aligned to the event
/// sources that produce them. Surfaced to the provider as a message tag
/// (Brevo `tags`); never user-visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmailCategory {
    /// Sign-in, verification, and account-security notices.
    Auth,
    /// Subscription, payment, trial, and plan-limit lifecycle.
    Billing,
    /// Post-signup guidance (invites, first-discovery walkthrough).
    Onboarding,
    /// Daemon connectivity / lifecycle notices.
    Daemon,
    /// Account-level lifecycle (e.g. organization deletion).
    Account,
    /// Per-discovery-session digest.
    Digest,
}

impl EmailCategory {
    /// Stable lowercase tag string used by the provider.
    pub fn as_str(&self) -> &'static str {
        match self {
            EmailCategory::Auth => "auth",
            EmailCategory::Billing => "billing",
            EmailCategory::Onboarding => "onboarding",
            EmailCategory::Daemon => "daemon",
            EmailCategory::Account => "account",
            EmailCategory::Digest => "digest",
        }
    }
}

/// Whether the recipient can suppress an email, and if so which preference
/// toggle governs it.
///
/// Distinct from [`EmailCategory`] (the provider analytics tag): pausability
/// cuts *across* categories — Billing, for example, splits into required
/// receipts and pausable nudges — so it cannot be derived from the category.
/// Every email declares its own [`EmailPreference`] via [`Email::preference`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmailPreference {
    /// Always sent — security, billing receipts, and account-lifecycle
    /// messages with legal or security weight. Ignores user preferences;
    /// surfaced in Settings as a single disabled "Required emails" row.
    Required,
    /// Suppressible by the recipient; gated at send time by the matching
    /// [`PausableCategory`] flag in their preferences.
    Pausable(PausableCategory),
}

/// The user-pausable email groups. Each maps 1:1 to a boolean flag on
/// `EmailSettings`, and each is one toggle in Settings → Email.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PausableCategory {
    /// Per-discovery-session digests.
    DiscoveryDigest,
    /// Product onboarding / first-run walkthrough nudges.
    ProductOnboarding,
    /// Daemon connectivity notices (standby, unreachable).
    DaemonAlerts,
    /// Trial reminders and plan-limit nudges.
    TrialAndUsage,
}

/// A binary file delivered alongside an email body (e.g. a Stripe invoice
/// PDF). Carried by value so the bytes are fetched before the email is
/// constructed; the transports base64/MIME-encode them at send time.
pub struct EmailAttachment {
    pub filename: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
}

/// A single transactional email.
///
/// Implementors are plain data structs carrying the template variables for one
/// message. The contract is compile-enforced: [`subject`](Email::subject) and
/// [`body_html`](Email::body_html) return owned `String` (never `Option`), so
/// it is impossible to construct an email missing its subject or body, and
/// there is no string-keyed lookup that could silently miss — each email is a
/// distinct type selected at the call site.
///
/// [`render_html`](Email::render_html) / [`render_text`](Email::render_text)
/// wrap the body in the shared chrome and expand the three layout tokens
/// (`{current_year}`, `{base_url}`, `{utm}`); individual emails never override
/// them.
pub trait Email: Send + Sync {
    /// The fully-rendered subject line. Implementors substitute any
    /// subject-level variables themselves.
    fn subject(&self) -> String;

    /// The inner content HTML — the message's `<tr>` rows. The shared header,
    /// logo, and footer are added by [`render_html`](Email::render_html); do
    /// not include them here. May contain the `{base_url}` and `{utm}` tokens.
    fn body_html(&self) -> String;

    /// Classification tag for grouping and provider analytics.
    fn category(&self) -> EmailCategory;

    /// Whether the recipient can suppress this email, and which preference
    /// governs it. Required of every email so the pausable/required split is
    /// exhaustive and compiler-enforced — a new message cannot be added
    /// without classifying it.
    fn preference(&self) -> EmailPreference;

    /// The `utm_campaign` slug for this email's CTAs.
    fn campaign(&self) -> &'static str;

    /// The `utm_medium` value for this email's CTAs. Defaults to the email's
    /// category name (`auth`, `billing`, `onboarding`, `daemon`, `account`,
    /// `digest`) so analytics gets a useful split out of the box. Override
    /// only when a particular email warrants a different bucket.
    fn utm_medium(&self) -> &'static str {
        self.category().as_str()
    }

    /// Bare UTM query-string fragment — no leading `?` or `&`. Single
    /// source of truth for the UTM format; consumed by [`with_utm`] for
    /// dynamic URL construction and by the `{utm}` token substitution in
    /// [`render_html`].
    fn utm_qs(&self) -> String {
        format!(
            "utm_source=email&utm_campaign={}&utm_medium={}",
            self.campaign(),
            self.utm_medium(),
        )
    }

    /// Append the standard UTM tracking query to a URL. Picks `?` vs `&`
    /// based on whether the URL already has a query string.
    fn with_utm(&self, url: &str) -> String {
        let sep = if url.contains('?') { '&' } else { '?' };
        format!("{url}{sep}{}", self.utm_qs())
    }

    /// Wrap the body in the shared chrome and substitute the layout tokens.
    ///
    /// `self_hosted` gates the footer's sender-identification block: cloud
    /// sends originate from Scanopy LLC and disclose the LLC + postal address,
    /// while self-hosted sends are relayed by the operator's own mail server,
    /// so that disclosure is replaced by a plain copyright line.
    fn render_html(&self, base_url: &str, self_hosted: bool) -> String {
        let year = chrono::Utc::now().format("%Y").to_string();
        let footer_legal = if self_hosted {
            FOOTER_LEGAL_SELF_HOSTED
        } else {
            FOOTER_LEGAL_CLOUD
        };
        format!("{}{}{}", EMAIL_HEADER, self.body_html(), EMAIL_FOOTER)
            .replace("{footer_legal}", footer_legal)
            .replace("{current_year}", &year)
            .replace("{base_url}", base_url)
            .replace("{utm}", &self.utm_qs())
    }

    /// Plaintext alternative, derived from the wrapped HTML.
    fn render_text(&self, base_url: &str, self_hosted: bool) -> String {
        strip_html_tags(self.render_html(base_url, self_hosted))
    }

    /// Binary files to deliver with the message. Default-empty so the vast
    /// majority of emails opt out by doing nothing; transports MIME/base64
    /// encode whatever is returned. Bytes must be fetched before the email is
    /// constructed (this is sync), so an attaching email holds them by value.
    fn attachments(&self) -> Vec<EmailAttachment> {
        Vec::new()
    }
}

/// Strip HTML tags for the plain-text multipart alternative.
pub fn strip_html_tags(html: String) -> String {
    html2text::from_read(html.as_bytes(), 80).unwrap_or_else(|_| html.to_string())
}

pub const EMAIL_HEADER: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Scanopy</title>
</head>
<body style="margin: 0; padding: 0; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif; background-color: #f5f5f5;">
    <table role="presentation" style="width: 100%; border-collapse: collapse; background-color: #f5f5f5;">
        <tr>
            <td align="center" style="padding: 40px 20px;">
                <table role="presentation" style="max-width: 600px; width: 100%; border-collapse: collapse; background-color: #ffffff; border-radius: 8px; box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);">
                    <!-- Header with Logo -->
                    <tr>
                        <td align="center" style="padding: 40px 40px 30px 40px;">
                            <img src="https://cdn.jsdelivr.net/gh/scanopy/scanopy@main/media/logo.png" alt="Scanopy" style="width: 80px; height: 80px; display: block;">
                        </td>
                    </tr>
"#;

pub const EMAIL_FOOTER: &str = r#"                    <!-- Footer -->
                    <tr>
                        <td align="center" style="padding: 30px 40px 20px 40px; background-color: #f9fafb; border-radius: 0 0 8px 8px;">
                            <!-- Social Links -->
                            <table role="presentation" style="margin: 0 auto 20px auto; border-collapse: collapse;">
                                <tr>
                                    <td style="padding: 0 10px;">
                                        <a href="https://discord.com/invite/b7ffQr8AcZ" style="display: inline-block;">
                                            <img src="https://cdn.jsdelivr.net/gh/selfhst/icons@master/png/discord.png" alt="Discord" style="width: 24px; height: 24px; display: block;">
                                        </a>
                                    </td>
                                    <td style="padding: 0 10px;">
                                        <a href="https://github.com/scanopy/scanopy" style="display: inline-block;">
                                            <img src="https://cdn.jsdelivr.net/gh/selfhst/icons@master/png/github.png" alt="GitHub" style="width: 24px; height: 24px; display: block;">
                                        </a>
                                    </td>
                                </tr>
                            </table>

                            <p style="margin: 0 0 12px 0; font-size: 12px; line-height: 18px; color: #9ca3af;"><a href="{base_url}/?modal=settings&tab=email&{utm}" style="color: #6b7280; text-decoration: underline;">Manage email preferences</a></p>
{footer_legal}
                        </td>
                    </tr>
                </table>
            </td>
        </tr>
    </table>
</body>
</html>
"#;

/// Footer sender-identification block for cloud sends: Scanopy LLC is the
/// commercial sender, so it discloses the entity and its postal address
/// (standard commercial-email sender identification).
pub const FOOTER_LEGAL_CLOUD: &str = r#"                            <p style="margin: 0; font-size: 12px; line-height: 18px; color: #9ca3af;">© {current_year} Scanopy LLC. All rights reserved.</p>
                            <p style="margin: 8px 0 0 0; font-size: 12px; line-height: 18px; color: #9ca3af;">Scanopy LLC &middot; 418 Broadway Ste N, Albany, NY 12207</p>"#;

/// Footer block for self-hosted sends: the operator's own mail server relays
/// these and there is no commercial relationship with Scanopy LLC to disclose,
/// so the LLC + address line is dropped for a plain branding line.
pub const FOOTER_LEGAL_SELF_HOSTED: &str = r#"                            <p style="margin: 0; font-size: 12px; line-height: 18px; color: #9ca3af;">© {current_year} Scanopy</p>"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::digest::payload::DiscoveryDigestPayload;
    use crate::server::networks::r#impl::DEFAULT_STALE_AFTER_HOURS;
    use uuid::Uuid;

    /// True if `s` still contains a `{snake_case}` placeholder — i.e. an
    /// unsubstituted template variable (`{plan_name}`, `{base_url}`, `{utm}`,
    /// …). Email HTML uses inline styles, so a `{` followed by a lowercase /
    /// underscore identifier then `}` can only be a leftover token.
    fn has_placeholder(s: &str) -> bool {
        let bytes = s.as_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            if b != b'{' {
                continue;
            }
            let mut j = i + 1;
            while j < bytes.len() && (bytes[j].is_ascii_lowercase() || bytes[j] == b'_') {
                j += 1;
            }
            if j > i + 1 && bytes.get(j) == Some(&b'}') {
                return true;
            }
        }
        false
    }

    /// Render an email end-to-end and assert it is fully substituted: a
    /// non-empty subject + HTML with no leftover `{token}` placeholders. This
    /// is real coverage against forgetting a `.replace(...)`, not a stub impl.
    fn assert_fully_rendered(email: &dyn Email) {
        let subject = email.subject();
        assert!(!subject.is_empty(), "empty subject");
        assert!(
            !has_placeholder(&subject),
            "leftover placeholder in subject: {subject}"
        );
        for self_hosted in [false, true] {
            let html = email.render_html("https://app.example.test", self_hosted);
            assert!(!html.is_empty(), "empty html");
            assert!(
                !has_placeholder(&html),
                "leftover placeholder in html for subject: {subject} (self_hosted={self_hosted})"
            );
        }
    }

    #[test]
    fn every_email_fully_renders() {
        // Auth
        assert_fully_rendered(&PasswordReset {
            url: "https://app.example.test",
            token: "reset-token",
        });
        assert_fully_rendered(&Verification {
            url: "https://app.example.test",
            token: "verify-token",
        });
        assert_fully_rendered(&PasswordChanged {
            timestamp: "2026-01-01 00:00 UTC",
        });
        assert_fully_rendered(&OidcLinked {
            provider_name: "Google",
        });
        assert_fully_rendered(&OidcUnlinked {
            provider_name: "Google",
        });
        assert_fully_rendered(&EmailChangedOld {
            new_email: "new@example.test",
        });

        // Onboarding
        assert_fully_rendered(&Invite {
            url: "https://app.example.test/invite/abc",
            inviter: "owner@example.test",
        });

        assert_fully_rendered(&DiscoveryGuide {
            daemon_name: "daemon-1",
            network_name: "Home",
        });

        // Daemon
        assert_fully_rendered(&InstallCommand {
            install_command: "curl … | sh",
            os: "linux",
        });
        assert_fully_rendered(&DaemonStandby {
            daemon_name: "daemon-1",
            network_name: "Home",
        });
        assert_fully_rendered(&DaemonUnreachable {
            daemon_name: "daemon-1",
            network_name: "Home",
        });
        assert_fully_rendered(&DaemonSunset {
            daemon_names: &["daemon-1", "daemon-2"],
            sunset_date: "November 1, 2026",
        });

        // Account
        assert_fully_rendered(&OrganizationDeleted);

        // Billing
        assert_fully_rendered(&TrialStarted {
            plan_name: "Pro",
            trial_days: 14,
            billing_period: "Monthly",
        });
        for has_payment in [true, false] {
            assert_fully_rendered(&TrialEnding {
                has_payment,
                plan_name: "Pro",
                billing_period: "Monthly",
                hosts_count: 12,
                networks_count: 3,
                daemons_count: 2,
                services_count: 20,
                days_into_trial: 11,
            });
        }
        assert_fully_rendered(&TrialExpired {
            plan_name: "Pro",
            billing_period: "Monthly",
        });
        assert_fully_rendered(&TrialConverted {
            plan_name: "Pro",
            billing_period: "Monthly",
        });
        assert_fully_rendered(&PlanChanged { plan_name: "Pro" });
        assert_fully_rendered(&SubscriptionCancelled {
            period_end_date: "January 1, 2026",
        });
        assert_fully_rendered(&PaymentMethodAdded);
        assert_fully_rendered(&PaymentMethodRemoved);
        assert_fully_rendered(&PaymentRecovered { amount: "$14.99" });
        assert_fully_rendered(&PaymentFailed);
        assert_fully_rendered(&PaymentActionRequired {
            cta_href: "https://billing.example.test/invoice/abc",
        });
        assert_fully_rendered(&CancellationInitiated {
            period_end: "January 1, 2026",
        });
        assert_fully_rendered(&SubscriptionReactivated);
        assert_fully_rendered(&SubscriptionPaused {
            resumes_at: "July 1, 2026",
        });
        assert_fully_rendered(&SubscriptionResumed);
        assert_fully_rendered(&CheckoutCompleted { plan_name: "Pro" });
        assert_fully_rendered(&UsageSummary {
            period: "Dec 1, 2025 – Jan 1, 2026",
            invoice_date: "January 1, 2026",
            total: "$14.99",
            attachment: None,
            hosted_invoice_url: Some("https://billing.example.test/invoice/abc"),
        });
        // Attached variant: PDF present, no hosted-URL fallback needed.
        assert_fully_rendered(&UsageSummary {
            period: "Dec 1, 2025 – Jan 1, 2026",
            invoice_date: "January 1, 2026",
            total: "$14.99",
            attachment: Some(EmailAttachment {
                filename: "scanopy-invoice-in_123.pdf".to_string(),
                content_type: "application/pdf".to_string(),
                bytes: vec![0x25, 0x50, 0x44, 0x46],
            }),
            hosted_invoice_url: None,
        });
        for has_overage in [true, false] {
            assert_fully_rendered(&PlanLimitApproaching {
                first_name: None,
                limit_type: "hosts",
                current_count: 8,
                limit: 10,
                plan_name: "Pro",
                has_overage,
            });
            assert_fully_rendered(&PlanLimitReached {
                first_name: Some("Ada"),
                limit_type: "hosts",
                current_count: 10,
                limit: 10,
                plan_name: "Pro",
                has_overage,
            });
        }

        // Digest
        let payload = DiscoveryDigestPayload {
            session_id: Uuid::nil(),
            network_id: Uuid::nil(),
            network_name: "Home".to_string(),
            started_at: chrono::Utc::now(),
            finished_at: chrono::Utc::now(),
            stale_after_hours: DEFAULT_STALE_AFTER_HOURS,
            subnets_scanned: vec![],
            hosts_added: vec![],
            hosts_stale: vec![],
            hosts_changed: vec![],
            vlans_added: vec![],
            vlans_stale: vec![],
            recipients: vec![],
        };
        assert_fully_rendered(&DiscoveryDigest {
            payload: &payload,
            base_url: "https://app.example.test",
        });

        assert_fully_rendered(&SelfHostedWelcome {
            plan_name: "Self-Hosted Standard",
            trial_days: Some(14),
            deployment_assistance: false,
        });
        assert_fully_rendered(&SelfHostedWelcome {
            plan_name: "Self-Hosted Plus",
            trial_days: None,
            deployment_assistance: true,
        });
    }

    /// Visit every email (every distinct variant) once, paired with a stable
    /// snapshot name. Single source of truth for the render/snapshot tests.
    fn for_each_email(mut f: impl FnMut(&str, &dyn Email)) {
        f(
            "password_reset",
            &PasswordReset {
                url: "https://app.example.test",
                token: "reset-token",
            },
        );
        f(
            "verification",
            &Verification {
                url: "https://app.example.test",
                token: "verify-token",
            },
        );
        f(
            "password_changed",
            &PasswordChanged {
                timestamp: "2026-01-01 00:00 UTC",
            },
        );
        f(
            "oidc_linked",
            &OidcLinked {
                provider_name: "Google",
            },
        );
        f(
            "oidc_unlinked",
            &OidcUnlinked {
                provider_name: "Google",
            },
        );
        f(
            "email_changed_old",
            &EmailChangedOld {
                new_email: "new@example.test",
            },
        );
        f(
            "invite",
            &Invite {
                url: "https://app.example.test/invite/abc",
                inviter: "owner@example.test",
            },
        );
        f(
            "discovery_guide",
            &DiscoveryGuide {
                daemon_name: "daemon-1",
                network_name: "Home",
            },
        );
        f(
            "install_command",
            &InstallCommand {
                install_command: "curl … | sh",
                os: "linux",
            },
        );
        f(
            "daemon_standby",
            &DaemonStandby {
                daemon_name: "daemon-1",
                network_name: "Home",
            },
        );
        f(
            "daemon_unreachable",
            &DaemonUnreachable {
                daemon_name: "daemon-1",
                network_name: "Home",
            },
        );
        f(
            "daemon_sunset",
            &DaemonSunset {
                daemon_names: &["daemon-1", "daemon-2"],
                sunset_date: "November 1, 2026",
            },
        );
        f("organization_deleted", &OrganizationDeleted);
        f(
            "trial_started",
            &TrialStarted {
                plan_name: "Pro",
                trial_days: 14,
                billing_period: "Monthly",
            },
        );
        f(
            "trial_ending_has_payment",
            &TrialEnding {
                has_payment: true,
                plan_name: "Pro",
                billing_period: "Monthly",
                hosts_count: 12,
                networks_count: 3,
                daemons_count: 2,
                services_count: 20,
                days_into_trial: 11,
            },
        );
        f(
            "trial_ending_no_payment",
            &TrialEnding {
                has_payment: false,
                plan_name: "Pro",
                billing_period: "Monthly",
                hosts_count: 12,
                networks_count: 3,
                daemons_count: 2,
                services_count: 20,
                days_into_trial: 11,
            },
        );
        f(
            "trial_expired",
            &TrialExpired {
                plan_name: "Pro",
                billing_period: "Monthly",
            },
        );
        f(
            "trial_converted",
            &TrialConverted {
                plan_name: "Pro",
                billing_period: "Monthly",
            },
        );
        f("plan_changed", &PlanChanged { plan_name: "Pro" });
        f(
            "subscription_cancelled",
            &SubscriptionCancelled {
                period_end_date: "January 1, 2026",
            },
        );
        f("payment_method_added", &PaymentMethodAdded);
        f("payment_method_removed", &PaymentMethodRemoved);
        f("payment_recovered", &PaymentRecovered { amount: "$14.99" });
        f("payment_failed", &PaymentFailed);
        f(
            "payment_action_required",
            &PaymentActionRequired {
                cta_href: "https://billing.example.test/invoice/abc",
            },
        );
        f(
            "cancellation_initiated",
            &CancellationInitiated {
                period_end: "January 1, 2026",
            },
        );
        f("subscription_reactivated", &SubscriptionReactivated);
        f(
            "subscription_paused",
            &SubscriptionPaused {
                resumes_at: "July 1, 2026",
            },
        );
        f("subscription_resumed", &SubscriptionResumed);
        f(
            "checkout_completed",
            &CheckoutCompleted { plan_name: "Pro" },
        );
        f(
            "self_hosted_welcome_trial",
            &SelfHostedWelcome {
                plan_name: "Self-Hosted Standard",
                trial_days: Some(14),
                deployment_assistance: false,
            },
        );
        f(
            "self_hosted_welcome_deployment_assistance",
            &SelfHostedWelcome {
                plan_name: "Self-Hosted Plus",
                trial_days: None,
                deployment_assistance: true,
            },
        );
        f(
            "usage_summary_attached",
            &UsageSummary {
                period: "Dec 1, 2025 – Jan 1, 2026",
                invoice_date: "January 1, 2026",
                total: "$14.99",
                attachment: Some(EmailAttachment {
                    filename: "scanopy-invoice-in_123.pdf".to_string(),
                    content_type: "application/pdf".to_string(),
                    bytes: vec![0x25, 0x50, 0x44, 0x46],
                }),
                hosted_invoice_url: None,
            },
        );
        f(
            "usage_summary_hosted_link",
            &UsageSummary {
                period: "Dec 1, 2025 – Jan 1, 2026",
                invoice_date: "January 1, 2026",
                total: "$14.99",
                attachment: None,
                hosted_invoice_url: Some("https://billing.example.test/invoice/abc"),
            },
        );
        f(
            "plan_limit_approaching_overage",
            &PlanLimitApproaching {
                first_name: None,
                limit_type: "hosts",
                current_count: 8,
                limit: 10,
                plan_name: "Pro",
                has_overage: true,
            },
        );
        f(
            "plan_limit_approaching_no_overage",
            &PlanLimitApproaching {
                first_name: None,
                limit_type: "hosts",
                current_count: 8,
                limit: 10,
                plan_name: "Pro",
                has_overage: false,
            },
        );
        f(
            "plan_limit_reached_overage",
            &PlanLimitReached {
                first_name: Some("Ada"),
                limit_type: "hosts",
                current_count: 10,
                limit: 10,
                plan_name: "Pro",
                has_overage: true,
            },
        );
        f(
            "plan_limit_reached_no_overage",
            &PlanLimitReached {
                first_name: Some("Ada"),
                limit_type: "hosts",
                current_count: 10,
                limit: 10,
                plan_name: "Pro",
                has_overage: false,
            },
        );

        let payload = DiscoveryDigestPayload {
            session_id: Uuid::nil(),
            network_id: Uuid::nil(),
            network_name: "Home".to_string(),
            started_at: chrono::Utc::now(),
            finished_at: chrono::Utc::now(),
            stale_after_hours: DEFAULT_STALE_AFTER_HOURS,
            subnets_scanned: vec![],
            hosts_added: vec![],
            hosts_stale: vec![],
            hosts_changed: vec![],
            vlans_added: vec![],
            vlans_stale: vec![],
            recipients: vec![],
        };
        f(
            "discovery_digest",
            &DiscoveryDigest {
                payload: &payload,
                base_url: "https://app.example.test",
            },
        );
    }

    /// Byte-for-byte fidelity net for the chrome-helper refactor: every email's
    /// `body_html()` must match its committed golden. `discovery_digest` carries
    /// `started_at`/`finished_at` timestamps, so its body is non-deterministic —
    /// snapshot only the deterministic emails. Regenerate with
    /// `UPDATE_EMAIL_SNAPSHOTS=1 cargo test --lib email_body_snapshots`.
    #[test]
    fn email_body_snapshots() {
        let dir = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/server/email/messages/snapshots"
        );
        let update = std::env::var("UPDATE_EMAIL_SNAPSHOTS").is_ok();
        if update {
            std::fs::create_dir_all(dir).unwrap();
        }
        for_each_email(|name, email| {
            if name == "discovery_digest" {
                return;
            }
            let path = format!("{dir}/{name}.html");
            let actual = email.body_html();
            if update {
                std::fs::write(&path, &actual).unwrap();
            } else {
                let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| {
                    panic!("missing snapshot {path}; run with UPDATE_EMAIL_SNAPSHOTS=1")
                });
                assert_eq!(actual, expected, "body_html drift for {name}");
            }
        });
    }

    /// The footer's sender-identification block is gated on deployment type,
    /// and every email carries the Manage Preferences deep link.
    #[test]
    fn footer_gates_on_deployment() {
        let cloud = PaymentFailed.render_html("https://app.example.test", false);
        let self_hosted = PaymentFailed.render_html("https://app.example.test", true);

        // Cloud: Scanopy LLC is the sender, so disclose the LLC + postal address.
        assert!(cloud.contains("Scanopy LLC. All rights reserved."));
        assert!(cloud.contains("418 Broadway Ste N, Albany, NY 12207"));

        // Self-hosted: operator is the sender — no LLC/address disclosure.
        assert!(!self_hosted.contains("Scanopy LLC"));
        assert!(!self_hosted.contains("418 Broadway"));
        assert!(self_hosted.contains("Scanopy</p>"));

        // Both: Manage Preferences deep link to Settings → Email.
        for html in [&cloud, &self_hosted] {
            assert!(html.contains("Manage email preferences"));
            assert!(html.contains("/?modal=settings&tab=email&utm_source=email"));
        }
    }
}
