//! HTTP transport for the UniFi Network Application controller.
//!
//! Two auth transports behind one entry point, because the *endpoint* is identical and only
//! the credential material differs:
//!
//! - **API key** — an `X-API-KEY` header. Stateless, but **UniFi OS only**: Ubiquiti does not
//!   support API keys on the legacy self-hosted Network Application (port 8443).
//! - **Local admin** — username/password exchanged for a session cookie. Works everywhere.
//!
//! Builds its own `reqwest::Client` rather than reusing the daemon's API client, which is
//! bound to the Scanopy server's URL and auth. Same shape as the other outbound scanners
//! (`daemon/utils/scanner.rs`, `integration/container/scanner.rs`), including honouring the
//! daemon's `accept_invalid_scan_certs` setting — UniFi controllers ship self-signed certs, so
//! without it the probe fails before it can authenticate.

use std::time::Duration;

use anyhow::{Error, Result, anyhow, bail};
use reqwest::{Client, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::json;

use crate::daemon::discovery::service::warnings::AttemptOutcome;
use crate::server::credentials::r#impl::mapping::{UnifiAuth, UnifiQueryCredential};
use crate::server::shared::trusted_ca::TrustedCaBundle;

use super::types::UnifiEnvelope;

/// Label used in credential-resolution error messages.
const CREDENTIAL_LABEL: &str = "UniFi controller connection";

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// The probe runs without an outer timeout wrapper (`dispatch::probe_integrations` only wraps
/// `execute`), so the client must bound its own requests or an unresponsive host stalls the scan.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Which API layout the controller presents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControllerFlavor {
    /// UniFi OS console (443) or UniFi OS Server (11443): the Network Application sits behind
    /// a `/proxy/network` prefix.
    UnifiOs,
    /// Legacy self-hosted Network Application (8443): endpoints are served at the root.
    Legacy,
}

impl ControllerFlavor {
    pub fn base_path(self) -> &'static str {
        match self {
            Self::UnifiOs => "/proxy/network",
            Self::Legacy => "",
        }
    }

    /// Session-login endpoint for this flavor.
    fn login_path(self) -> &'static str {
        match self {
            // UniFi OS authenticates at the console level, outside the network app prefix.
            Self::UnifiOs => "/api/auth/login",
            Self::Legacy => "/api/login",
        }
    }
}

/// An authenticated connection to one controller site.
pub struct UnifiClient {
    client: Client,
    origin: String,
    flavor: ControllerFlavor,
    site: String,
}

impl UnifiClient {
    pub fn flavor(&self) -> ControllerFlavor {
        self.flavor
    }

    pub fn site(&self) -> &str {
        &self.site
    }

    /// Connect and authenticate, auto-detecting the controller flavor.
    ///
    /// Detection is by path rather than by port: while 443/11443 imply UniFi OS and 8443 implies
    /// legacy in practice, reverse proxies and non-standard ports break that correlation. We try
    /// the UniFi OS layout first and fall back, so at most one extra request is spent.
    pub async fn connect(
        host: &str,
        credential: &UnifiQueryCredential,
        accept_invalid_certs: bool,
        trusted_ca: Option<&TrustedCaBundle>,
    ) -> Result<Self, Error> {
        let mut builder = TrustedCaBundle::apply(
            trusted_ca,
            Client::builder()
                .connect_timeout(CONNECT_TIMEOUT)
                .timeout(REQUEST_TIMEOUT)
                .danger_accept_invalid_certs(accept_invalid_certs)
                // Session auth rides a cookie; harmless for the API-key transport.
                .cookie_store(true),
        );

        if let UnifiAuth::ApiKey { api_key } = &credential.auth {
            // Set as a default header so the key is never interpolated into a URL or a log line.
            let secret = api_key.resolve("api_key", CREDENTIAL_LABEL)?;
            let mut headers = reqwest::header::HeaderMap::new();
            let mut value = reqwest::header::HeaderValue::from_str(secret.expose_secret())
                .map_err(|_| {
                    anyhow!(
                        "UniFi API key contains characters that are not valid in an HTTP header"
                    )
                })?;
            value.set_sensitive(true);
            headers.insert("X-API-KEY", value);
            builder = builder.default_headers(headers);
        }

        let client = builder
            .build()
            .map_err(|e| anyhow!("Failed to build UniFi HTTP client: {e}"))?;

        let origin = format!("https://{}:{}", host, credential.port);

        // Try UniFi OS first, then legacy.
        let mut last_error = None;
        for flavor in [ControllerFlavor::UnifiOs, ControllerFlavor::Legacy] {
            let candidate = Self {
                client: client.clone(),
                origin: origin.clone(),
                flavor,
                site: credential.site.clone(),
            };
            match candidate.authenticate_and_verify(credential).await {
                Ok(()) => return Ok(candidate),
                Err(e) => last_error = Some(e),
            }
        }
        Err(last_error
            .unwrap_or_else(|| anyhow!("Could not determine UniFi controller API layout")))
    }

