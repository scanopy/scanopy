//! Daemon HTTP client (GET/POST with retry) and the request wrappers built on it.
use super::*;

/// A non-2xx HTTP response from a daemon. Carries the status code so the retry policy can
/// tell a definitive client error (4xx — e.g. a 401 key mismatch that won't succeed on
/// retry) apart from a transient transport/server error.
#[derive(Debug)]
pub(crate) struct DaemonHttpError {
    /// e.g. "GET /api/status" — for the message/log.
    pub op: String,
    pub status: reqwest::StatusCode,
}

impl std::fmt::Display for DaemonHttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} failed: HTTP {}", self.op, self.status)
    }
}

impl std::error::Error for DaemonHttpError {}

/// Retry decision for daemon HTTP calls: retry transport errors and 5xx (transient), but
/// NOT a definitive 4xx (e.g. 401 key mismatch) — retrying it just spams the log with
/// escalating-backoff warnings before the poll loop marks the daemon unreachable anyway.
pub(crate) fn should_retry_daemon_http_error(e: &anyhow::Error) -> bool {
    e.downcast_ref::<DaemonHttpError>()
        .is_none_or(|h| !h.status.is_client_error())
}

/// How long a daemon request keeps retrying before it gives up.
#[derive(Debug, Clone, Copy)]
pub(crate) enum RetryPolicy {
    /// Five retries backing off from 5s to 30s, about two and a half minutes in all: long enough
    /// to ride out a daemon restart. For a polled daemon, giving up is also what marks it
    /// unreachable, so this ladder sets that threshold.
    Standard,
    /// One retry after 2s, each attempt limited to 5s: about 12s in all. For requests someone is
    /// waiting on, where the standard ladder would hold their request open for minutes.
    Brief,
}

impl RetryPolicy {
    fn backoff(self) -> ExponentialBuilder {
        match self {
            RetryPolicy::Standard => ExponentialBuilder::default()
                .with_min_delay(Duration::from_secs(5))
                .with_max_delay(Duration::from_secs(30))
                .with_max_times(UNREACHABLE_THRESHOLD),
            RetryPolicy::Brief => ExponentialBuilder::default()
                .with_min_delay(Duration::from_secs(2))
                .with_max_delay(Duration::from_secs(2))
                .with_max_times(1),
        }
    }

    /// A per-attempt limit tighter than the client's own, if this policy needs one.
    fn attempt_timeout(self) -> Option<Duration> {
        match self {
            RetryPolicy::Standard => None,
            RetryPolicy::Brief => Some(Duration::from_secs(5)),
        }
    }
}

impl DaemonService {
    // ========================================================================
    // Daemon HTTP helpers with built-in retry
    // ========================================================================

    /// Send GET request to daemon with auth, retrying per [`RetryPolicy::Standard`].
    async fn get_from_daemon<T: serde::de::DeserializeOwned>(
        &self,
        daemon: &Daemon,
        api_key: &str,
        path: &str,
    ) -> Result<T> {
        let url = format!("{}{}", daemon.base.url, path);
        let daemon_id = daemon.id;

        // SSRF guard: in cloud mode, refuse to call a daemon URL that resolves
        // to an internal address (no-op for self-hosted LAN deployments).
        crate::server::daemons::ssrf::guard_daemon_url(&daemon.base.url, self.deployment_type)
            .await?;

        (|| async {
            let response = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .send()
                .await?;

            if !response.status().is_success() {
                return Err(DaemonHttpError {
                    op: format!("GET {}", path),
                    status: response.status(),
                }
                .into());
            }

            let api_response: ApiResponse<T> = response.json().await?;

            if !api_response.success {
                anyhow::bail!(
                    "GET {} failed: {}",
                    path,
                    api_response
                        .error
                        .unwrap_or_else(|| "Unknown error".to_string())
                );
            }

            api_response
                .data
                .ok_or_else(|| anyhow::anyhow!("GET {} response missing data", path))
        })
        .retry(RetryPolicy::Standard.backoff())
        .when(should_retry_daemon_http_error)
        .notify(|e, dur| {
            tracing::warn!(
                daemon_id = %daemon_id,
                path = %path,
                "Request failed, retrying in {:?}: {}",
                dur,
                e
            );
        })
        .await
    }

