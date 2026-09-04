use crate::server::auth::r#impl::oidc::OidcProviderMetadata;
use crate::server::openapi::tags as api_tags;
use crate::server::shared::types::api::ApiResponse;
use crate::server::{
    auth::r#impl::oidc::OidcProviderConfig, shared::services::factory::ServiceFactory,
};
use anyhow::{Error, Result};
use axum::Json;
use axum::extract::State;
use axum::http::header::CACHE_CONTROL;
use axum::response::IntoResponse;
use clap::Parser;
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

use crate::server::shared::storage::factory::StorageFactory;
use sqlx::PgPool;

#[derive(Parser)]
#[command(name = "scanopy-server")]
#[command(about = "Scanopy server")]
pub struct ServerCli {
    /// Override server port
    #[arg(long)]
    server_port: Option<u16>,

    /// Override log level
    #[arg(long)]
    log_level: Option<String>,

    /// Override rust system log level
    #[arg(long)]
    rust_log: Option<String>,

    /// Override database path
    #[arg(long)]
    database_url: Option<String>,

    /// Override integrated daemon url
    #[arg(long)]
    integrated_daemon_url: Option<String>,

    /// Use secure session cookies (if serving UI behind HTTPS)
    #[arg(long)]
    use_secure_session_cookies: Option<bool>,

    /// Enable or disable registration flow
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    disable_registration: Option<bool>,

    /// Disable email/password login (use when OIDC is configured)
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    disable_password_login: Option<bool>,

    /// Stripe publishable key (exposed to the frontend for Stripe Elements)
    #[arg(long)]
    stripe_key: Option<String>,

    /// Stripe secret key
    #[arg(long)]
    stripe_secret: Option<String>,

    /// Stripe webhook signing secret
    #[arg(long)]
    stripe_webhook_secret: Option<String>,

    #[arg(long)]
    smtp_username: Option<String>,

    #[arg(long)]
    smtp_password: Option<String>,

    /// Email used as to/from in emails send by Scanopy using SMTP
    #[arg(long)]
    smtp_email: Option<String>,

    #[arg(long)]
    smtp_relay: Option<String>,

    #[arg(long)]
    smtp_port: Option<u16>,

    /// Server URL used in features like password reset and invite links
    #[arg(long)]
    public_url: Option<String>,

    /// Configure what proxy (if any) is providing IP address for requests, ie in a reverse proxy setup, for accurate IP in auth event logging
    #[arg(long)]
    pub client_ip_source: Option<String>,

    /// List of OIDC providers
    #[arg(long)]
    pub oidc_providers: Option<String>,

    #[arg(long)]
    pub posthog_key: Option<String>,

    #[arg(long)]
    pub metrics_token: Option<String>,

    #[arg(long)]
    pub brevo_api_key: Option<String>,
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
    pub client_ip_source: Option<String>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_relay: Option<String>,
    pub smtp_email: Option<String>,
    /// SMTP port. Defaults to 465 (implicit TLS) when unset. Ports other than
    /// 465 use STARTTLS (e.g. 587, 25).
    pub smtp_port: Option<u16>,
    /// Directory the logging email transport writes rendered emails to
    /// (`SCANOPY_EMAIL_LOG_DIR`). Setting it selects that transport ahead of
    /// Brevo and SMTP, so nothing is actually delivered — it exists to
    /// exercise email paths locally without credentials.
    ///
    /// Testing only: the written files contain full bodies, including
    /// password-reset and verification tokens, in plaintext on disk.
    #[serde(default)]
    pub email_log_dir: Option<PathBuf>,
    #[serde(default)]
    pub oidc_providers: Option<Vec<OidcProviderConfig>>,

    // Used in SaaS deployment
    pub stripe_key: Option<String>,
    pub stripe_secret: Option<String>,
    pub stripe_webhook_secret: Option<String>,
    pub posthog_key: Option<String>,

    // Testing
    #[serde(default)]
    pub enforce_billing_for_testing: bool,

    // Metrics
    pub metrics_token: Option<String>,

    // Brevo CRM integration
    pub brevo_api_key: Option<String>,