    /// How a [`Self::connect`] failure should be reported to the operator.
    ///
    /// Reads the transport error out of the chain rather than matching on our own message text,
    /// so rewording a `bail!` cannot silently change the classification. Everything this client
    /// raises itself is a login or HTTP-status problem — a refused password, an unknown site, an
    /// unexpected status — which is why the fallback is `Rejected`; a genuine transport failure
    /// always carries a `reqwest::Error` and is classified from it (self-signed certificate,
    /// connection refused, timeout).
    pub fn classify_connect_error(error: &Error) -> AttemptOutcome {
        let observed = error
            .chain()
            .find_map(|cause| cause.downcast_ref::<reqwest::Error>())
            .map(AttemptOutcome::from)
            .unwrap_or(AttemptOutcome::Rejected);
        // A credential field we could not read never reached the network, so neither the transport
        // reading nor the fallback applies to it — telling an operator their password was refused,
        // when the file holding it is missing, points them at the wrong thing entirely (GH #668).
        AttemptOutcome::for_credential_error(error, observed)
    }

    /// Log in (if the transport is stateful) and confirm the requested site exists.
    ///
    /// Verifies via the **site list** rather than by probing a site-scoped endpoint and reading
    /// the status code. Measured against UniFi OS Server 5.1.21, a site-scoped call with an
    /// unknown site name returns **401, not 404** — indistinguishable from a rejected
    /// credential, so a typo'd site would be reported to the user as a bad API key.
    ///
    /// `api/self/sites` is not site-scoped, so one call settles both questions: a 401 there is
    /// genuinely a bad credential, and if it succeeds the site name can be checked against the
    /// returned list and the valid names offered back to the user.
    async fn authenticate_and_verify(
        &self,
        credential: &UnifiQueryCredential,
    ) -> Result<(), Error> {
        self.login(credential).await?;

        match self.list_sites().await {
            Ok(sites) => {
                if sites.iter().any(|s| s == &self.site) {
                    return Ok(());
                }
                bail!(
                    "UniFi site '{}' was not found on this controller. Available sites: {}. \
                     Use the internal site name from the controller URL \
                     (/manage/site/<name>), not its display name.",
                    self.site,
                    if sites.is_empty() {
                        "none visible to this account".to_string()
                    } else {
                        sites.join(", ")
                    }
                );
            }
            // Older controllers may not expose the site list. Fall back to probing a
            // site-scoped endpoint: less precise, but better than refusing to connect.
            Err(e) if e.to_string().contains("HTTP 404") => {
                let _: UnifiEnvelope<serde_json::Value> = self.get_site("stat/sysinfo").await?;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Internal names of the sites this credential can see.
    async fn list_sites(&self) -> Result<Vec<String>, Error> {
        #[derive(serde::Deserialize)]
        struct SiteEntry {
            /// The internal site name — what a credential's `site` field must hold.
            name: Option<String>,
        }

        let envelope: UnifiEnvelope<SiteEntry> = self.get("api/self/sites").await?;
        Ok(envelope.data.into_iter().filter_map(|s| s.name).collect())
    }

    /// Establish a session for the local-admin transport. No-op for API keys.
    async fn login(&self, credential: &UnifiQueryCredential) -> Result<(), Error> {
        let UnifiAuth::LocalAdmin { username, password } = &credential.auth else {
            return Ok(());
        };

        let secret = password.resolve("password", CREDENTIAL_LABEL)?;
        let url = format!("{}{}", self.origin, self.flavor.login_path());
        let response = self
            .client
            .post(&url)
            .json(&json!({ "username": username, "password": secret.expose_secret() }))
            .send()
            .await
            .map_err(|e| Error::new(e).context("Could not reach UniFi controller"))?;

        match response.status() {
            s if s.is_success() => Ok(()),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                bail!("UniFi controller rejected the username or password")
            }
            StatusCode::BAD_REQUEST => {
                // Legacy controllers answer a bad login with 400 rather than 401.
                bail!("UniFi controller rejected the username or password")
            }
            s => bail!("UniFi login failed with HTTP {s}"),
        }
    }

    /// GET a site-scoped endpoint (e.g. `stat/device`) and decode its envelope.
    ///
    /// The site is validated up front by [`Self::authenticate_and_verify`], so a 401 reaching
    /// here really is a credential problem rather than an unknown site.
    pub async fn get_site<T: DeserializeOwned>(
        &self,
        endpoint: &str,
    ) -> Result<UnifiEnvelope<T>, Error> {
        self.get(&format!("api/s/{}/{}", self.site, endpoint)).await
    }

    /// GET any endpoint under the controller's network-app base path and decode its envelope.
    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<UnifiEnvelope<T>, Error> {
        let url = format!("{}{}/{}", self.origin, self.flavor.base_path(), path);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::new(e).context("Could not reach UniFi controller"))?;

        let endpoint = path;
        match response.status() {
            s if s.is_success() => {}
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                bail!("{}", self.rejected_credential_message())
            }
            s => bail!("UniFi controller returned HTTP {s} for {endpoint}"),
        }

        let envelope: UnifiEnvelope<T> = response
            .json()
            .await
            .map_err(|e| anyhow!("Could not parse UniFi {endpoint} response: {e}"))?;

        if !envelope.meta.is_ok() {
            bail!(
                "UniFi controller reported an error for {endpoint}: {}",
                envelope.meta.msg.as_deref().unwrap_or("unknown error")
            );
        }
        Ok(envelope)
    }

    /// A rejected credential on the legacy layout is the single most likely misconfiguration,
    /// because API keys simply do not exist there — say so instead of a bare 401.
    fn rejected_credential_message(&self) -> String {
        match self.flavor {
            ControllerFlavor::Legacy => "UniFi controller rejected the credential. Note that the \
                 legacy self-hosted Network Application does not support API keys — use a UniFi \
                 Local Admin credential instead."
                .to_string(),
            ControllerFlavor::UnifiOs => "UniFi controller rejected the credential".to_string(),
        }
    }
}
