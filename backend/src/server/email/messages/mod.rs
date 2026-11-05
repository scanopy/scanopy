//! Transactional email messages.
//!
//! Each email lives in its own file as a plain data struct holding that
//! message's template variables, plus an `impl Email`. Adding a new email is:
//! create a file here, define the struct, `impl Email` — no central registry
//! to edit. The transports ([`super::smtp`] / [`super::logging`]) dispatch on
//! `&dyn Email` and never need to know which concrete email they're sending.

mod airgap_expiring;
mod airgap_renewal;
mod cancellation_initiated;
mod checkout_completed;
mod compose;
mod daemon_scans_failing;
mod daemon_standby;
mod daemon_sunset;
mod daemon_unreachable;
mod discovery_digest;
mod discovery_guide;
mod email_changed_old;
mod install_command;
mod invite;
mod invoice_credited;
mod invoice_finalization_failed;
mod invoice_issued;
mod invoice_overdue;
mod licensed_server_upgrade;
pub mod links;
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
mod release_notes;
mod self_hosted_license_ended;
mod self_hosted_license_resumed;
mod self_hosted_payment_failed;
mod self_hosted_plan_changed;
mod self_hosted_trial_ending;
mod self_hosted_welcome;
mod subscription_cancelled;
mod subscription_paused;
mod subscription_reactivated;
mod subscription_resumed;
mod terms_update;
mod trial_converted;
mod trial_ending;
mod trial_expired;
mod trial_extended;
mod trial_started;
mod usage_summary;
mod verification;

use crate::server::shared::logos::Logo;
use strum::{EnumIter, IntoStaticStr};

use crate::server::shared::types::{
    Color,
    icon::Icon,
    metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
};

pub use airgap_expiring::AirgapExpiring;
pub use airgap_renewal::AirgapRenewal;
pub use cancellation_initiated::CancellationInitiated;
pub use checkout_completed::CheckoutCompleted;
pub use compose::{BILLING_DETAILS_TAGLINE, Body, Content};
pub use daemon_scans_failing::DaemonScansFailing;
pub use daemon_standby::DaemonStandby;
pub use daemon_sunset::DaemonSunset;
pub use daemon_unreachable::DaemonUnreachable;
pub use discovery_digest::DiscoveryDigest;
pub use discovery_guide::DiscoveryGuide;
pub use email_changed_old::EmailChangedOld;
pub use install_command::InstallCommand;
pub use invite::Invite;
pub use invoice_credited::InvoiceCredited;
pub use invoice_finalization_failed::InvoiceFinalizationFailed;
pub use invoice_issued::InvoiceIssued;
pub use invoice_overdue::InvoiceOverdue;
pub use licensed_server_upgrade::LicensedServerUpgrade;
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
pub use release_notes::ReleaseNotes;
pub use self_hosted_license_ended::SelfHostedLicenseEnded;
pub use self_hosted_license_resumed::SelfHostedLicenseResumed;
pub use self_hosted_payment_failed::SelfHostedPaymentFailed;
pub use self_hosted_plan_changed::SelfHostedPlanChanged;
pub use self_hosted_trial_ending::SelfHostedTrialEnding;
pub use self_hosted_welcome::SelfHostedWelcome;
pub use subscription_cancelled::SubscriptionCancelled;
pub use subscription_paused::SubscriptionPaused;
pub use subscription_reactivated::SubscriptionReactivated;
pub use subscription_resumed::SubscriptionResumed;
pub use terms_update::TermsUpdate;
pub use trial_converted::TrialConverted;
pub use trial_ending::TrialEnding;
pub use trial_expired::TrialExpired;
pub use trial_extended::{ExtendedTrialEnd, TrialExtended};
pub use trial_started::TrialStarted;
pub use usage_summary::UsageSummary;
pub use verification::Verification;

