use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use chrono::{DateTime, Utc};
use email_address::EmailAddress;
use semver::Version;
use uuid::Uuid;

use super::messages::{
    AirgapExpiring, AirgapRenewal, CancellationInitiated, CheckoutCompleted, DaemonStandby,
    DaemonSunset, DaemonUnreachable, DiscoveryDigest, DiscoveryGuide, Email, EmailAttachment,
    EmailChangedOld, EmailPreference, InstallCommand, Invite, InvoiceCredited,
    InvoiceFinalizationFailed, InvoiceIssued, InvoiceOverdue, OidcLinked, OidcUnlinked,
    OrganizationDeleted, PasswordChanged, PasswordReset, PaymentActionRequired, PaymentFailed,
    PaymentMethodAdded, PaymentMethodRemoved, PaymentRecovered, PlanChanged, PlanLimitApproaching,
    PlanLimitReached, SelfHostedLicenseEnded, SelfHostedLicenseResumed, SelfHostedPaymentFailed,
    SelfHostedPlanChanged, SelfHostedTrialEnding, SelfHostedWelcome, SubscriptionCancelled,
    SubscriptionPaused, SubscriptionReactivated, SubscriptionResumed, TrialConverted, TrialEnding,
    TrialExpired, TrialStarted, UsageSummary, Verification, links,
};
use super::transport::EmailTransport;
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    billing::types::base::{BillingInvoice, BillingPlan, BillingRate},
    config::DeploymentType,
    daemons::{r#impl::base::Daemon, service::DaemonService},
    digest::payload::DiscoveryDigestPayload,
    hosts::service::HostService,
    networks::{r#impl::Network, service::NetworkService},
    organizations::{
        r#impl::base::{LimitNotificationLevel, OrgNotifications, Organization},
        service::OrganizationService,
    },
    services::service::ServiceService,
    shared::{
        events::{
            traits::{Event, OrgScope},
            types::AnalyticsOperation,
        },
        services::traits::{CrudService, EventBusService},
        storage::filter::StorableFilter,
        types::metadata::TypeMetadataProvider,
    },
    users::{r#impl::base::User, service::UserService},
};

/// How far ahead an air-gapped organization is warned that its licence period
/// is ending. Long enough to cancel before the renewal bills, or to plan a
/// move to another plan, which only becomes possible once the key expires.
const AIRGAP_EXPIRY_WARNING_DAYS: i64 = 14;

/// Counts of entities discovered/created during the trial, plus elapsed days
/// since org creation. Populates the trial-ending email and the in-app trial
/// value recap card.
pub struct TrialRecapMetrics {
    pub hosts_count: u64,
    pub networks_count: u64,
    pub daemons_count: u64,
    pub services_count: u64,
    pub days_into_trial: i64,
}

/// Per-limit pending notification, collected during [`EmailService::check_plan_limits`]
/// and dispatched once the org owner has been resolved.
struct PendingLimitEmail {
    reached: bool,
    count: u64,
    limit: u64,
    has_overage: bool,
}

/// One limit whose stored notification level moves, plus the email that
/// announces an upward crossing. `email` is `None` for a drop back below the
/// threshold or when emails are suppressed.
struct LimitLevelChange {
    limit_type: &'static str,
    level: LimitNotificationLevel,
    email: Option<PendingLimitEmail>,
}

impl LimitLevelChange {
    /// Stores the new level in `notifications` when `send` succeeded (a change
    /// with no email passes `Ok`). A failed send leaves the old level, so the
    /// next entity change that crosses the threshold retries the email.
    /// Returns whether `notifications` changed.
    fn record(&self, notifications: &mut OrgNotifications, send: &Result<()>) -> bool {
        if send.is_err() {
            return false;
        }
        let stored = match self.limit_type {
            "hosts" => &mut notifications.hosts,
            "networks" => &mut notifications.networks,
            "seats" => &mut notifications.seats,
            _ => return false,
        };
        *stored = self.level.clone();
        true
    }
}

/// Email service: builds the right [`Email`] for each event and hands it to
/// the configured transport (Brevo or SMTP).
pub struct EmailService {
    transport: Box<dyn EmailTransport>,
    pub user_service: Arc<UserService>,
    pub organization_service: Arc<OrganizationService>,
    pub host_service: Arc<HostService>,
    pub network_service: Arc<NetworkService>,
    pub service_service: Arc<ServiceService>,
    pub daemon_service: Arc<DaemonService>,
    pub public_url: String,
    /// Deployment type of this instance — gates the footer's sender-identity
    /// block (cloud discloses Scanopy LLC; self-hosted does not).
    pub deployment_type: DeploymentType,
    /// HTTP client for fetching invoice PDFs from Stripe's public links before
    /// attaching them to billing emails.
    http: reqwest::Client,
}

impl EmailService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        transport: Box<dyn EmailTransport>,
        user_service: Arc<UserService>,
        organization_service: Arc<OrganizationService>,
        host_service: Arc<HostService>,
        network_service: Arc<NetworkService>,
        service_service: Arc<ServiceService>,
        daemon_service: Arc<DaemonService>,
        public_url: String,
        deployment_type: DeploymentType,
    ) -> Self {
        Self {
            transport,
            user_service,
            organization_service,
            host_service,
            network_service,
            service_service,
            daemon_service,
            public_url,
            deployment_type,
            http: reqwest::Client::new(),
        }
    }

    /// Render `email` against the installed `public_url` and send it to `to`.
    ///
    /// Pausable emails are gated on the recipient's preferences: if the user
    /// has the matching category switched off, the send is silently skipped.
    /// Required emails always send, and a recipient with no account yet
    /// (pre-signup) defaults to sending.
    async fn dispatch(&self, to: EmailAddress, email: &dyn Email) -> Result<()> {
        let recipient = self.user_service.get_by_email(&to).await?;
        if matches!(email.preference(), EmailPreference::Pausable(_))
            && let Some(user) = &recipient
            && !user.base.email_settings.allows(email.preference())
        {
            return Ok(());
        }
        self.transport
            .send(
                to,
                email,
                &self.public_url,
                self.deployment_type.is_self_hosted(),
            )
            .await?;

        // A recipient without an account (verification, invite) has no org
        // to attribute the send to.
        if let Some(user) = recipient {
            self.publish_email_sent(&user, email).await;
        }
        Ok(())
    }

    /// Record a send for analytics. Best-effort: the email has already gone.
    async fn publish_email_sent(&self, user: &User, email: &dyn Email) {
        if let Err(e) = self
            .organization_service
            .event_bus()
            .publish(Event::new(
                OrgScope {
                    organization_id: user.base.organization_id,
                },
                AnalyticsOperation::EmailSent {
                    utm_campaign: email.campaign().to_string(),
                    utm_medium: email.utm_medium().to_string(),
                    user_id: user.id,
                },
                AuthenticatedEntity::System,
            ))
            .await
        {
            tracing::warn!(
                user_id = %user.id,
                campaign = email.campaign(),
                error = %e,
                "Failed to publish email send",
            );
        }
    }

    /// True when an org on `plan` is locked out of this cloud app because it
    /// bought a self-hosted license here and runs Scanopy on its own server.
    /// The same two signals as the lock in `auth/middleware/billing.rs`.
    ///
    /// The deployment clause is what keeps this off the customer's own
    /// server: there the org holds the very same plan (`plan_for_license`),
    /// and its daemon, digest and limit emails are the ones that matter.
    fn self_hosted_plan_locked(&self, plan: Option<BillingPlan>) -> bool {
        !self.deployment_type.is_self_hosted()
            && plan.is_some_and(|plan| plan.license_plan().is_some())
    }

    /// [`Self::self_hosted_plan_locked`] for callers that hold only the org id.
    pub async fn organization_self_hosted_plan_locked(&self, org_id: &Uuid) -> Result<bool> {
        let plan = self
            .organization_service
            .get_by_id(org_id)
            .await?
            .and_then(|organization| organization.base.plan);
        Ok(self.self_hosted_plan_locked(plan))
    }

    // ========================================================================
    // Auth emails
    // ========================================================================

    pub async fn send_password_reset(
        &self,
        to: EmailAddress,
        url: String,
        token: String,
    ) -> Result<()> {
        self.dispatch(
            to,
            &PasswordReset {
                url: &url,
                token: &token,
            },
        )
        .await
    }

    pub async fn send_invite(
        &self,
        to: EmailAddress,
        from: EmailAddress,
        url: String,
    ) -> Result<()> {
        self.dispatch(
            to,
            &Invite {
                url: &url,
                inviter: from.as_str(),
            },
        )
        .await
    }

    pub async fn send_verification_email(
        &self,
        to: EmailAddress,
        url: String,
        token: String,
    ) -> Result<()> {
        self.dispatch(
            to,
            &Verification {
                url: &url,
                token: &token,
            },
        )
        .await
    }

    pub async fn send_password_changed_email(
        &self,
        to: EmailAddress,
        timestamp: &str,
    ) -> Result<()> {
        self.dispatch(to, &PasswordChanged { timestamp }).await
    }

    pub async fn send_oidc_linked_email(
        &self,
        to: EmailAddress,
        provider_name: &str,
    ) -> Result<()> {
        self.dispatch(to, &OidcLinked { provider_name }).await
    }

    pub async fn send_oidc_unlinked_email(
        &self,
        to: EmailAddress,
        provider_name: &str,
    ) -> Result<()> {
        self.dispatch(to, &OidcUnlinked { provider_name }).await
    }

    pub async fn send_email_changed_old_email(
        &self,
        to: EmailAddress,
        new_email: EmailAddress,
    ) -> Result<()> {
        self.dispatch(
            to,
            &EmailChangedOld {
                new_email: new_email.as_str(),
            },
        )
        .await
    }

    // ========================================================================
    // Billing emails
    // ========================================================================

    pub async fn send_trial_started_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
        trial_days: u32,
        billing_period: &str,
    ) -> Result<()> {
        self.dispatch(
            to,
            &TrialStarted {
                plan_name,
                trial_days,
                billing_period,
            },
        )
        .await
    }

    pub async fn send_trial_ending_email(
        &self,
        to: EmailAddress,
        org_id: Uuid,
        plan_name: &str,
        has_payment: bool,
        billing_period: &str,
    ) -> Result<()> {
        let metrics = self.compute_trial_recap_metrics(org_id).await?;
        self.dispatch(
            to,
            &TrialEnding {
                has_payment,
                plan_name,
                billing_period,
                hosts_count: metrics.hosts_count,
                networks_count: metrics.networks_count,
                daemons_count: metrics.daemons_count,
                services_count: metrics.services_count,
                days_into_trial: metrics.days_into_trial,
            },
        )
        .await
    }

    /// The self-hosted counterpart of [`Self::send_trial_ending_email`]. It
    /// computes no recap: the hosts and scans a license customer cares about
    /// are on their own server, so the cloud org's counts are all zero.
    pub async fn send_self_hosted_trial_ending_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
        has_payment: bool,
        billing_period: &str,
        key_expires: &str,
    ) -> Result<()> {
        self.dispatch(
            to,
            &SelfHostedTrialEnding {
                plan_name,
                billing_period,
                has_payment,
                key_expires,
            },
        )
        .await
    }

    pub async fn send_self_hosted_license_ended_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
        was_trial: bool,
        air_gapped: bool,
        key_expires: &str,
        defaulted: bool,
    ) -> Result<()> {
        self.dispatch(
            to,
            &SelfHostedLicenseEnded {
                plan_name,
                was_trial,
                air_gapped,
                key_expires,
                defaulted,
            },
        )
        .await
    }

    /// A lapsed self-hosted org settled the invoice we wrote off, so its plan
    /// and licence are back with the service it was still owed.
    pub async fn send_self_hosted_license_resumed_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
        resumes_through: &str,
        air_gapped: bool,
    ) -> Result<()> {
        self.dispatch(
            to,
            &SelfHostedLicenseResumed {
                plan_name,
                resumes_through,
                air_gapped,
            },
        )
        .await
    }

    pub async fn send_self_hosted_plan_changed_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
        air_gapped: bool,
    ) -> Result<()> {
        self.dispatch(
            to,
            &SelfHostedPlanChanged {
                plan_name,
                air_gapped,
            },
        )
        .await
    }

    pub async fn send_trial_expired_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
        billing_period: &str,
    ) -> Result<()> {
        self.dispatch(
            to,
            &TrialExpired {
                plan_name,
                billing_period,
            },
        )
        .await
    }

    pub async fn send_trial_converted_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
        billing_period: &str,
    ) -> Result<()> {
        self.dispatch(
            to,
            &TrialConverted {
                plan_name,
                billing_period,
            },
        )
        .await
    }

    pub async fn send_plan_changed_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
        license_key_stops: bool,
    ) -> Result<()> {
        self.dispatch(
            to,
            &PlanChanged {
                plan_name,
                license_key_stops,
            },
        )
        .await
    }

    pub async fn send_subscription_cancelled_email(
        &self,
        to: EmailAddress,
        period_end_date: &str,
    ) -> Result<()> {
        self.dispatch(to, &SubscriptionCancelled { period_end_date })
            .await
    }

    pub async fn send_organization_deleted_email(&self, to: EmailAddress) -> Result<()> {
        self.dispatch(to, &OrganizationDeleted).await
    }

    pub async fn send_payment_method_added_email(&self, to: EmailAddress) -> Result<()> {
        self.dispatch(to, &PaymentMethodAdded).await
    }

    pub async fn send_payment_method_removed_email(&self, to: EmailAddress) -> Result<()> {
        self.dispatch(to, &PaymentMethodRemoved).await
    }

    pub async fn send_payment_recovered_email(&self, to: EmailAddress, amount: &str) -> Result<()> {
        self.dispatch(to, &PaymentRecovered { amount }).await
    }

    pub async fn send_cancellation_initiated_email(
        &self,
        to: EmailAddress,
        period_end: &str,
        licensed: bool,
    ) -> Result<()> {
        self.dispatch(
            to,
            &CancellationInitiated {
                period_end,
                licensed,
            },
        )
        .await
    }

    pub async fn send_subscription_reactivated_email(&self, to: EmailAddress) -> Result<()> {
        self.dispatch(to, &SubscriptionReactivated).await
    }

    pub async fn send_subscription_paused_email(
        &self,
        to: EmailAddress,
        resumes_at: &str,
    ) -> Result<()> {
        self.dispatch(to, &SubscriptionPaused { resumes_at }).await
    }

    pub async fn send_subscription_resumed_email(&self, to: EmailAddress) -> Result<()> {
        self.dispatch(to, &SubscriptionResumed).await
    }

    pub async fn send_checkout_completed_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
    ) -> Result<()> {
        self.dispatch(to, &CheckoutCompleted { plan_name }).await
    }

    pub async fn send_self_hosted_welcome_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
        trial_days: Option<u32>,
        onboarding_call: bool,
    ) -> Result<()> {
        self.dispatch(
            to,
            &SelfHostedWelcome {
                plan_name,
                trial_days,
                onboarding_call,
            },
        )
        .await
    }

    pub async fn send_payment_failed_email(&self, to: EmailAddress) -> Result<()> {
        self.dispatch(to, &PaymentFailed).await
    }

    pub async fn send_airgap_renewal_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
        current_key_expires: &str,
        renewed_through: &str,
    ) -> Result<()> {
        self.dispatch(
            to,
            &AirgapRenewal {
                plan_name,
                current_key_expires,
                renewed_through,
            },
        )
        .await
    }

    /// Warn an air-gapped organization before its renewal: the key on their
    /// server is about to stop, and this is also when their plan becomes
    /// changeable, since leaving an air-gapped key is refused until then.
    pub async fn send_airgap_expiring_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
        key_expires: &str,
        renews_at: &str,
        amount: &str,
    ) -> Result<()> {
        self.dispatch(
            to,
            &AirgapExpiring {
                plan_name,
                key_expires,
                renews_at,
                amount,
            },
        )
        .await
    }

    pub async fn send_self_hosted_payment_failed_email(
        &self,
        to: EmailAddress,
        key_expires: &str,
        air_gapped: bool,
    ) -> Result<()> {
        self.dispatch(
            to,
            &SelfHostedPaymentFailed {
                key_expires,
                air_gapped,
            },
        )
        .await
    }

    pub async fn send_payment_action_required_email(
        &self,
        to: EmailAddress,
        hosted_invoice_url: Option<String>,
    ) -> Result<()> {
        let cta_href = hosted_invoice_url.unwrap_or_else(|| links::SETTINGS_BILLING.to_string());
        self.dispatch(
            to,
            &PaymentActionRequired {
                cta_href: &cta_href,
            },
        )
        .await
    }

    /// A plan change that credits more than it charges. Nothing is payable, so
    /// this says what the credit is and where it goes rather than presenting a
    /// negative invoice.
    pub async fn send_invoice_credited_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
        invoice: &BillingInvoice,
    ) -> Result<()> {
        let credit = format_cents(invoice.total_cents.abs(), &invoice.currency);
        self.dispatch(
            to,
            &InvoiceCredited {
                plan_name,
                credit: &credit,
            },
        )
        .await
    }

    /// A sent invoice for a self-hosted licence was finalized. Stripe mails the
    /// document; this says what it covers and that the licence keeps working
    /// until it is paid.
    pub async fn send_invoice_issued_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
        invoice: &BillingInvoice,
        po_number: Option<String>,
    ) -> Result<()> {
        let amount = format_cents(invoice.amount_due_cents, &invoice.currency);
        let due_date = invoice.due_date.map(format_timestamp).unwrap_or_default();
        let cta_href = invoice
            .hosted_invoice_url
            .clone()
            .unwrap_or_else(|| links::SETTINGS_BILLING.to_string());
        self.dispatch(
            to,
            &InvoiceIssued {
                plan_name,
                amount: &amount,
                due_date: &due_date,
                po_number: po_number.as_deref(),
                cta_href: &cta_href,
            },
        )
        .await
    }

    /// Stripe refused to finalize a licence invoice, so it was never issued.
    pub async fn send_invoice_finalization_failed_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
        reason: &str,
    ) -> Result<()> {
        self.dispatch(to, &InvoiceFinalizationFailed { plan_name, reason })
            .await
    }

    /// A sent invoice passed its due date. The licence still has its grace
    /// period, so this names the date the key stops rather than the due date.
    pub async fn send_invoice_overdue_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
        invoice: &BillingInvoice,
        po_number: Option<String>,
    ) -> Result<()> {
        let amount = format_cents(invoice.amount_due_cents, &invoice.currency);
        let due_date = invoice.due_date.map(format_timestamp).unwrap_or_default();
        let key_expires = invoice
            .provisional_paid_through()
            .map(format_timestamp)
            .unwrap_or_default();
        let cta_href = invoice
            .hosted_invoice_url
            .clone()
            .unwrap_or_else(|| links::SETTINGS_BILLING.to_string());
        self.dispatch(
            to,
            &InvoiceOverdue {
                plan_name,
                amount: &amount,
                due_date: &due_date,
                key_expires: &key_expires,
                po_number: po_number.as_deref(),
                cta_href: &cta_href,
            },
        )
        .await
    }

    pub async fn send_usage_summary_email(
        &self,
        to: EmailAddress,
        invoice: &BillingInvoice,
    ) -> Result<()> {
        // Use line item period (actual service dates), not invoice-level period
        // (which is when items were added to the invoice)
        let period = invoice
            .line_items
            .first()
            .map(|item| format_invoice_period(item.period_start, item.period_end))
            .unwrap_or_else(|| format_invoice_period(invoice.period_start, invoice.period_end));
        let invoice_date = format_timestamp(invoice.created_at);

        // `amount_paid` already reflects discounts, account credits and pause
        // credits, so this total is authoritative — but we never re-render line
        // items here, because a hand-built table won't reconcile against those
        // separate Stripe credit lines. The canonical breakdown is the invoice
        // itself: attach the PDF, falling back to the hosted link if Stripe
        // hasn't rendered the PDF yet.
        let total = format_cents(invoice.amount_paid_cents, &invoice.currency);
        let attachment = self.fetch_invoice_pdf(invoice).await;

        self.dispatch(
            to,
            &UsageSummary {
                period: &period,
                invoice_date: &invoice_date,
                total: &total,
                attachment,
                hosted_invoice_url: invoice.hosted_invoice_url.as_deref(),
            },
        )
        .await
    }

    /// Best-effort fetch of the Stripe invoice PDF for attaching. Stripe
    /// renders the PDF lazily and `invoice_pdf` is a public signed link, so we
    /// GET it directly with a short timeout; any miss (not ready, slow, error)
    /// returns `None` and the caller falls back to the hosted invoice URL.
    async fn fetch_invoice_pdf(&self, invoice: &BillingInvoice) -> Option<EmailAttachment> {
        let url = invoice.invoice_pdf.as_deref()?;
        let response = self
            .http
            .get(url)
            .timeout(Duration::from_secs(2))
            .send()
            .await
            .ok()?;
        if !response.status().is_success() {
            return None;
        }
        let bytes = response.bytes().await.ok()?;
        Some(EmailAttachment {
            filename: format!("scanopy-invoice-{}.pdf", invoice.stripe_invoice_id),
            content_type: "application/pdf".to_string(),
            bytes: bytes.to_vec(),
        })
    }

    // ========================================================================
    // Daemon emails
    // ========================================================================

    pub async fn send_daemon_standby_email(
        &self,
        to: EmailAddress,
        daemon_name: &str,
        network_name: &str,
    ) -> Result<()> {
        self.dispatch(
            to,
            &DaemonStandby {
                daemon_name,
                network_name,
            },
        )
        .await
    }

    pub async fn send_daemon_unreachable_email(
        &self,
        to: EmailAddress,
        daemon_name: &str,
        network_name: &str,
    ) -> Result<()> {
        self.dispatch(
            to,
            &DaemonUnreachable {
                daemon_name,
                network_name,
            },
        )
        .await
    }

    /// Notify a recipient that one or more of their org's daemons will lose
    /// support on `sunset_date`. `daemon_names` aggregates every affected daemon
    /// so the recipient gets one email, not one per daemon.
    pub async fn send_daemon_sunset_email(
        &self,
        to: EmailAddress,
        daemon_names: &[&str],
        sunset_date: &str,
    ) -> Result<()> {
        self.dispatch(
            to,
            &DaemonSunset {
                daemon_names,
                sunset_date,
            },
        )
        .await
    }

    pub async fn send_install_command_email(
        &self,
        to: EmailAddress,
        install_command: &str,
        os: &str,
    ) -> Result<()> {
        self.dispatch(
            to,
            &InstallCommand {
                install_command,
                os,
            },
        )
        .await
    }

    // ========================================================================
    // Digest email
    // ========================================================================

    /// Send a per-discovery-session digest. Routed through the same transport
    /// as every other email.
    pub async fn send_discovery_digest_email(
        &self,
        to: EmailAddress,
        payload: &DiscoveryDigestPayload,
    ) -> Result<()> {
        self.dispatch(
            to,
            &DiscoveryDigest {
                payload,
                base_url: &self.public_url,
            },
        )
        .await
    }

    // ========================================================================
    // Onboarding emails
    // ========================================================================

    /// Send discovery guide email (Free or Paid variant based on `is_free`)
    pub async fn send_discovery_guide_email(
        &self,
        to: EmailAddress,
        daemon_name: &str,
        network_name: &str,
    ) -> Result<()> {
        self.dispatch(
            to,
            &DiscoveryGuide {
                daemon_name,
                network_name,
            },
        )
        .await
    }

    /// Send discovery guide email for an organization after first daemon registration.
    /// Determines free/paid variant from org plan and looks up owner email.
    pub async fn send_discovery_guide_for_org(
        &self,
        org_id: Uuid,
        daemon_name: &str,
        network_name: &str,
    ) -> Result<()> {
        let organization = self
            .organization_service
            .get_by_id(&org_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Organization not found"))?;

        // A guide to discovery and topology in an app the org is locked out of
        // leads nowhere. Reachable when a daemon key made on a cloud plan is
        // first used after the switch to a self-hosted one.
        if self.self_hosted_plan_locked(organization.base.plan) {
            return Ok(());
        }

        let owner_email = self.get_owner_email(&org_id).await?;

        self.send_discovery_guide_email(owner_email, daemon_name, network_name)
            .await
    }

    // ========================================================================
    // Plan-limit checks
    // ========================================================================

    /// Check plan limits for an organization and send notification emails on threshold crossings
    pub async fn check_plan_limits(&self, org_id: Uuid, suppress_emails: bool) -> Result<()> {
        let mut org = match self.organization_service.get_by_id(&org_id).await? {
            Some(org) => org,
            None => return Ok(()),
        };

        let plan = org
            .base
            .plan
            .unwrap_or_else(crate::server::billing::plans::get_free_plan);
        // On the cloud app a self-hosted plan's seat and network limits govern
        // the customer's own server. The cloud org's rows are left over from
        // before the switch (or an invite accepted after it), and an upgrade
        // nudge about them is noise. Returns before the ratchet write, so an
        // org that moves back to a cloud plan is still warned.
        if self.self_hosted_plan_locked(org.base.plan) {
            return Ok(());
        }
        let plan_name = plan.to_string();
        let notifications = org.base.notifications.clone();
        let mut changes: Vec<LimitLevelChange> = Vec::new();

        // Check each limit type
        struct LimitCheck {
            limit: Option<u64>,
            count: u64,
            limit_type: &'static str,
            level: LimitNotificationLevel,
            has_overage: bool,
        }

        let network_filter = StorableFilter::<Network>::new_from_org_id(&org_id);
        let network_count = self.network_service.get_all(network_filter).await?.len() as u64;

        let networks = self
            .network_service
            .get_all(StorableFilter::<Network>::new_from_org_id(&org_id))
            .await?;
        let network_ids: Vec<Uuid> = networks.iter().map(|n| n.id).collect();

        // count_for_* narrow SCD2 entities to live rows so snapshot closed-copies
        // don't trip plan-limit warnings.
        let host_count = self.host_service.count_for_networks(&network_ids).await?;
        let seat_count = self.user_service.count_for_org(&org_id).await?;

        let config = plan.config();
        let checks = vec![
            LimitCheck {
                limit: plan.host_limit(),
                count: host_count,
                limit_type: "hosts",
                level: notifications.hosts.clone(),
                has_overage: config.host_cents.is_some(),
            },
            LimitCheck {
                limit: plan.network_limit(),
                count: network_count,
                limit_type: "networks",
                level: notifications.networks.clone(),
                has_overage: config.network_cents.is_some(),
            },
            LimitCheck {
                limit: plan.seat_limit(),
                count: seat_count,
                limit_type: "seats",
                level: notifications.seats.clone(),
                has_overage: config.seat_cents.is_some(),
            },
        ];

        for check in checks {
            let limit = match check.limit {
                Some(l) if l > 1 => l,
                _ => continue, // Skip unlimited or limits <= 1 (always at capacity)
            };

            let threshold_80 = (limit as f64 * 0.8) as u64;
            let new_level = if check.count >= limit {
                LimitNotificationLevel::Reached
            } else if check.count >= threshold_80 {
                LimitNotificationLevel::Approaching
            } else {
                LimitNotificationLevel::None
            };

            if new_level != check.level {
                // Any move into Approaching or Reached announces itself.
                let email =
                    (!suppress_emails && new_level != LimitNotificationLevel::None).then(|| {
                        PendingLimitEmail {
                            reached: new_level == LimitNotificationLevel::Reached,
                            count: check.count,
                            limit,
                            has_overage: check.has_overage,
                        }
                    });
                changes.push(LimitLevelChange {
                    limit_type: check.limit_type,
                    level: new_level,
                    email,
                });
            }
        }

        if changes.is_empty() {
            return Ok(());
        }

        // The owner is resolved on the first pending send. A failed lookup
        // fails every pending send, so those limits keep their old level.
        let owner_email = tokio::sync::OnceCell::new();
        let mut stored = notifications;
        let mut changed = false;
        let mut failed = 0usize;
        for change in &changes {
            let send = match &change.email {
                None => Ok(()),
                Some(pending) => match owner_email
                    .get_or_init(|| async {
                        self.get_owner_email(&org_id)
                            .await
                            .map_err(|e| format!("{e:#}"))
                    })
                    .await
                {
                    Err(e) => Err(anyhow::anyhow!("Owner email lookup failed: {e}")),
                    Ok(owner) => {
                        if pending.reached {
                            self.dispatch(
                                owner.clone(),
                                &PlanLimitReached {
                                    first_name: None,
                                    limit_type: change.limit_type,
                                    current_count: pending.count,
                                    limit: pending.limit,
                                    plan_name: &plan_name,
                                    has_overage: pending.has_overage,
                                },
                            )
                            .await
                        } else {
                            self.dispatch(
                                owner.clone(),
                                &PlanLimitApproaching {
                                    first_name: None,
                                    limit_type: change.limit_type,
                                    current_count: pending.count,
                                    limit: pending.limit,
                                    plan_name: &plan_name,
                                    has_overage: pending.has_overage,
                                },
                            )
                            .await
                        }
                    }
                },
            };
            if let Err(e) = &send {
                failed += 1;
                tracing::warn!(
                    organization_id = %org_id,
                    limit_type = change.limit_type,
                    error = %e,
                    "Failed to send plan limit email; level left unchanged for retry"
                );
            }
            changed |= change.record(&mut stored, &send);
        }

        if changed {
            org.base.notifications = stored;
            self.organization_service
                .update(&mut org, AuthenticatedEntity::System)
                .await?;
        }

        if failed > 0 {
            anyhow::bail!(
                "{failed} of {} plan limit emails failed to send",
                changes.iter().filter(|c| c.email.is_some()).count()
            );
        }
        Ok(())
    }

    // ========================================================================
    // Air-gapped licence expiry sweep (daily)
    // ========================================================================

    /// Warn every organization holding an air-gapped key whose licence period
    /// ends soon.
    ///
    /// Stripe's `invoice.upcoming` cannot do this job: it fires only for
    /// subscriptions that are *automatically charged*, so invoice-billed
    /// customers — the ones most likely to hold an air-gapped key — never
    /// produce one. Their servers never call home either, so without this
    /// sweep the first they hear of a renewal is the charge, and the one
    /// moment their plan becomes changeable passes unannounced.
    ///
    /// Each organization is warned once per licence period, tracked by the
    /// paid-through date on its notifications ratchet, so running daily is
    /// safe and a renewal re-arms it.
    pub async fn warn_expiring_airgap_keys(&self) -> Result<()> {
        let now = Utc::now();
        let filter = StorableFilter::<Organization>::new_with_airgap_key_expiring_between(
            now,
            now + chrono::Duration::days(AIRGAP_EXPIRY_WARNING_DAYS),
        );
        let expiring = self.organization_service.get_all(filter).await?;

        for organization in expiring {
            if let Err(e) = self.warn_expiring_airgap_key(organization).await {
                tracing::warn!(error = %e, "Failed to warn an org about its expiring air-gapped key");
            }
        }
        Ok(())
    }

    /// One organization's warning, plus the ratchet write that stops it
    /// repeating for the same licence period.
    async fn warn_expiring_airgap_key(&self, mut organization: Organization) -> Result<()> {
        let (Some(plan), Some(paid_through)) = (
            organization.base.plan,
            organization.base.license_paid_through,
        ) else {
            return Ok(());
        };
        if plan.license_plan().is_none() {
            return Ok(());
        }
        if organization
            .base
            .notifications
            .airgap_expiry_notified_through
            == Some(paid_through)
        {
            return Ok(());
        }

        let owner = self.get_owner_email(&organization.id).await?;
        self.send_airgap_expiring_email(
            owner,
            plan.name(),
            &format_timestamp(paid_through),
            &format_timestamp(paid_through),
            &format_cents(plan.config().base_cents, "usd"),
        )
        .await?;

        organization
            .base
            .notifications
            .airgap_expiry_notified_through = Some(paid_through);
        self.organization_service
            .update(&mut organization, AuthenticatedEntity::System)
            .await?;

        tracing::info!(
            organization_id = %organization.id,
            paid_through = %paid_through,
            "Warned an air-gapped organization that its license key expires soon"
        );
        Ok(())
    }

    // ========================================================================
    // Daemon sunset sweep (boot-time)
    // ========================================================================

    /// Email each organization whose active daemons face an announced sunset
    /// cutover. Each daemon is matched to the cutover it actually faces — the
    /// soonest one whose floor is above it — and daemons are grouped per (org,
    /// floor), so one aggregated email per group (naming every affected daemon)
    /// goes to the org owner plus each distinct daemon maintainer. An org-level
    /// ratchet on the highest floor already announced to that org means each org
    /// is emailed at most once per floor, so this is safe to run on every server
    /// boot — including after a new cutover is appended to the list.
    ///
    /// No-op while the sunset machinery is dormant (no cutover has a date yet).
    pub async fn announce_daemon_sunsets(&self) -> Result<()> {
        use crate::server::daemons::r#impl::version::applicable_sunset;

        // Nothing announced at all — skip the sweep entirely rather than query
        // every daemon to learn that. A version-less daemon counts as below every
        // floor, so "not even that faces a cutover" means no cutover has a date.
        if applicable_sunset(None).is_none() {
            return Ok(());
        }

        // Active daemons across all orgs. A daemon that reports no version is
        // treated as genuinely old (modern daemons always report one) and is
        // included — as long as it has actually connected (has a last_seen); a
        // provisioned-but-never-connected daemon has no version yet for the
        // benign reason that it hasn't checked in, so it is skipped rather than
        // emailed about.
        let daemons = self
            .daemon_service
            .get_all(StorableFilter::<Daemon>::new_for_active_daemons())
            .await?;

        let mut by_org_floor: HashMap<(Uuid, Version), (DateTime<Utc>, Vec<Daemon>)> =
            HashMap::new();
        for daemon in daemons {
            let version = daemon.base.version.as_ref();
            if version.is_none() && daemon.base.last_seen.is_none() {
                continue;
            }
            // Dormant cutovers yield None here, which is what makes the whole
            // sweep a no-op before a launch date is baked in.
            let Some((floor, effective_on)) = applicable_sunset(version) else {
                continue;
            };
            let Some(network) = self
                .network_service
                .get_by_id(&daemon.base.network_id)
                .await?
            else {
                continue;
            };
            by_org_floor
                .entry((network.base.organization_id, floor))
                .or_insert_with(|| (effective_on, Vec::new()))
                .1
                .push(daemon);
        }

        // Ascending floor order. Once a group fails, that org's higher floors
        // are skipped this sweep: emailing them would raise the ratchet past
        // the failed floor and it would never be retried. The ratchet stays at
        // the highest floor every recipient received, and the next boot
        // retries from there.
        let mut groups: Vec<_> = by_org_floor.into_iter().collect();
        groups.sort_by(|((_, a_floor), _), ((_, b_floor), _)| a_floor.cmp(b_floor));

        let mut failed_orgs: HashSet<Uuid> = HashSet::new();
        for ((org_id, floor), (effective_on, affected)) in groups {
            if failed_orgs.contains(&org_id) {
                continue;
            }
            let sunset_display = effective_on.format("%B %-d, %Y").to_string();
            if let Err(e) = self
                .announce_org_sunset(org_id, &affected, &floor, &sunset_display)
                .await
            {
                failed_orgs.insert(org_id);
                tracing::warn!(org_id = %org_id, error = %e, "Failed to send daemon sunset notification");
            }
        }

        Ok(())
    }

    /// Send the aggregated sunset email to one org's owner + maintainers, then
    /// advance the org's ratchet. Skips orgs already notified for this floor.
    async fn announce_org_sunset(
        &self,
        org_id: Uuid,
        affected: &[Daemon],
        floor: &Version,
        sunset_display: &str,
    ) -> Result<()> {
        let mut org = match self.organization_service.get_by_id(&org_id).await? {
            Some(o) => o,
            None => return Ok(()),
        };
        // Daemons left on the cloud org after a move to a self-hosted plan are
        // idle: the org is locked out of the cloud app and its scans run on
        // its own server. Skipped without advancing the ratchet, so an org
        // that returns to a cloud plan is told on the next boot.
        if self.self_hosted_plan_locked(org.base.plan) {
            return Ok(());
        }
        // Ratchet: the stored value is the highest floor this org has been told
        // about, and floors are totally ordered, so anything at or below it has
        // already been communicated.
        if org
            .base
            .notifications
            .sunset_notified_floor
            .as_ref()
            .is_some_and(|notified| floor <= notified)
        {
            return Ok(());
        }

        let daemon_names: Vec<&str> = affected.iter().map(|d| d.base.name.as_str()).collect();

        // Recipients: org owner + each distinct daemon maintainer (dedup by user
        // id so a maintainer of several affected daemons is emailed once).
        let owners = self.user_service.get_organization_owners(&org_id).await?;
        let owner = owners.first();
        let mut seen_users: HashSet<Uuid> = HashSet::new();
        let mut recipients: Vec<EmailAddress> = Vec::new();
        if let Some(owner) = owner {
            seen_users.insert(owner.id);
            recipients.push(owner.base.email.clone());
        }
        for daemon in affected {
            if seen_users.insert(daemon.base.user_id)
                && let Some(user) = self.user_service.get_by_id(&daemon.base.user_id).await?
            {
                recipients.push(user.base.email);
            }
        }

        let mut failed = 0usize;
        for to in &recipients {
            if let Err(e) = self
                .send_daemon_sunset_email(to.clone(), &daemon_names, sunset_display)
                .await
            {
                failed += 1;
                tracing::warn!(org_id = %org_id, error = %e, "Failed to dispatch daemon sunset email");
            }
        }

        // Any failed recipient leaves the ratchet where it was, so the next
        // boot sweep retries this floor. Recipients who already received it
        // get it again then.
        if failed > 0 {
            anyhow::bail!(
                "{failed} of {} daemon sunset emails failed to send; floor {floor} not recorded",
                recipients.len()
            );
        }

        // Every recipient was emailed: advance the ratchet so subsequent boots
        // don't re-send for this floor (or any lower one). Monotonic: it only
        // ever moves up.
        org.base.notifications.sunset_notified_floor = Some(floor.clone());
        self.organization_service
            .update(&mut org, AuthenticatedEntity::System)
            .await?;
        Ok(())
    }

    /// Counts of entities discovered/created during the trial, plus how many
    /// days have elapsed since org creation. Used to populate the trial-ending
    /// email and the in-app trial value recap card.
    async fn compute_trial_recap_metrics(&self, org_id: Uuid) -> Result<TrialRecapMetrics> {
        let org = self
            .organization_service
            .get_by_id(&org_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Organization not found"))?;

        let networks = self
            .network_service
            .get_all(StorableFilter::<Network>::new_from_org_id(&org_id))
            .await?;
        let networks_count = networks.len() as u64;
        let network_ids: Vec<Uuid> = networks.iter().map(|n| n.id).collect();

        // count_for_networks narrows SCD2 entities to live rows so snapshot
        // closed-copies don't inflate digest counts.
        let hosts_count = self.host_service.count_for_networks(&network_ids).await?;

        let services_count = self
            .service_service
            .count_for_networks(&network_ids)
            .await?;

        let daemons_count = self.daemon_service.count_for_networks(&network_ids).await?;

        let days_into_trial = (chrono::Utc::now() - org.created_at).num_days();

        Ok(TrialRecapMetrics {
            hosts_count,
            networks_count,
            daemons_count,
            services_count,
            days_into_trial,
        })
    }

    /// Get the owner email for an organization
    pub async fn get_owner_email(&self, org_id: &Uuid) -> Result<EmailAddress> {
        let owners = self.user_service.get_organization_owners(org_id).await?;
        let owner = owners
            .first()
            .ok_or_else(|| anyhow::anyhow!("No owner found for organization {}", org_id))?;
        Ok(owner.base.email.clone())
    }
}

/// Format a plan's base price for display in emails (e.g. "$14.99/mo")
pub fn format_plan_price(plan: &BillingPlan) -> String {
    let config = plan.config();
    let amount = format_cents(config.base_cents, "usd");
    match config.rate {
        BillingRate::Month => format!("{}/mo", amount),
        BillingRate::Year => format!("{}/yr", amount),
    }
}

/// Format an amount in cents to a display string (e.g. 2999 → "$29.99")
pub fn format_cents(amount: i64, currency: &str) -> String {
    let dollars = amount as f64 / 100.0;
    match currency {
        "usd" => format!("${:.2}", dollars),
        _ => format!("{:.2} {}", dollars, currency.to_uppercase()),
    }
}

/// Format a chrono timestamp into "February 22, 2026"
fn format_timestamp(dt: chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%B %-d, %Y").to_string()
}

/// Format invoice period timestamps into a human-readable range
fn format_invoice_period(
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
) -> String {
    format!(
        "{} – {}",
        start.format("%b %-d, %Y"),
        end.format("%b %-d, %Y")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change(limit_type: &'static str, level: LimitNotificationLevel) -> LimitLevelChange {
        LimitLevelChange {
            limit_type,
            level,
            email: None,
        }
    }

    #[test]
    fn limit_level_advances_only_when_its_send_succeeded() {
        let mut notifications = OrgNotifications::default();
        let sent = change("hosts", LimitNotificationLevel::Reached);
        let failed = change("networks", LimitNotificationLevel::Approaching);

        assert!(sent.record(&mut notifications, &Ok(())));
        assert!(!failed.record(&mut notifications, &Err(anyhow::anyhow!("smtp down"))));

        assert_eq!(notifications.hosts, LimitNotificationLevel::Reached);
        // The failed limit keeps its old level, so the next crossing retries.
        assert_eq!(notifications.networks, LimitNotificationLevel::None);
        assert_eq!(notifications.seats, LimitNotificationLevel::None);
    }
}