    // External service IP restrictions
    // Maps service name (lowercase) to list of allowed IPs/CIDRs
    // Populated from SCANOPY_EXTERNAL_SERVICE_<NAME>_ALLOWED_IPS env vars
    #[serde(default)]
    pub external_service_allowed_ips: HashMap<String, Vec<String>>,

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
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentType {
    Cloud,
    SelfHosted,
}

impl DeploymentType {
    /// True for deployments the customer hosts themselves. Cloud is the only
    /// Scanopy-LLC-operated deployment, so this is the "is Scanopy LLC the
    /// sender" signal the email footer gates on.
    pub fn is_self_hosted(&self) -> bool {
        !matches!(self, DeploymentType::Cloud)
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
    /// Whether this deployment has billing configured.
    pub billing_enabled: bool,
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
    /// Whether the deployment asks users to opt in to product email.
    pub has_email_opt_in: bool,
    /// Base URL this server is reachable at, as configured by the operator.
    #[schema(format = "uri")]
    pub public_url: String,
    /// Public analytics key, when analytics is enabled.
    pub posthog_key: Option<String>,
    /// Whether the client should show a cookie-consent prompt.
    pub needs_cookie_consent: bool,
    /// How this instance is run: cloud, or self-hosted.
    pub deployment_type: DeploymentType,
    /// `SCANOPY_SNAPSHOT_RETENTION_DAYS_OVERRIDE` if set on this instance.
    /// Frontend uses it inside the plan-comparison view to display the
    /// effective retention for this deployment rather than the per-plan
    /// fixture default.
    pub snapshot_retention_days_override: Option<u32>,
    /// True when this self-hosted instance has reached the organization cap
    /// (`included_orgs`) of the plan it runs on, so new-org registration is
    /// blocked. Always false on cloud (multi-tenant) and on unlimited-org plans
    /// — which is every self-hosted deployment unless the plan is edited.
    pub org_limit_reached: bool,
    /// Admin contact email to show users blocked by `org_limit_reached`,
    /// from `SCANOPY_SERVER_ADMIN_CONTACT_EMAIL`.
    #[schema(value_type = String, format = "email")]
    pub server_admin_contact_email: Option<EmailAddress>,
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
            stripe_key: None,
            stripe_secret: None,
            stripe_webhook_secret: None,
            smtp_username: None,
            smtp_password: None,
            smtp_email: None,
            smtp_relay: None,
            smtp_port: None,
            email_log_dir: None,
            client_ip_source: None,
            oidc_providers: None,
            posthog_key: None,
            enforce_billing_for_testing: false,
            metrics_token: None,
            brevo_api_key: None,
            external_service_allowed_ips: HashMap::new(),
            server_admin_contact_email: None,
            snapshot_retention_days_override: None,
        }
    }
}

impl ServerConfig {
    pub fn load(cli_args: ServerCli) -> anyhow::Result<Self> {
        // Standard configuration layering: Defaults → Env → CLI (highest priority)
        let mut figment = Figment::from(Serialized::defaults(ServerConfig::default()))
            .merge(Toml::file("../oidc.toml"))
            .merge(Env::prefixed("SCANOPY_"));

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
        if let Some(metrics_token) = cli_args.metrics_token {
            figment = figment.merge(("metrics_token", metrics_token));
        }
        if let Some(brevo_api_key) = cli_args.brevo_api_key {
            figment = figment.merge(("brevo_api_key", brevo_api_key));
        }

        let mut config: ServerConfig = figment
            .extract()
            .map_err(|e| Error::msg(format!("Configuration error: {}", e)))?;

        // Parse external service IP restrictions from env vars
        // Format: SCANOPY_EXTERNAL_SERVICE_<NAME>_ALLOWED_IPS=192.168.1.0/24,10.0.0.1
        config.external_service_allowed_ips = Self::load_external_service_allowed_ips();

        Ok(config)
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
}

pub struct AppState {
    pub config: ServerConfig,
    pub services: ServiceFactory,
    pub session_store: SessionManagerLayer<PostgresStore>,
    pub pool: PgPool,
}

impl AppState {
    pub async fn new(config: ServerConfig) -> Result<Arc<Self>, Error> {
        let storage =
            StorageFactory::new(&config.database_url(), config.use_secure_session_cookies).await?;
        let services = ServiceFactory::new(&storage, config.clone()).await?;

        Ok(Arc::new(Self {
            config,
            services,
            session_store: storage.sessions,
            pool: storage.pool,
        }))
    }
}

pub fn get_deployment_type(config: &ServerConfig) -> DeploymentType {
    if config.stripe_secret.is_some() {
        // Stripe configured => the Scanopy-operated cloud.
        DeploymentType::Cloud
    } else {
        // Everything else is the customer's own instance, running the full
        // edition — there is no key to check and nothing to unlock.
        DeploymentType::SelfHosted
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
pub async fn get_public_config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let oidc_providers = state
        .services
        .oidc_service
        .as_ref()
        .map(|o| o.as_ref().list_providers())
        .unwrap_or_default();

    let deployment_type = get_deployment_type(&state.config);

    let billing_enabled = state.config.stripe_secret.is_some();
    // Self-hosted only: is the instance at the org cap of the plan it runs on?
    // Cloud is multi-tenant, and the self-hosted plan is unlimited-org, so this
    // only ever fires if that plan is edited to carry a cap.
    // Fails open (false) so a transient DB error doesn't block registration.
    let org_limit_reached = if billing_enabled {
        false
    } else {
        use crate::server::organizations::r#impl::base::Organization;
        use crate::server::shared::services::traits::CrudService;
        use crate::server::shared::storage::filter::StorableFilter;

        let included_orgs = crate::server::billing::plans::self_hosted_plan()
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

    (
        [(CACHE_CONTROL, "no-store, no-cache, must-revalidate")],
        Json(ApiResponse::success(PublicConfigResponse {
            server_port: state.config.server_port,
            disable_registration: state.config.disable_registration,
            disable_password_login: state.config.disable_password_login,
            oidc_providers,
            billing_enabled,
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
            has_email_opt_in: state.config.brevo_api_key.is_some(),
            posthog_key: state.config.posthog_key.clone(),
            needs_cookie_consent: state.config.posthog_key.is_some()
                || state.config.brevo_api_key.is_some(),
            deployment_type,
            snapshot_retention_days_override: state.config.snapshot_retention_days_override,
            org_limit_reached,
            server_admin_contact_email: state.config.server_admin_contact_email.clone(),
        })),
    )
}