/// What an email is about. Each email declares one, and the category decides
/// whether the recipient can pause it, whether an org locked out of this app
/// still gets it, and the tag analytics groups it under.
///
/// A pausable category's id is its `EmailSettings` field name, which is how
/// Settings → Email finds a toggle's metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum EmailCategory {
    /// Sign-in, verification, and account-security notices.
    Auth,
    /// Account-level lifecycle (e.g. organization deletion).
    Account,
    /// Receipts, payment and subscription notices, and the license emails.
    Billing,
    /// Trial reminders and plan-limit nudges about the cloud app.
    TrialAndUsage,
    /// The daemon install command a user asked to be sent.
    DaemonSetup,
    /// Daemon connectivity notices (standby, unreachable, sunset).
    DaemonAlerts,
    /// An invitation to join an organization.
    Invitation,
    /// First-run walkthrough nudges.
    ProductOnboarding,
    /// Per-discovery-session digests.
    DiscoveryDigest,
    /// What changed in a new Scanopy release.
    ProductUpdates,
    /// Notice of a change to Scanopy's terms, privacy policy or other legal
    /// documents.
    Legal,
}

impl EmailCategory {
    /// Whether the recipient can switch this category off in Settings →
    /// Email. Required categories carry security, billing, and
    /// account-lifecycle messages and ignore preferences; Settings shows
    /// them as one disabled "Required emails" row.
    pub fn pausable(self) -> bool {
        match self {
            Self::Auth
            | Self::Account
            | Self::Billing
            | Self::DaemonSetup
            | Self::Invitation
            | Self::Legal => false,
            Self::TrialAndUsage
            | Self::DaemonAlerts
            | Self::ProductOnboarding
            | Self::DiscoveryDigest
            | Self::ProductUpdates => true,
        }
    }

    /// Whether the cloud app sends this category to an org locked out of it
    /// by a self-hosted license plan (`EmailService::self_hosted_plan_locked`).
    /// `EmailService::dispatch` enforces it, and Settings → Email hides a
    /// toggle that has nothing to act on.
    pub fn sent_while_self_hosted_plan_locked(self) -> bool {
        match self {
            Self::Auth
            | Self::Account
            | Self::Billing
            | Self::DaemonSetup
            | Self::Invitation
            | Self::Legal => true,
            // The cloud trial and plan-limit emails are about the cloud app.
            // A licensed trial gets SelfHostedWelcome and SelfHostedTrialEnding
            // instead, and both are Billing.
            Self::TrialAndUsage => false,
            // Daemons left on the cloud org are expected to go quiet.
            Self::DaemonAlerts => false,
            // A guide to discovery and topology in an app it can't open.
            Self::ProductOnboarding => false,
            // Every link leads into the locked app, and the org's scans run
            // on its own server.
            Self::DiscoveryDigest => false,
            // Its owners get LicensedServerUpgrade. Its other members get no
            // release email: `ReleaseNotesService` leaves them out, and their
            // own server announces each release to them.
            Self::ProductUpdates => true,
        }
    }

    /// Stable lowercase tag, used as the default `utm_medium`. Coarser than the category, and unchanged since before
    /// categories split by pausability, so analytics keeps its history.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Auth => "auth",
            Self::Account => "account",
            Self::Billing | Self::TrialAndUsage => "billing",
            Self::DaemonSetup | Self::DaemonAlerts => "daemon",
            Self::Invitation | Self::ProductOnboarding => "onboarding",
            Self::DiscoveryDigest => "digest",
            Self::ProductUpdates => "product",
            Self::Legal => "legal",
        }
    }
}

/// The id is also the category segment of the one-click unsubscribe URL, so
/// renaming a pausable variant breaks the links in emails already sent.
impl HasId for EmailCategory {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for EmailCategory {
    fn color(&self) -> Color {
        Color::Blue
    }

    fn icon(&self) -> Icon {
        Icon::Mail
    }
}

impl TypeMetadataProvider for EmailCategory {
    fn name(&self) -> &'static str {
        match self {
            Self::Auth => "Sign-in and security",
            Self::Account => "Account",
            Self::Billing => "Billing and license",
            Self::TrialAndUsage => "Trial and usage",
            Self::DaemonSetup => "Daemon setup",
            Self::DaemonAlerts => "Daemon alerts",
            Self::Invitation => "Invitations",
            Self::ProductOnboarding => "Product onboarding",
            Self::DiscoveryDigest => "Discovery digest",
            Self::ProductUpdates => "Product updates",
            Self::Legal => "Legal notices",
        }
    }

    fn metadata(&self) -> serde_json::Value {
        serde_json::json!({
            "pausable": self.pausable(),
            "sent_while_self_hosted_plan_locked": self.sent_while_self_hosted_plan_locked()
        })
    }
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

    /// What this email is about. Required of every email, so a new message
    /// cannot be added without deciding whether it can be paused and whether
    /// a locked org gets it.
    fn category(&self) -> EmailCategory;