    /// Send POST request to daemon with optional auth, retrying per `policy`.
    /// Returns `Option<T>` - `Some(data)` if response contains data, `None` otherwise.
    /// For endpoints that don't return data, use `::<serde_json::Value>` and ignore result.
    ///
    /// If `api_key` is `None`, the request is sent without an Authorization header.
    /// This is used for legacy daemons (< v0.14.0) that don't require authentication.
    async fn post_to_daemon<T: serde::de::DeserializeOwned>(
        &self,
        daemon: &Daemon,
        api_key: Option<&str>,
        path: &str,
        body: &impl serde::Serialize,
        policy: RetryPolicy,
    ) -> Result<Option<T>> {
        let url = format!("{}{}", daemon.base.url, path);
        let daemon_id = daemon.id;
        let body_json = serde_json::to_value(body)?;
        let api_key_owned = api_key.map(|s| s.to_owned());

        // SSRF guard: in cloud mode, refuse to POST (including credential-bearing
        // discovery payloads) to a daemon URL that resolves to an internal
        // address (no-op for self-hosted LAN deployments).
        crate::server::daemons::ssrf::guard_daemon_url(&daemon.base.url, self.deployment_type)
            .await?;

        (|| async {
            let mut request = self.client.post(&url).json(&body_json);

            // Only add auth header if API key provided (v0.14.0+ daemons)
            if let Some(ref key) = api_key_owned {
                request = request.header("Authorization", format!("Bearer {}", key));
            }
            if let Some(timeout) = policy.attempt_timeout() {
                request = request.timeout(timeout);
            }

            let response = request.send().await?;

            if !response.status().is_success() {
                return Err(DaemonHttpError {
                    op: format!("POST {}", path),
                    status: response.status(),
                }
                .into());
            }

            let api_response: ApiResponse<T> = response.json().await?;

            if !api_response.success {
                anyhow::bail!(
                    "POST {} failed: {}",
                    path,
                    api_response
                        .error
                        .unwrap_or_else(|| "Unknown error".to_string())
                );
            }

            Ok(api_response.data)
        })
        .retry(policy.backoff())
        .when(should_retry_daemon_http_error)
        .notify(|e, dur| {
            tracing::warn!(
                daemon_id = %daemon_id,
                path = %path,
                "Request failed, retrying in {:?}: {}",
                dur,
                e
            );
        })
        .await
    }

    // ========================================================================
    // Daemon HTTP methods (using helpers)
    // ========================================================================

    /// Poll daemon status via GET /api/status
    pub(crate) async fn poll_status(&self, daemon: &Daemon, api_key: &str) -> Result<DaemonStatus> {
        self.get_from_daemon(daemon, api_key, "/api/status").await
    }

    /// Poll daemon discovery via GET /api/poll
    pub(crate) async fn poll_discovery(
        &self,
        daemon: &Daemon,
        api_key: &str,
    ) -> Result<DiscoveryPollResponse> {
        self.get_from_daemon(daemon, api_key, "/api/poll").await
    }

    /// Send created entities back to daemon via POST /api/discovery/entities-created
    pub(crate) async fn send_created_entities(
        &self,
        daemon: &Daemon,
        api_key: &str,
        created_entities: CreatedEntitiesPayload,
    ) -> Result<()> {
        // Skip if there's nothing to send
        if created_entities.hosts.is_empty() && created_entities.subnets.is_empty() {
            return Ok(());
        }

        // Legacy cleanup: remove once minimum_supported >= 0.16.0
        // Pre-0.16.0 daemons expect old entity/field names in responses
        let version_str = daemon.base.version.as_ref().map(|v| v.to_string());
        if pre_interface_to_ip_address_rename(version_str.as_deref()) {
            let mut json = serde_json::to_value(&created_entities)?;
            rewrite_response_for_legacy_daemon(&mut json);
            let _: Option<serde_json::Value> = self
                .post_to_daemon(
                    daemon,
                    Some(api_key),
                    "/api/discovery/entities-created",
                    &json,
                    RetryPolicy::Standard,
                )
                .await?;
        } else {
            let _: Option<serde_json::Value> = self
                .post_to_daemon(
                    daemon,
                    Some(api_key),
                    "/api/discovery/entities-created",
                    &created_entities,
                    RetryPolicy::Standard,
                )
                .await?;
        }

        tracing::debug!(
            daemon_id = %daemon.id,
            hosts_count = created_entities.hosts.len(),
            subnets_count = created_entities.subnets.len(),
            "Sent created entities to ServerPoll daemon"
        );

        Ok(())
    }

