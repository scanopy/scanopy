use crate::server::auth::r#impl::oidc::OidcProviderMetadata;
use crate::server::github::latest_release::{self, AvailableUpgrade};
use crate::server::license::key::LicenseKey;
use crate::server::license::mint::LicenseIssuer;
use crate::server::license::online::InstanceUsage;
use crate::server::license::service::{LicenseService, self_hosted_plan};
use crate::server::license::types::LicenseKeyType;
use crate::server::license::types::LicenseStatusDiscriminants;
use crate::server::openapi::tags as api_tags;
use crate::server::shared::handlers::cache::AppCache;
use crate::server::shared::types::api::ApiResponse;
use crate::server::{
    auth::r#impl::oidc::OidcProviderConfig, shared::services::factory::ServiceFactory,
};
use anyhow::{Error, Result};
use axum::extract::State;
use axum::http::header::CACHE_CONTROL;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use chrono::{DateTime, Utc};
use clap::{CommandFactory, Parser};
use email_address::EmailAddress;
use figment::{
    Figment,
    providers::{Env, Format, Serialized, Toml},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tower_sessions::SessionManagerLayer;
use tower_sessions_sqlx_store::PostgresStore;
use utoipa::ToSchema;

use crate::server::shared::env_file::apply_file_env_vars;
use crate::server::shared::storage::factory::StorageFactory;
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;

/// Prefix figment reads `ServerConfig` fields from: `SCANOPY_SERVER_PORT` sets `server_port`.
const ENV_PREFIX: &str = "SCANOPY_";

/// One row of the server configuration docs table. See [`ServerConfig::docs_fields`].
#[derive(Debug, Clone, Serialize)]
pub struct ServerConfigField {
    pub id: String,
    pub cli_flag: Option<String>,
    pub env_var: String,
    pub default: Option<String>,
    pub description: String,
}

// Every user-settable `ServerConfig` field has a flag here, and the doc comment on each flag is the
// description the docs render (`ServerConfig::docs_fields`). Cloud-only flags are hidden, which also
// keeps them out of the docs. (A `///` comment here would become clap's long_about.)
#[derive(Parser)]
#[command(name = "scanopy-server")]
#[command(about = "Scanopy server")]
pub struct ServerCli {
    /// Port the server listens on.
    #[arg(long)]
    server_port: Option<u16>,

    /// Logging verbosity: trace, debug, info, warn or error.
    #[arg(long)]
    log_level: Option<String>,

    /// Log filter for dependencies, in RUST_LOG syntax. Empty uses the log level for everything.
    #[arg(long)]
    rust_log: Option<String>,

    /// PostgreSQL connection string.
    #[arg(long)]
    database_url: Option<String>,

    /// URL the server uses to reach the integrated daemon. The published Docker Compose file sets
    /// it to http://host.docker.internal:60073.
    #[arg(long)]
    integrated_daemon_url: Option<String>,

    /// Mark session cookies Secure, so browsers send them over HTTPS only. Set it when the UI is
    /// served over HTTPS.
    #[arg(long)]
    use_secure_session_cookies: Option<bool>,

    /// Turn off sign-up for new users.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    disable_registration: Option<bool>,

    /// Turn off email and password login, leaving OIDC as the only way to sign in.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    disable_password_login: Option<bool>,

    /// Stop checking GitHub for new Scanopy releases. The check is an anonymous request to
    /// api.github.com twice a day; with it off, admins see no banner when an upgrade is out.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    disable_update_check: Option<bool>,

    /// Stripe publishable key (exposed to the frontend for Stripe Elements)
    #[arg(long, hide = true)]
    stripe_key: Option<String>,

    /// Stripe secret key
    #[arg(long, hide = true)]
    stripe_secret: Option<String>,

    /// Stripe webhook signing secret
    #[arg(long, hide = true)]
    stripe_webhook_secret: Option<String>,

    /// Username for the SMTP relay.
    #[arg(long)]
    smtp_username: Option<String>,

    /// Password for the SMTP relay.
    #[arg(long)]
    smtp_password: Option<String>,

    /// Sender address for email Scanopy sends over SMTP.
    #[arg(long)]
    smtp_email: Option<String>,

    /// SMTP relay host, for example smtp.gmail.com.
    #[arg(long)]
    smtp_relay: Option<String>,

    /// SMTP relay port. 465 uses implicit TLS; any other port uses STARTTLS.
    #[arg(long)]
    smtp_port: Option<u16>,

    /// Most emails this server sends per second. Set it below your relay's sending limit; Amazon
    /// SES production accounts start at 14 per second. Release announcements use half of it.
    #[arg(long)]
    smtp_max_per_second: Option<u32>,

    /// Hold the release email for the version this server runs. The email goes out on the first
    /// start without this set. The release notes still show in the app.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    hold_release_email: Option<bool>,

    /// ARN of the SNS topic that delivers SES bounce and complaint notifications.
    #[arg(long)]
    email_feedback_sns_topic_arn: Option<String>,

    /// AWS region of the Amazon SES account. Signup validation calls SES in this region, and the
    /// SNS topic ARN and an SES SMTP relay are checked against it.
    #[arg(long)]
    aws_region: Option<String>,

    /// Access key id of the IAM user that signup validation calls SES as, allowed only
    /// `ses:GetEmailAddressInsights`. Not the SES SMTP username.
    #[arg(long, hide = true)]
    email_validation_aws_access_key_id: Option<String>,

    /// IAM secret access key for `email_validation_aws_access_key_id`. Not the SES SMTP password.
    #[arg(long, hide = true)]
    email_validation_aws_secret_access_key: Option<String>,

    /// URL users reach the server at. Used in password reset links, invite links and webhooks.
    #[arg(long)]
    public_url: Option<String>,

    /// Where the server reads the client IP from when it sits behind a reverse proxy, for
    /// accurate IPs in auth event logs. Unset uses the connecting address.
    #[arg(long)]
    pub client_ip_source: Option<String>,

    /// OIDC providers users can sign in with.
    #[arg(long)]
    pub oidc_providers: Option<String>,

    #[arg(long, hide = true)]
    pub posthog_key: Option<String>,

    /// Bearer token the Prometheus metrics endpoint requires.
    #[arg(long)]
    pub metrics_token: Option<String>,

    /// PEM file of extra CA certificates to trust for outbound TLS, in addition to the bundled
    /// roots.
    #[arg(long)]
    pub trusted_ca_bundle: Option<PathBuf>,

    /// Serve the web UI from this directory instead of the copy compiled into the binary. The
    /// Docker image sets it to /app/static. If the directory has no readable index.html, the
    /// server logs a warning and serves the compiled-in UI.
    #[arg(long)]
    pub web_external_path: Option<PathBuf>,

    /// License key for the commercial edition. Omit it for the community edition.
    #[arg(long)]
    pub license_key: Option<String>,

    /// Mark this server as non-production (staging, testing or disaster recovery). It takes a
    /// non-production slot on the license key and shows a banner saying so.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    pub non_production: Option<bool>,

    /// Email address shown to users who can't create an organization because the server has
    /// reached its organization limit.
    #[arg(long)]
    pub server_admin_contact_email: Option<String>,

    /// Days to keep topology snapshots before pruning them. Unset uses the plan's retention
    /// window.
    #[arg(long)]
    pub snapshot_retention_days_override: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub server_port: u16,
    pub log_level: String,
    pub rust_log: String,
    pub database_url: String,
    pub web_external_path: Option<PathBuf>,
    pub public_url: String,
    pub integrated_daemon_url: Option<String>,
    pub use_secure_session_cookies: bool,
    pub disable_registration: bool,
    pub disable_password_login: bool,
    /// `SCANOPY_DISABLE_UPDATE_CHECK`: no GitHub release check, no upgrade banner.
    #[serde(default)]
    pub disable_update_check: bool,
    pub client_ip_source: Option<String>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_relay: Option<String>,
    pub smtp_email: Option<String>,
    /// SMTP port, 465 (implicit TLS) by default. Ports other than 465 use
    /// STARTTLS (e.g. 587, 25).
    pub smtp_port: Option<u16>,
    /// Emails sent per second by this process, across every email. Bulk sends
    /// take half (`EmailService::bulk_limiter`), so other emails keep the rest.
    pub smtp_max_per_second: u32,
    /// Hold the release email until a start without this set
    /// (`ReleaseNotesService::announce_running_version`).
    pub hold_release_email: bool,
    /// Directory the logging email transport writes rendered emails to
    /// (`SCANOPY_EMAIL_LOG_DIR`). Setting it selects that transport ahead of
    /// SMTP, so nothing is actually delivered — it exists to
    /// exercise email paths locally without credentials.
    ///
    /// Testing only: the written files contain full bodies, including
    /// password-reset and verification tokens, in plaintext on disk.
    #[serde(default)]
    pub email_log_dir: Option<PathBuf>,
    /// ARN of the SNS topic that delivers SES bounce and complaint notifications to
    /// `POST /api/email/feedback/sns` (`SCANOPY_EMAIL_FEEDBACK_SNS_TOPIC_ARN`). Unset, that
    /// endpoint answers 404. Messages from any other topic are rejected.
    #[serde(default)]
    pub email_feedback_sns_topic_arn: Option<String>,
    /// Region of the Amazon SES account (`SCANOPY_AWS_REGION`). Signup validation calls SES
    /// here; the SNS topic ARN and an SES SMTP relay are checked against it at startup.
    #[serde(default)]
    pub aws_region: Option<String>,
    /// Keys of the IAM user that signup validation calls `GetEmailAddressInsights` as
    /// (`SCANOPY_EMAIL_VALIDATION_AWS_*`), scoped to that one action and distinct from the SES
    /// SMTP credentials. Validation runs only when `aws_region` and both keys are set.
    #[serde(default)]
    pub email_validation_aws_access_key_id: Option<String>,
    #[serde(default)]
    pub email_validation_aws_secret_access_key: Option<String>,
    #[serde(default)]
    pub oidc_providers: Option<Vec<OidcProviderConfig>>,

    // Used in SaaS deployment
    pub stripe_key: Option<String>,
    pub stripe_secret: Option<String>,
    pub stripe_webhook_secret: Option<String>,
    pub posthog_key: Option<String>,

    /// Enforce plan limits on a self-hosted server, from
    /// `SCANOPY_ENFORCE_PLAN_LIMITS_FOR_TESTING` (or the older
    /// `SCANOPY_ENFORCE_BILLING_FOR_TESTING`). The integration stack sets it so
    /// the billing middleware runs; the server stays self-hosted otherwise.
    #[serde(default)]
    pub enforce_plan_limits_for_testing: bool,

    // Metrics
    pub metrics_token: Option<String>,

    // External service IP restrictions
    // Maps service name (lowercase) to list of allowed IPs/CIDRs
    // Populated from SCANOPY_EXTERNAL_SERVICE_<NAME>_ALLOWED_IPS env vars
    #[serde(default)]
    pub external_service_allowed_ips: HashMap<String, Vec<String>>,

    // License key for commercial self-hosted deployments
    pub license_key: Option<String>,

    /// Whether this server is non-production, from `SCANOPY_NON_PRODUCTION`.
    /// Sent with each license check-in, so the server takes a non-production
    /// slot on the key.
    #[serde(default)]
    pub non_production: bool,

    /// Ed25519 PEM private key the cloud signs license keys and entitlements
    /// with, from `SCANOPY_LICENSE_SIGNING_KEY`. Cloud only; never set on
    /// customer servers.
    pub license_signing_key: Option<String>,

    /// Where an online license key fetches its entitlement, from
    /// `SCANOPY_LICENSE_SERVER_URL`. Defaults to Scanopy Cloud
    /// (`CLOUD_BASE_URL`); set it to a local server to test check-ins.
    #[serde(default)]
    pub license_server_url: Option<String>,

    /// Admin contact email shown to users who are blocked from creating a new
    /// organization on a self-hosted instance at its org cap. Populated from
    /// `SCANOPY_SERVER_ADMIN_CONTACT_EMAIL`; a malformed value fails config load.
    #[serde(default)]
    pub server_admin_contact_email: Option<EmailAddress>,

    /// Override the snapshot retention window for self-hosted / community /
    /// enterprise / demo plans. Default 90 days when unset. Ignored for the
    /// SaaS-tier plans (Free / Starter / Pro / Business / Team) which carry
    /// fixed per-tier retention values. Picks up `SCANOPY_SNAPSHOT_RETENTION_DAYS_OVERRIDE`
    /// from the env automatically via the existing Figment env-var pipeline.
    #[serde(default)]
    pub snapshot_retention_days_override: Option<u32>,

    /// PEM file (`SCANOPY_TRUSTED_CA_BUNDLE`) whose CA certificates are trusted in addition to
    /// the bundled roots by every outbound client that reaches an operator-controlled host
    /// (OIDC issuers, SMTP relay, daemons, license server). A set but missing, unreadable or
    /// certificate-free file stops startup.
    #[serde(default)]
    pub trusted_ca_bundle: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentType {
    Cloud,
    Commercial,
    Community,
}

impl DeploymentType {
    /// True for deployments the customer hosts themselves (Commercial /
    /// Community). Cloud is the only Scanopy-LLC-operated deployment, so this
    /// is the "is Scanopy LLC the sender" signal the email footer gates on.
    pub fn is_self_hosted(&self) -> bool {
        !matches!(self, DeploymentType::Cloud)
    }

    /// Whether users accept the terms at sign-up and again when they change.
    /// Only Scanopy Cloud collects acceptance; a self-hosted server has
    /// nothing to gate on.
    pub fn requires_terms_acceptance(&self) -> bool {
        *self == DeploymentType::Cloud
    }

    /// Whether the license plan's `included_orgs` caps how many orgs can be
    /// created. Cloud is multi-tenant and never caps.
    pub fn caps_orgs_by_license(&self) -> bool {
        self.is_self_hosted()
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PublicConfigResponse {
    /// Port this server listens on.
    pub server_port: u16,
    /// Whether self-service sign-up is turned off on this deployment.
    pub disable_registration: bool,
    /// Whether email/password login is turned off, leaving OIDC as the only method.
    pub disable_password_login: bool,
    /// Identity providers available on the login screen.
    pub oidc_providers: Vec<OidcProviderMetadata>,
    /// Stripe publishable key, exposed so the frontend can mount Stripe
    /// Elements (Payment Element) for in-app card collection. `None` when
    /// billing isn't configured. Publishable keys are safe to expose to the
    /// browser (same as `posthog_key`).
    pub stripe_publishable_key: Option<String>,
    /// `STRIPE_SAVE_OFFER_COUPON_ID` env var is set. When false, the
    /// cancel modal hides the discount save-offer panel so the user
    /// doesn't see an option the deployment can't fulfil.
    pub discount_save_offer_available: bool,
    /// Whether a daemon runs alongside the server, so no separate install is needed to start scanning.
    pub has_integrated_daemon: bool,
    /// Whether outbound email is configured. Invites and password resets need it.
    pub has_email_service: bool,
    /// Base URL this server is reachable at, as configured by the operator.
    #[schema(format = "uri")]
    pub public_url: String,
    /// Public analytics key, when analytics is enabled.
    pub posthog_key: Option<String>,
    /// Whether the client should show a cookie-consent prompt.
    pub needs_cookie_consent: bool,
    /// How this instance is run: cloud, commercial self-hosted, or community.
    pub deployment_type: DeploymentType,
    /// Runtime state of the configured license key. `None` on deployments
    /// that don't require one (community and cloud).
    pub license_status: Option<LicenseStatusDiscriminants>,
    /// Whether the configured key is an offline key or an online key that
    /// fetches its entitlement from Scanopy Cloud. `None` when no key applies.
    pub license_key_type: Option<LicenseKeyType>,
    /// When Scanopy Cloud last answered this instance's license check-in.
    /// Online keys only.
    pub license_entitlement_at: Option<DateTime<Utc>>,
    /// How many server instances this instance's online key is running on,
    /// against how many it covers, as Scanopy Cloud reported at the last
    /// check-in. Online keys only.
    pub license_instances: Option<InstanceUsage>,
    /// Whether this server was started with `SCANOPY_NON_PRODUCTION`.
    pub non_production: bool,
    /// Hard expiry — the drop-dead date after which the server rejects
    /// the key. Referenced by the grace-period banner.
    #[schema(format = "date")]
    pub license_expiry: Option<String>,
    /// User-visible expiry — the date displayed to end users under
    /// normal operation. 7 days earlier than `license_expiry` for keys
    /// issued after grace-period support landed.
    #[schema(format = "date")]
    pub license_intended_expiry: Option<String>,
    /// End of the paid period, shown as "Valid through" and in the expiry
    /// banners. Falls back to the intended expiry for keys minted without a
    /// paid-through date.
    pub license_valid_through: Option<DateTime<Utc>>,
    /// True when the license is past `intended_exp` but not yet past
    /// the hard `exp` — the silent grace window.
    pub license_in_grace_period: bool,
    /// `SCANOPY_SNAPSHOT_RETENTION_DAYS_OVERRIDE` if set on this instance.
    /// Frontend uses it inside the plan-comparison view to display the
    /// effective retention for this deployment rather than the per-plan
    /// fixture default.
    pub snapshot_retention_days_override: Option<u32>,
    /// True when this self-hosted instance has reached its licensed
    /// organization cap (`included_orgs`), so new-org registration is blocked.
    /// Always false on cloud (multi-tenant) and on unlimited-org plans.
    pub org_limit_reached: bool,
    /// Admin contact email to show users blocked by `org_limit_reached`,
    /// from `SCANOPY_SERVER_ADMIN_CONTACT_EMAIL`.
    #[schema(value_type = String, format = "email")]
    pub server_admin_contact_email: Option<EmailAddress>,
    /// Whether this deployment can sign license keys. False on any server
    /// without a signing key, where the Settings License tab and the
    /// self-hosted plans would otherwise offer something the mint path
    /// refuses. Reads the built issuer rather than the config value, since a
    /// key can be present but unparseable.
    pub license_signing_available: bool,
    /// Days past an organization's paid-through date before a license key
    /// reaches its user-visible expiry. Published so the UI shows the same
    /// dates the mint path bakes into keys, instead of its own copy.
    pub license_key_buffer_days: u32,
    /// Further days past the user-visible expiry before a key stops working.
    pub license_key_grace_days: u32,
    /// The newest promoted release, when it is newer than this server. `None`
    /// on cloud, with `SCANOPY_DISABLE_UPDATE_CHECK` set, before the first
    /// check answers, or when this server is current.
    pub available_upgrade: Option<AvailableUpgrade>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server_port: 60072,
            log_level: "info".to_string(),
            rust_log: "".to_string(),
            database_url: "postgresql://postgres:password@localhost:5432/scanopy".to_string(),
            public_url: "http://localhost:60072".to_string(),
            web_external_path: None,
            use_secure_session_cookies: false,
            integrated_daemon_url: None,
            disable_registration: false,
            disable_password_login: false,
            disable_update_check: false,
            stripe_key: None,
            stripe_secret: None,
            stripe_webhook_secret: None,
            smtp_username: None,
            smtp_password: None,
            smtp_email: None,
            smtp_relay: None,
            smtp_port: Some(465),
            smtp_max_per_second: 10,
            hold_release_email: false,
            email_log_dir: None,
            email_feedback_sns_topic_arn: None,
            aws_region: None,
            email_validation_aws_access_key_id: None,
            email_validation_aws_secret_access_key: None,
            client_ip_source: None,
            oidc_providers: None,
            posthog_key: None,
            enforce_plan_limits_for_testing: false,
            metrics_token: None,
            external_service_allowed_ips: HashMap::new(),
            license_key: None,
            non_production: false,
            license_signing_key: None,
            license_server_url: None,
            server_admin_contact_email: None,
            snapshot_retention_days_override: None,
            trusted_ca_bundle: None,
        }
    }
}

impl ServerConfig {
    /// Whether the billing middleware enforces plan limits: always on cloud,
    /// and on a self-hosted server that turned it on for testing.
    pub fn enforces_plan_limits(&self) -> bool {
        get_deployment_type(self) == DeploymentType::Cloud || self.enforce_plan_limits_for_testing
    }

    pub fn load(cli_args: ServerCli) -> anyhow::Result<Self> {
        apply_file_env_vars::<ServerConfig>()?;

        // Standard configuration layering: Defaults → Env → CLI (highest priority)
        let mut figment = Figment::from(Serialized::defaults(ServerConfig::default()))
            .merge(Toml::file("../oidc.toml"))
            .merge(
                Env::prefixed(ENV_PREFIX)
                    // Test stacks written before the rename set SCANOPY_ENFORCE_BILLING_FOR_TESTING.
                    .map(|key| {
                        if key == "enforce_billing_for_testing" {
                            "enforce_plan_limits_for_testing".into()
                        } else {
                            key.into()
                        }
                    }),
            );

        // Add CLI overrides (highest priority) - only if explicitly provided
        if let Some(server_port) = cli_args.server_port {
            figment = figment.merge(("server_port", server_port));
        }
        if let Some(log_level) = cli_args.log_level {
            figment = figment.merge(("log_level", log_level));
        }
        if let Some(rust_log) = cli_args.rust_log {
            figment = figment.merge(("rust_log", rust_log));
        }
        if let Some(database_url) = cli_args.database_url {
            figment = figment.merge(("database_url", database_url));
        }
        if let Some(integrated_daemon_url) = cli_args.integrated_daemon_url {
            figment = figment.merge(("integrated_daemon_url", integrated_daemon_url));
        }
        if let Some(use_secure_session_cookies) = cli_args.use_secure_session_cookies {
            figment = figment.merge(("use_secure_session_cookies", use_secure_session_cookies));
        }
        if let Some(stripe_key) = cli_args.stripe_key {
            figment = figment.merge(("stripe_key", stripe_key));
        }
        if let Some(stripe_secret) = cli_args.stripe_secret {
            figment = figment.merge(("stripe_secret", stripe_secret));
        }
        if let Some(stripe_webhook_secret) = cli_args.stripe_webhook_secret {
            figment = figment.merge(("stripe_webhook_secret", stripe_webhook_secret));
        }
        if let Some(smtp_username) = cli_args.smtp_username {
            figment = figment.merge(("smtp_username", smtp_username));
        }
        if let Some(smtp_password) = cli_args.smtp_password {
            figment = figment.merge(("smtp_password", smtp_password));
        }
        if let Some(smtp_relay) = cli_args.smtp_relay {
            figment = figment.merge(("smtp_relay", smtp_relay));
        }
        if let Some(smtp_email) = cli_args.smtp_email {
            figment = figment.merge(("smtp_email", smtp_email));
        }
        if let Some(smtp_port) = cli_args.smtp_port {
            figment = figment.merge(("smtp_port", smtp_port));
        }
        if let Some(smtp_max_per_second) = cli_args.smtp_max_per_second {
            figment = figment.merge(("smtp_max_per_second", smtp_max_per_second));
        }
        if let Some(hold_release_email) = cli_args.hold_release_email {
            figment = figment.merge(("hold_release_email", hold_release_email));
        }
        if let Some(topic_arn) = cli_args.email_feedback_sns_topic_arn {
            figment = figment.merge(("email_feedback_sns_topic_arn", topic_arn));
        }
        if let Some(region) = cli_args.aws_region {
            figment = figment.merge(("aws_region", region));
        }
        if let Some(key_id) = cli_args.email_validation_aws_access_key_id {
            figment = figment.merge(("email_validation_aws_access_key_id", key_id));
        }
        if let Some(secret) = cli_args.email_validation_aws_secret_access_key {
            figment = figment.merge(("email_validation_aws_secret_access_key", secret));
        }
        if let Some(public_url) = cli_args.public_url {
            figment = figment.merge(("public_url", public_url));
        }
        if let Some(client_ip_source) = cli_args.client_ip_source {
            figment = figment.merge(("client_ip_source", client_ip_source));
        }
        if let Some(oidc_providers) = cli_args.oidc_providers {
            figment = figment.merge(("oidc_providers", oidc_providers));
        }
        if let Some(posthog_key) = cli_args.posthog_key {
            figment = figment.merge(("posthog_key", posthog_key));
        }
        if let Some(disable_registration) = cli_args.disable_registration {
            figment = figment.merge(("disable_registration", disable_registration));
        }
        if let Some(disable_password_login) = cli_args.disable_password_login {
            figment = figment.merge(("disable_password_login", disable_password_login));
        }
        if let Some(disable_update_check) = cli_args.disable_update_check {
            figment = figment.merge(("disable_update_check", disable_update_check));
        }
        if let Some(metrics_token) = cli_args.metrics_token {
            figment = figment.merge(("metrics_token", metrics_token));
        }
        if let Some(trusted_ca_bundle) = cli_args.trusted_ca_bundle {
            figment = figment.merge(("trusted_ca_bundle", trusted_ca_bundle));
        }
        if let Some(web_external_path) = cli_args.web_external_path {
            figment = figment.merge(("web_external_path", web_external_path));
        }
        if let Some(license_key) = cli_args.license_key {
            figment = figment.merge(("license_key", license_key));
        }
        if let Some(non_production) = cli_args.non_production {
            figment = figment.merge(("non_production", non_production));
        }
        if let Some(server_admin_contact_email) = cli_args.server_admin_contact_email {
            figment = figment.merge(("server_admin_contact_email", server_admin_contact_email));
        }
        if let Some(snapshot_retention_days_override) = cli_args.snapshot_retention_days_override {
            figment = figment.merge((
                "snapshot_retention_days_override",
                snapshot_retention_days_override,
            ));
        }

        let mut config: ServerConfig = figment
            .extract()
            .map_err(|e| Error::msg(format!("Configuration error: {}", e)))?;

        // Parse external service IP restrictions from env vars
        // Format: SCANOPY_EXTERNAL_SERVICE_<NAME>_ALLOWED_IPS=192.168.1.0/24,10.0.0.1
        config.external_service_allowed_ips = Self::load_external_service_allowed_ips();

        Ok(config)
    }

    /// Settings whose embedded AWS region differs from `aws_region`, as (environment variable,
    /// region it names). Empty when `aws_region` is unset. SES publishes feedback only to a topic
    /// in its own region, so a mismatched topic never receives any. An SMTP relay that is not an
    /// SES endpoint is not checked.
    pub fn aws_region_mismatches(&self) -> Vec<(&'static str, String)> {
        let Some(expected) = self.aws_region.as_deref().filter(|r| !r.is_empty()) else {
            return Vec::new();
        };
        let topic_region = self
            .email_feedback_sns_topic_arn
            .as_deref()
            .and_then(|arn| arn.split(':').nth(3));
        let relay_region = self.smtp_relay.as_deref().and_then(|relay| {
            relay
                .strip_prefix("email-smtp.")?
                .strip_suffix(".amazonaws.com")
        });
        [
            ("SCANOPY_EMAIL_FEEDBACK_SNS_TOPIC_ARN", topic_region),
            ("SCANOPY_SMTP_RELAY", relay_region),
        ]
        .into_iter()
        .filter_map(|(var, region)| {
            region
                .filter(|r| *r != expected)
                .map(|r| (var, r.to_owned()))
        })
        .collect()
    }

    /// Every user-settable parameter, for the server configuration docs (`server-config.json`).
    ///
    /// Read from the code the server itself runs: the flag and description from clap, the
    /// environment variable from the `SCANOPY_` prefix `load` hands figment, and the default from
    /// `ServerConfig::default()`. Hidden (cloud-only) flags are left out.
    pub fn docs_fields() -> Vec<ServerConfigField> {
        let defaults =
            serde_json::to_value(ServerConfig::default()).expect("ServerConfig serializes to JSON");

        ServerCli::command()
            .get_arguments()
            .filter(|arg| !arg.is_hide_set())
            .filter(|arg| !matches!(arg.get_id().as_str(), "help" | "version"))
            .map(|arg| {
                let id = arg.get_id().to_string();
                let default = match defaults.get(&id) {
                    None | Some(serde_json::Value::Null) => None,
                    Some(serde_json::Value::String(s)) if s.is_empty() => None,
                    Some(serde_json::Value::String(s)) => Some(s.clone()),
                    Some(other) => Some(other.to_string()),
                };
                ServerConfigField {
                    cli_flag: arg.get_long().map(|long| format!("--{long}")),
                    env_var: format!("{ENV_PREFIX}{}", id.to_uppercase()),
                    description: arg
                        .get_long_help()
                        .or(arg.get_help())
                        .map(|help| help.to_string())
                        .unwrap_or_default(),
                    default,
                    id,
                }
            })
            .collect()
    }

    /// Load external service IP restrictions from environment variables.
    ///
    /// Scans for env vars matching `SCANOPY_EXTERNAL_SERVICE_<NAME>_ALLOWED_IPS`
    /// and parses them into a HashMap of service name -> allowed IPs.
    fn load_external_service_allowed_ips() -> HashMap<String, Vec<String>> {
        let mut result = HashMap::new();
        let prefix = "SCANOPY_EXTERNAL_SERVICE_";
        let suffix = "_ALLOWED_IPS";

        for (key, value) in std::env::vars() {
            if key.starts_with(prefix) && key.ends_with(suffix) {
                // Extract service name from: SCANOPY_EXTERNAL_SERVICE_<NAME>_ALLOWED_IPS
                let service_name = key
                    .strip_prefix(prefix)
                    .and_then(|s| s.strip_suffix(suffix))
                    .map(|s| s.to_lowercase());

                if let Some(name) = service_name
                    && !name.is_empty()
                {
                    let ips: Vec<String> = value
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();

                    if !ips.is_empty() {
                        result.insert(name, ips);
                    }
                }
            }
        }

        result
    }

    pub fn database_url(&self) -> String {
        self.database_url.to_string()
    }

    /// The license key that actually applies, wrapped as a [`LicenseKey`]. On
    /// cloud licensing is irrelevant, so a stray `SCANOPY_LICENSE_KEY` is
    /// ignored — it must never validate, lock, or reconfigure a cloud
    /// deployment.
    pub fn effective_license_key(&self) -> Option<LicenseKey> {
        if get_deployment_type(self) == DeploymentType::Cloud {
            None
        } else {
            self.license_key.clone().map(LicenseKey::new)
        }
    }
}

pub struct AppState {
    pub config: ServerConfig,
    pub services: ServiceFactory,
    pub session_store: SessionManagerLayer<PostgresStore>,
    /// Present only when a license key applies (self-hosted commercial). A
    /// keyless deployment (community or cloud) has no license service —
    /// licensing is "not required".
    pub license_service: Option<Arc<LicenseService>>,
    /// Present only when `license_signing_key` is configured (the cloud).
    /// Mints license keys and entitlements for orgs on self-hosted plans.
    pub license_issuer: Option<Arc<LicenseIssuer>>,
    pub pool: PgPool,
    /// Cancelled when a shutdown signal arrives. Stops the HTTP listener and ends open SSE
    /// streams so in-flight requests can drain.
    pub shutdown: CancellationToken,
}

impl AppState {
    pub async fn new(config: ServerConfig) -> Result<Arc<Self>, Error> {
        let storage =
            StorageFactory::new(&config.database_url(), config.use_secure_session_cookies).await?;
        let services = ServiceFactory::new(&storage, config.clone()).await?;

        // Built by the service factory, which gives it the org service it
        // persists entitlements and reconciles plans through.
        let license_service = services.license_service.clone();

        // A configured but unparseable signing key fails startup rather than
        // silently serving 500s from every mint.
        let license_issuer = config
            .license_signing_key
            .as_deref()
            .map(LicenseIssuer::from_pem)
            .transpose()?
            .map(Arc::new);

        Ok(Arc::new(Self {
            config,
            services,
            session_store: storage.sessions,
            license_service,
            license_issuer,
            pool: storage.pool,
            shutdown: CancellationToken::new(),
        }))
    }
}

pub fn get_deployment_type(config: &ServerConfig) -> DeploymentType {
    if config.stripe_secret.is_some() {
        // Stripe configured => the Scanopy-operated cloud.
        DeploymentType::Cloud
    } else if config.license_key.is_some() {
        // Self-hosted with a license key => commercial edition. Validity is a
        // separate concern (LicenseService); an invalid key still reads as a
        // commercial deployment and locks the server via license middleware.
        DeploymentType::Commercial
    } else {
        // Self-hosted, no key => free community edition.
        DeploymentType::Community
    }
}

/// Get public server configuration
///
/// Returns public configuration settings like OIDC providers, billing status, etc.
#[utoipa::path(
    get,
    path = "/api/config",
    tags = [api_tags::CONFIG, api_tags::INTERNAL],
    responses(
        (status = 200, description = "Public server configuration", body = ApiResponse<PublicConfigResponse>)
    )
)]
pub async fn get_public_config(
    State(state): State<Arc<AppState>>,
    Extension(cache): Extension<Arc<AppCache>>,
) -> impl IntoResponse {
    let oidc_providers = state
        .services
        .oidc_service
        .as_ref()
        .map(|o| o.as_ref().list_providers())
        .unwrap_or_default();

    let deployment_type = get_deployment_type(&state.config);
    let license_service = state.license_service.as_deref();
    let current_license = match license_service {
        Some(svc) => Some(svc.current_status().await),
        None => None,
    };
    let license_key_type = license_service.map(|svc| svc.key_type());
    let license_entitlement_at = match license_service {
        Some(svc) => svc.entitlement_at().await,
        None => None,
    };
    let license_instances = match license_service {
        Some(svc) => svc.instance_usage().await,
        None => None,
    };
    let license_status = current_license.as_ref().map(|s| s.discriminant());
    let license_expiry = current_license.as_ref().and_then(|s| s.expiry_date());
    let license_intended_expiry = current_license
        .as_ref()
        .and_then(|s| s.intended_expiry_date());
    let license_valid_through = current_license.as_ref().and_then(|s| s.valid_through());
    let license_in_grace_period = current_license
        .as_ref()
        .is_some_and(|s| s.in_grace_period());

    // Self-hosted only: is the instance at its licensed org cap? Cloud is
    // multi-tenant, and unlimited-org plans (Commercial/Enterprise) never cap.
    // Fails open (false) so a transient DB error doesn't block registration.
    let org_limit_reached = if !deployment_type.caps_orgs_by_license() {
        false
    } else {
        use crate::server::organizations::r#impl::base::Organization;
        use crate::server::shared::services::traits::CrudService;
        use crate::server::shared::storage::filter::StorableFilter;

        let included_orgs = self_hosted_plan(license_service)
            .await
            .config()
            .included_orgs;
        match included_orgs {
            Some(max) => state
                .services
                .organization_service
                .get_all(StorableFilter::<Organization>::new())
                .await
                .map(|orgs| orgs.len() >= max as usize)
                .unwrap_or(false),
            None => false,
        }
    };

    let available_upgrade = if deployment_type.is_self_hosted() {
        latest_release::available_upgrade(&cache).await
    } else {
        None
    };

    (
        [(CACHE_CONTROL, "no-store, no-cache, must-revalidate")],
        Json(ApiResponse::success(PublicConfigResponse {
            server_port: state.config.server_port,
            disable_registration: state.config.disable_registration,
            disable_password_login: state.config.disable_password_login,
            oidc_providers,
            stripe_publishable_key: state.config.stripe_key.clone(),
            discount_save_offer_available: std::env::var("STRIPE_SAVE_OFFER_COUPON_ID").is_ok(),
            has_integrated_daemon: state.config.integrated_daemon_url.is_some(),
            // Whether a transport was actually built, not whether it looks configured.
            // Re-deriving this from config duplicated the selection rules in
            // `ServiceFactory` and could disagree with them: a bad relay hostname or an
            // unparseable sender leaves `email_service` as `None` while every variable is
            // present, and the UI would go on offering password reset and email invites
            // that silently do nothing.
            has_email_service: state.services.email_service.is_some(),
            public_url: state.config.public_url.clone(),
            posthog_key: state.config.posthog_key.clone(),
            needs_cookie_consent: state.config.posthog_key.is_some(),
            deployment_type,
            license_status,
            license_key_type,
            license_entitlement_at,
            license_instances,
            non_production: state.config.non_production,
            license_expiry,
            license_intended_expiry,
            license_valid_through,
            license_in_grace_period,
            snapshot_retention_days_override: state.config.snapshot_retention_days_override,
            org_limit_reached,
            server_admin_contact_email: state.config.server_admin_contact_email.clone(),
            // The built issuer, not the config value: a signing key can be
            // present and unparseable, and this is the same gate the mint
            // path checks.
            license_signing_available: state.license_issuer.is_some(),
            license_key_buffer_days: crate::server::license::mint::PAID_THROUGH_BUFFER_DAYS as u32,
            license_key_grace_days: crate::server::license::mint::GRACE_PERIOD_DAYS as u32,
            available_upgrade,
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// `ServerConfig` fields with no CLI flag, and so no row in the docs. Anything a self-hosted
    /// user sets belongs in `ServerCli` instead.
    const ENV_ONLY_INTERNAL: &[&str] = &[
        // Testing only: writes rendered emails, tokens included, to disk.
        "email_log_dir",
        // Testing only.
        "enforce_plan_limits_for_testing",
        // Cloud only.
        "license_signing_key",
        // Points license check-ins at a local server for testing.
        "license_server_url",
        // A family of `SCANOPY_EXTERNAL_SERVICE_<NAME>_ALLOWED_IPS` variables, not one field.
        "external_service_allowed_ips",
    ];

    fn config_fields() -> HashSet<String> {
        let serde_json::Value::Object(fields) =
            serde_json::to_value(ServerConfig::default()).unwrap()
        else {
            panic!("ServerConfig serializes to an object");
        };
        fields.keys().cloned().collect()
    }

    fn cli_args() -> HashSet<String> {
        ServerCli::command()
            .get_arguments()
            .map(|arg| arg.get_id().to_string())
            .filter(|id| id != "help" && id != "version")
            .collect()
    }

    #[test]
    fn every_config_field_has_a_flag_or_is_internal() {
        let args = cli_args();
        let missing: Vec<_> = config_fields()
            .into_iter()
            .filter(|f| !args.contains(f) && !ENV_ONLY_INTERNAL.contains(&f.as_str()))
            .collect();
        assert!(
            missing.is_empty(),
            "ServerConfig fields with no ServerCli flag (add one, or list the field in \
             ENV_ONLY_INTERNAL if no self-hosted user sets it): {missing:?}"
        );
    }

    #[test]
    fn every_flag_sets_a_config_field() {
        let fields = config_fields();
        let orphans: Vec<_> = cli_args()
            .into_iter()
            .filter(|a| !fields.contains(a))
            .collect();
        assert!(
            orphans.is_empty(),
            "ServerCli flags that name no ServerConfig field: {orphans:?}"
        );
    }

    #[test]
    fn every_documented_field_has_a_description() {
        let undocumented: Vec<_> = ServerConfig::docs_fields()
            .into_iter()
            .filter(|f| f.description.trim().is_empty())
            .map(|f| f.id)
            .collect();
        assert!(
            undocumented.is_empty(),
            "ServerCli flags with no doc comment, which the docs render as the description: \
             {undocumented:?}"
        );
    }

    fn ses_config(region: Option<&str>, topic_arn: &str, relay: &str) -> ServerConfig {
        ServerConfig {
            aws_region: region.map(str::to_owned),
            email_feedback_sns_topic_arn: Some(topic_arn.to_owned()),
            smtp_relay: Some(relay.to_owned()),
            ..ServerConfig::default()
        }
    }

    #[test]
    fn aws_region_mismatches_names_settings_in_another_region() {
        let config = ses_config(
            Some("us-east-2"),
            "arn:aws:sns:us-east-1:123456789012:ses-feedback",
            "email-smtp.eu-west-1.amazonaws.com",
        );
        assert_eq!(
            config.aws_region_mismatches(),
            vec![
                (
                    "SCANOPY_EMAIL_FEEDBACK_SNS_TOPIC_ARN",
                    "us-east-1".to_owned()
                ),
                ("SCANOPY_SMTP_RELAY", "eu-west-1".to_owned()),
            ]
        );
    }

    #[test]
    fn aws_region_mismatches_passes_matching_and_non_ses_settings() {
        let matching = ses_config(
            Some("us-east-2"),
            "arn:aws:sns:us-east-2:123456789012:ses-feedback",
            "email-smtp.us-east-2.amazonaws.com",
        );
        assert!(matching.aws_region_mismatches().is_empty());

        let other_relay = ses_config(
            Some("us-east-2"),
            "arn:aws:sns:us-east-2:123456789012:ses-feedback",
            "smtp.gmail.com",
        );
        assert!(other_relay.aws_region_mismatches().is_empty());
    }

    #[test]
    fn aws_region_mismatches_checks_nothing_without_a_region() {
        let config = ses_config(
            None,
            "arn:aws:sns:us-east-1:123456789012:ses-feedback",
            "email-smtp.eu-west-1.amazonaws.com",
        );
        assert!(config.aws_region_mismatches().is_empty());
    }
}

#[cfg(test)]
mod deployment_tests {
    use super::*;

    #[test]
    fn cloud_ignores_a_configured_license_key() {
        let self_hosted = ServerConfig {
            license_key: Some("key".to_string()),
            ..ServerConfig::default()
        };
        assert!(self_hosted.effective_license_key().is_some());

        let cloud = ServerConfig {
            stripe_secret: Some("sk_test_unused".to_string()),
            ..self_hosted
        };
        assert!(cloud.effective_license_key().is_none());
    }

    #[test]
    fn plan_limits_apply_on_cloud_and_on_a_self_hosted_server_that_asks_for_them() {
        let cloud = ServerConfig {
            stripe_secret: Some("sk_test_unused".to_string()),
            ..ServerConfig::default()
        };
        let testing = ServerConfig {
            enforce_plan_limits_for_testing: true,
            ..ServerConfig::default()
        };
        assert!(cloud.enforces_plan_limits());
        assert!(testing.enforces_plan_limits());
        assert!(!ServerConfig::default().enforces_plan_limits());

        // The testing flag enforces limits without making the server cloud.
        assert_eq!(get_deployment_type(&testing), DeploymentType::Community);
    }

    /// Test stacks set `SCANOPY_ENFORCE_PLAN_LIMITS_FOR_TESTING`, and ones written before the
    /// rename set `SCANOPY_ENFORCE_BILLING_FOR_TESTING`; either turns the flag on.
    #[test]
    #[serial_test::serial]
    fn either_env_name_enforces_plan_limits() {
        for name in [
            "SCANOPY_ENFORCE_PLAN_LIMITS_FOR_TESTING",
            "SCANOPY_ENFORCE_BILLING_FOR_TESTING",
        ] {
            let original = std::env::var(name).ok();
            // SAFETY: This test runs serially and restores the original value after
            unsafe { std::env::set_var(name, "true") };

            let config = ServerConfig::load(ServerCli::parse_from(["scanopy-server"]));

            // SAFETY: This test runs serially
            unsafe {
                match original {
                    Some(val) => std::env::set_var(name, val),
                    None => std::env::remove_var(name),
                }
            }

            assert!(
                config
                    .expect("config loads")
                    .enforce_plan_limits_for_testing,
                "{name} did not set the flag"
            );
        }
    }
}