    /// The `utm_campaign` slug for this email's CTAs.
    fn campaign(&self) -> &'static str;

    /// The `utm_medium` value for this email's CTAs. Defaults to the
    /// category's tag (`auth`, `billing`, `onboarding`, `daemon`, `account`,
    /// `digest`) so analytics gets a useful split out of the box. Override
    /// only when a particular email warrants a different bucket.
    fn utm_medium(&self) -> &'static str {
        self.category().tag()
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
        let year = crate::server::shared::clock::now().format("%Y").to_string();
        let footer_legal = if self_hosted {
            FOOTER_LEGAL_SELF_HOSTED
        } else {
            FOOTER_LEGAL_CLOUD
        };
        format!("{}{}{}", EMAIL_HEADER, self.body_html(), EMAIL_FOOTER)
            // The logos come from the list the credits read (`shared/logos.rs`).
            .replace("{scanopy_logo}", &Logo::Scanopy.url())
            .replace("{discord_logo}", &Logo::Discord.url())
            .replace("{github_logo}", &Logo::GitHub.url())
            .replace("{footer_legal}", footer_legal)
            // Before the two tokens below, which the link itself contains.
            .replace("{preferences_url}", links::SETTINGS_EMAIL)
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

    /// Tags this email adds to the SES message tags every cloud send carries
    /// (`category`, `campaign`, `user_id`), as `(name, value)`. Values hold
    /// only ASCII letters, digits, `_` and `-`. Default-empty.
    fn ses_message_tags(&self) -> Vec<(&'static str, String)> {
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
                            <img src="{scanopy_logo}" alt="Scanopy" style="width: 80px; height: 80px; display: block;">
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
                                            <img src="{discord_logo}" alt="Discord" style="height: 24px; width: auto; display: block;">
                                        </a>
                                    </td>
                                    <td style="padding: 0 10px;">
                                        <a href="https://github.com/scanopy/scanopy" style="display: inline-block;">
                                            <img src="{github_logo}" alt="GitHub" style="height: 24px; width: auto; display: block;">
                                        </a>
                                    </td>
                                </tr>
                            </table>

                            <p style="margin: 0 0 12px 0; font-size: 12px; line-height: 18px; color: #9ca3af;"><a href="{preferences_url}" style="color: #6b7280; text-decoration: underline;">Manage email preferences</a></p>
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
    use crate::server::license::types::LicenseKeyType;
    use crate::server::shared::events::types::TrialExtensionSource;
    use crate::server::sites::r#impl::DEFAULT_STALE_AFTER_HOURS;
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
    fn assert_fully_rendered(name: &str, email: &dyn Email) {
        let subject = email.subject();
        assert!(!subject.is_empty(), "{name}: empty subject");
        assert!(
            !has_placeholder(&subject),
            "{name}: leftover placeholder in subject: {subject}"
        );
        for self_hosted in [false, true] {
            let html = email.render_html("https://app.example.test", self_hosted);
            assert!(!html.is_empty(), "{name}: empty html");
            assert!(
                !has_placeholder(&html),
                "{name}: leftover placeholder in html (self_hosted={self_hosted})"
            );
        }
    }

    #[test]
    fn every_email_fully_renders() {
        for_each_email(assert_fully_rendered);
    }

    /// Visit every email (every distinct variant) once, with a name for
    /// failure messages.
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
                site_name: "Home",
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
                site_name: "Home",
            },
        );
        f(
            "daemon_unreachable",
            &DaemonUnreachable {
                daemon_name: "daemon-1",
                site_name: "Home",
            },
        );
        f(
            "daemon_scans_failing",
            &DaemonScansFailing {
                daemon_name: "daemon-1",
                site_name: "Home",
                failed_runs: 3,
                discovery_names: &["Office LAN", "Server VLAN"],
                reason_name: "Daemon offline",
                reason_description: "The daemon stopped checking in with the server during the scan, so no progress or cancellation could get through. Check that the daemon is running and can reach the server.",
                log_command: Some(
                    "sudo grep -h '0b8f7c52-4d1e-4f3a-9a77-1c2d3e4f5a6b' /var/log/scanopy/*.log | sort -u",
                ),
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
                sites_count: 3,
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
                sites_count: 3,
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
        for source in [TrialExtensionSource::User, TrialExtensionSource::Staff] {
            for end in [
                ExtendedTrialEnd::Converts,
                ExtendedTrialEnd::NoPaymentMethod,
                ExtendedTrialEnd::CancellationScheduled,
            ] {
                for key_expires in [None, Some("March 3, 2027")] {
                    f(
                        &format!("trial_extended_{source}_{end:?}_{key_expires:?}"),
                        &TrialExtended {
                            source,
                            plan_name: "Pro",
                            billing_period: "Monthly",
                            days_added: 7,
                            new_trial_end: "March 1, 2027",
                            end,
                            key_expires,
                        },
                    );
                }
            }
        }
        f(
            "trial_converted",
            &TrialConverted {
                plan_name: "Pro",
                billing_period: "Monthly",
            },
        );
        f(
            "plan_changed",
            &PlanChanged {
                plan_name: "Pro",
                ended_license: None,
            },
        );
        f(
            "plan_changed_from_self_hosted",
            &PlanChanged {
                plan_name: "Pro",
                ended_license: Some(LicenseKeyType::Online),
            },
        );
        f(
            "plan_changed_from_self_hosted_airgapped",
            &PlanChanged {
                plan_name: "Pro",
                ended_license: Some(LicenseKeyType::Offline),
            },
        );
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
            "invoice_issued_with_po",
            &InvoiceIssued {
                plan_name: "Self-Hosted Standard",
                amount: "$4,000.00",
                due_date: "October 18, 2026",
                po_number: Some("PO-4471"),
                cta_href: "https://billing.example.test/invoice/abc",
            },
        );
        f(
            "invoice_issued",
            &InvoiceIssued {
                plan_name: "Self-Hosted Standard",
                amount: "$4,000.00",
                due_date: "October 18, 2026",
                po_number: None,
                cta_href: "https://billing.example.test/invoice/abc",
            },
        );
        f(
            "invoice_finalization_failed",
            &InvoiceFinalizationFailed {
                plan_name: "Self-Hosted Standard",
                reason: "We could not determine the customer's tax location.",
            },
        );
        f(
            "invoice_credited",
            &InvoiceCredited {
                plan_name: "Self-Hosted Standard",
                credit: "$1,999.98",
            },
        );
        f(
            "invoice_overdue_with_po",
            &InvoiceOverdue {
                plan_name: "Self-Hosted Standard",
                amount: "$4,000.00",
                due_date: "October 18, 2026",
                key_expires: "November 17, 2026",
                po_number: Some("PO-4471"),
                cta_href: "https://billing.example.test/invoice/abc",
            },
        );
        f(
            "invoice_overdue",
            &InvoiceOverdue {
                plan_name: "Self-Hosted Standard",
                amount: "$4,000.00",
                due_date: "October 18, 2026",
                key_expires: "November 17, 2026",
                po_number: None,
                cta_href: "https://billing.example.test/invoice/abc",
            },
        );
        f(
            "cancellation_initiated",
            &CancellationInitiated {
                period_end: "January 1, 2026",
                licensed: false,
            },
        );
        f(
            "cancellation_initiated_self_hosted",
            &CancellationInitiated {
                period_end: "January 1, 2026",
                licensed: true,
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
                onboarding_call: false,
            },
        );
        f(
            "self_hosted_welcome_onboarding_call",
            &SelfHostedWelcome {
                plan_name: "Self-Hosted Plus",
                trial_days: None,
                onboarding_call: true,
            },
        );
        f(
            "airgap_renewal",
            &AirgapRenewal {
                plan_name: "Self-Hosted Plus",
                current_key_expires: "October 15, 2026",
                renewed_through: "October 1, 2027",
            },
        );
        f(
            "airgap_expiring",
            &AirgapExpiring {
                plan_name: "Self-Hosted Plus",
                key_expires: "October 15, 2026",
                renews_at: "October 1, 2026",
                amount: "$6,000.00",
            },
        );
        f(
            "self_hosted_payment_failed_airgapped",
            &SelfHostedPaymentFailed {
                key_expires: "October 1, 2026",
                air_gapped: true,
            },
        );
        f(
            "self_hosted_payment_failed_online",
            &SelfHostedPaymentFailed {
                key_expires: "October 1, 2026",
                air_gapped: false,
            },
        );
        f(
            "self_hosted_trial_ending_has_payment",
            &SelfHostedTrialEnding {
                plan_name: "Self-Hosted Standard",
                billing_period: "Yearly",
                has_payment: true,
                key_expires: "October 8, 2026",
            },
        );
        f(
            "self_hosted_trial_ending_no_payment",
            &SelfHostedTrialEnding {
                plan_name: "Self-Hosted Standard",
                billing_period: "Yearly",
                has_payment: false,
                key_expires: "October 8, 2026",
            },
        );
        f(
            "self_hosted_license_ended_trial",
            &SelfHostedLicenseEnded {
                plan_name: "Self-Hosted Standard",
                was_trial: true,
                air_gapped: false,
                key_expires: "March 22, 2026",
                defaulted: false,
            },
        );
        f(
            "self_hosted_license_ended_cancelled_online",
            &SelfHostedLicenseEnded {
                plan_name: "Self-Hosted Plus",
                was_trial: false,
                air_gapped: false,
                key_expires: "March 22, 2026",
                defaulted: false,
            },
        );
        f(
            "self_hosted_license_ended_cancelled_airgapped",
            &SelfHostedLicenseEnded {
                plan_name: "Self-Hosted Plus",
                was_trial: false,
                air_gapped: true,
                key_expires: "March 22, 2026",
                defaulted: false,
            },
        );
        f(
            "self_hosted_license_ended_defaulted",
            &SelfHostedLicenseEnded {
                plan_name: "Self-Hosted Standard",
                was_trial: false,
                air_gapped: false,
                key_expires: "March 22, 2026",
                defaulted: true,
            },
        );
        f(
            "self_hosted_license_resumed_online",
            &SelfHostedLicenseResumed {
                plan_name: "Self-Hosted Standard",
                resumes_through: "March 22, 2027",
                air_gapped: false,
            },
        );
        f(
            "self_hosted_license_resumed_airgapped",
            &SelfHostedLicenseResumed {
                plan_name: "Self-Hosted Plus",
                resumes_through: "March 22, 2027",
                air_gapped: true,
            },
        );
        f(
            "self_hosted_plan_changed_online",
            &SelfHostedPlanChanged {
                plan_name: "Self-Hosted Plus",
                air_gapped: false,
            },
        );
        f(
            "self_hosted_plan_changed_airgapped",
            &SelfHostedPlanChanged {
                plan_name: "Self-Hosted Plus",
                air_gapped: true,
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
        let note = crate::server::release_notes::catalog::ReleaseNote::parse(
            "---\nversion: 0.18.0\ndate: 2026-10-11\ntitle: 'Faster discovery'\nemail: true\n---\n\n- Fix a thing\n\n![The discovery view](https://scanopy.net/changelog/v0.18.0/discovery.png)\n",
        )
        .unwrap();
        f("release_notes", &ReleaseNotes { note: &note });
        f(
            "licensed_server_upgrade",
            &LicensedServerUpgrade { note: &note },
        );

        let payload = DiscoveryDigestPayload {
            session_id: Uuid::nil(),
            site_id: Uuid::nil(),
            site_name: "Home".to_string(),
            started_at: crate::server::shared::clock::now(),
            finished_at: crate::server::shared::clock::now(),
            stale_after_hours: DEFAULT_STALE_AFTER_HOURS,
            subnets_scanned: vec![],
            hosts_added: vec![],
            hosts_stale: vec![],
            hosts_changed: vec![],
            vlans_added: vec![],
            vlans_stale: vec![],
            recipients: vec![],
            discovery_id: Some(Uuid::nil()),
            warnings: crate::server::digest::payload::WarningCounts {
                fix_credentials: 2,
                fix_other: 1,
                check_device: 0,
                no_action: 3,
                add_credentials: 1,
            },
            credentials_used: 4,
            credential_problems: vec![crate::server::digest::payload::CredentialProblem {
                name: Some("Lab switches".to_string()),
                type_label: "SNMP v2c".to_string(),
                hosts_reached: 0,
                warnings: 2,
            }],
            integrations_without_credentials: vec![
                crate::server::digest::payload::IntegrationWithoutCredential {
                    integration:
                        crate::server::credentials::r#impl::types::CredentialIntegration::Proxmox,
                    host_ids: vec![Uuid::nil(), Uuid::nil()],
                },
            ],
        };
        f(
            "discovery_digest",
            &DiscoveryDigest {
                payload: &payload,
                base_url: "https://app.example.test",
            },
        );
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