    /// Send discovery request to daemon (HTTP only, no event publishing).
    ///
    /// If `api_key` is `None`, the request is sent without authentication.
    /// This is used for legacy daemons (< v0.14.0) that don't require auth.
    pub async fn send_discovery_request_to_daemon(
        &self,
        daemon: &Daemon,
        api_key: Option<&str>,
        request: DaemonDiscoveryRequest,
    ) -> Result<(), Error> {
        Self::warn_if_insecure_daemon_url(&daemon.base.url);

        tracing::info!(
            daemon_id = %daemon.id,
            session_id = %request.session_id,
            "Sending discovery request to daemon"
        );

        // Unified: serialize with credential_mappings via with_exposed_credentials().
        // Legacy: serialize with SNMP inline via with_exposed_snmp().
        let payload = if request.discovery_type.runs_network_scan() {
            request.with_exposed_credentials()
        } else {
            request.with_exposed_snmp()
        };
        let _: Option<serde_json::Value> = self
            .post_to_daemon(
                daemon,
                api_key,
                "/api/discovery/initiate",
                &payload,
                RetryPolicy::Standard,
            )
            .await?;

        tracing::info!(
            daemon_id = %daemon.id,
            session_id = %request.session_id,
            "Discovery request sent successfully"
        );

        Ok(())
    }

    /// Send discovery cancellation to daemon (HTTP only, no event publishing).
    ///
    /// Retries briefly: a user's cancel request waits on this, as does the stall sweep once per
    /// stalled session, and neither should sit through the standard ladder.
    ///
    /// If `api_key` is `None`, the request is sent without authentication.
    /// This is used for legacy daemons (< v0.14.0) that don't require auth.
    pub async fn send_discovery_cancellation_to_daemon(
        &self,
        daemon: &Daemon,
        api_key: Option<&str>,
        session_id: Uuid,
    ) -> Result<(), Error> {
        let _: Option<serde_json::Value> = self
            .post_to_daemon(
                daemon,
                api_key,
                "/api/discovery/cancel",
                &session_id,
                RetryPolicy::Brief,
            )
            .await?;

        tracing::info!(
            daemon_id = %daemon.id,
            session_id = %session_id,
            "Discovery cancellation sent successfully"
        );

        Ok(())
    }

    /// Send first contact request to ServerPoll daemon.
    /// This assigns the daemon its server-side ID and returns the daemon's status.
    pub(crate) async fn send_first_contact(
        &self,
        daemon: &Daemon,
        api_key: &str,
    ) -> Result<DaemonStatus> {
        let policy = DaemonVersionPolicy::default();
        // Populate deprecation/sunset warnings for this daemon's version instead
        // of sending an empty list — ServerPoll daemons previously never received
        // any sunset warning on first contact.
        let deprecation_warnings = policy.evaluate(daemon.base.version.as_ref()).warnings;
        let server_capabilities = ServerCapabilities {
            server_version: policy.latest.clone(),
            minimum_daemon_version: policy.minimum_supported.clone(),
            deprecation_warnings,
        };

        let request = FirstContactRequest {
            daemon_id: daemon.id,
            network_id: Some(daemon.base.network_id),
            name: Some(daemon.base.name.clone()),
            server_capabilities,
        };

        self.post_to_daemon(
            daemon,
            Some(api_key),
            "/api/first-contact",
            &request,
            RetryPolicy::Standard,
        )
        .await?
        .ok_or_else(|| anyhow::anyhow!("First contact response missing daemon status"))
    }

    /// Initialize a local daemon (for integrated daemon setup)
    pub async fn initialize_local_daemon(
        &self,
        daemon_url: String,
        network_id: Uuid,
        api_key: String,
    ) -> Result<(), Error> {
        match self
            .client
            .post(format!("{}/api/initialize", daemon_url))
            .json(&InitializeDaemonRequest {
                network_id,
                api_key,
            })
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    tracing::info!("Successfully initialized daemon");
                } else {
                    let body = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "Could not read body".to_string());
                    tracing::warn!(status = %status, body = %body, "Daemon returned error");
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to reach daemon");
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn http_err(status: reqwest::StatusCode) -> anyhow::Error {
        DaemonHttpError {
            op: "GET /api/status".to_string(),
            status,
        }
        .into()
    }

    #[test]
    fn does_not_retry_4xx_daemon_responses() {
        // A 401 (key mismatch) or any 4xx is definitive — retrying can't fix it.
        assert!(!should_retry_daemon_http_error(&http_err(
            reqwest::StatusCode::UNAUTHORIZED
        )));
        assert!(!should_retry_daemon_http_error(&http_err(
            reqwest::StatusCode::FORBIDDEN
        )));
        assert!(!should_retry_daemon_http_error(&http_err(
            reqwest::StatusCode::NOT_FOUND
        )));
    }

    #[test]
    fn retries_transport_and_5xx_errors() {
        // Transport errors carry no DaemonHttpError → retry.
        assert!(should_retry_daemon_http_error(&anyhow::anyhow!(
            "connection refused"
        )));
        // 5xx is transient (server briefly down) → retry.
        assert!(should_retry_daemon_http_error(&http_err(
            reqwest::StatusCode::BAD_GATEWAY
        )));
    }
}
