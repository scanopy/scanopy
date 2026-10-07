//! HTTP transport for the Proxmox VE API.
//!
//! Builds its own `reqwest::Client`, like the UniFi client, honouring the daemon's
//! `accept_invalid_scan_certs` setting and trusted CA bundle: a Proxmox VE node serves a
//! self-signed certificate from its own cluster CA unless an operator replaced it.

use std::time::Duration;

use anyhow::{Error, Result, anyhow, bail};
use reqwest::{Client, StatusCode};
use serde::de::DeserializeOwned;

use crate::daemon::discovery::service::warnings::AttemptOutcome;
use crate::server::credentials::r#impl::mapping::ProxmoxQueryCredential;
use crate::server::shared::trusted_ca::TrustedCaBundle;

use super::types::{PveEnvelope, PveVersion};

/// Label used in credential-resolution error messages.
const CREDENTIAL_LABEL: &str = "Proxmox VE API connection";

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// The probe runs without an outer timeout, so every request bounds itself.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// An authenticated connection to one node's API.
pub struct ProxmoxClient {
    client: Client,
    origin: String,
}

impl ProxmoxClient {
    /// Build the client and confirm the token with `GET /version`, which every valid token may
    /// read whatever its privileges.
    pub async fn connect(
        host: &str,
        credential: &ProxmoxQueryCredential,
        accept_invalid_certs: bool,
        trusted_ca: Option<&TrustedCaBundle>,
    ) -> Result<(Self, PveVersion), Error> {
        let secret = credential
            .token_secret
            .resolve("token_secret", CREDENTIAL_LABEL)?;
        // A default header, so the secret never lands in a URL or a log line.
        let mut value = reqwest::header::HeaderValue::from_str(&format!(
            "PVEAPIToken={}={}",
            credential.token_id.trim(),
            secret.expose_secret().trim()
        ))
        .map_err(|_| {
            anyhow!("Proxmox token ID or secret contains characters not valid in an HTTP header")
        })?;
        value.set_sensitive(true);
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::AUTHORIZATION, value);

        let client = TrustedCaBundle::apply(
            trusted_ca,
            Client::builder()
                .connect_timeout(CONNECT_TIMEOUT)
                .timeout(REQUEST_TIMEOUT)
                .danger_accept_invalid_certs(accept_invalid_certs)
                .default_headers(headers),
        )
        .build()
        .map_err(|e| anyhow!("Failed to build Proxmox HTTP client: {e}"))?;

        let host = match host.parse::<std::net::IpAddr>() {
            Ok(std::net::IpAddr::V6(v6)) => format!("[{v6}]"),
            _ => host.to_string(),
        };
        let client = Self {
            client,
            origin: format!("https://{}:{}/api2/json", host, credential.port),
        };
        let version = client
            .get::<PveVersion>("/version")
            .await?
            .ok_or_else(|| anyhow!("Proxmox VE refused /version to this token"))?;
        Ok((client, version))
    }

    /// How a [`Self::connect`] failure should be reported. Same reading as the UniFi client: a
    /// transport error is classified from the `reqwest::Error` in the chain, and anything this
    /// client raised itself is the API refusing the token.
    pub fn classify_connect_error(error: &Error) -> AttemptOutcome {
        let observed = error
            .chain()
            .find_map(|cause| cause.downcast_ref::<reqwest::Error>())
            .map(AttemptOutcome::from)
            .unwrap_or(AttemptOutcome::Rejected);
        AttemptOutcome::for_credential_error(error, observed)
    }

    /// GET an API path and unwrap its `data`.
    ///
    /// `Ok(None)` when the token lacks the privilege for this path (403): every read past
    /// `/version` depends on what the token was granted, and a missing grant narrows what the
    /// integration reports rather than failing it. A 401 is a refused token and fails.
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<Option<T>, Error> {
        let response = self
            .client
            .get(format!("{}{}", self.origin, path))
            .send()
            .await
            .map_err(|e| Error::new(e).context("Could not reach the Proxmox VE API"))?;

        match response.status() {
            s if s.is_success() => {}
            StatusCode::UNAUTHORIZED => {
                bail!("Proxmox VE rejected the API token (check the token ID and secret)")
            }
            StatusCode::FORBIDDEN => {
                tracing::debug!(path, "Proxmox VE token lacks the privilege for this path");
                return Ok(None);
            }
            s => bail!("Proxmox VE returned HTTP {s} for {path}"),
        }

        let envelope: PveEnvelope<T> = response
            .json()
            .await
            .map_err(|e| anyhow!("Could not parse the Proxmox VE {path} response: {e}"))?;
        Ok(Some(envelope.data))
    }

    /// Like [`Self::get`], for a read that fails in ordinary operation: a guest agent that is
    /// not installed or not running answers with a 5xx. Logged and treated as absent.
    pub async fn get_best_effort<T: DeserializeOwned>(&self, path: &str) -> Option<T> {
        match self.get(path).await {
            Ok(value) => value,
            Err(e) => {
                tracing::debug!(path, error = %e, "Proxmox VE read skipped");
                None
            }
        }
    }
}
