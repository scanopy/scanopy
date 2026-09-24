use crate::daemon::shared::config::ConfigStore;
use crate::daemon::shared::forward_compat::DaemonResponse;
use crate::server::shared::types::api::{ApiErrorResponse, ApiResponse};
use anyhow::{Error, bail};
use reqwest::{Client, Method, RequestBuilder};
use serde::Serialize;
use std::error::Error as StdError;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::OnceCell;

/// Classified connection error types.
/// Display impl gives clean diagnostic messages (suitable for all contexts).
/// `cause_and_fix()` provides prescriptive guidance for daemon startup only.
#[derive(Debug)]
pub enum ConnectionError {
    Timeout {
        url: String,
    },
    ConnectionRefused {
        url: String,
    },
    ConnectionReset {
        url: String,
    },
    Tls {
        url: String,
    },
    /// DNS or other connect-phase failure where io::ErrorKind is Other
    DnsOrConnect {
        url: String,
        detail: String,
    },
    Other {
        url: String,
        detail: String,
    },
}

impl std::error::Error for ConnectionError {}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout { url } => write!(f, "Connection timed out to {url}"),
            Self::ConnectionRefused { url } => write!(f, "Connection refused by {url}"),
            Self::ConnectionReset { url } => write!(f, "Connection to {url} was reset"),
            Self::Tls { url } => write!(f, "TLS/certificate error connecting to {url}"),
            Self::DnsOrConnect { url, detail } => {
                write!(f, "Connection to {url} failed ({detail})")
            }
            Self::Other { url, detail } => write!(f, "Failed to connect to {url}: {detail}"),
        }
    }
}

impl ConnectionError {
    /// Prescriptive Cause/Fix guidance for daemon startup logging.
    pub fn cause_and_fix(&self) -> &'static str {
        match self {
            Self::Timeout { .. } => {
                "Cause: firewall blocking outbound traffic or server unreachable. Fix: check that your firewall allows outbound traffic to this server."
            }
            Self::ConnectionRefused { .. } => {
                "Cause: server not running or wrong URL/port. Fix: check server is running; verify URL and port."
            }
            Self::ConnectionReset { .. } => {
                "Cause: server closed the connection unexpectedly. Fix: check server logs for errors."
            }
            Self::Tls { .. } => {
                "Cause: self-signed or untrusted certificate. Fix: add --allow-self-signed-certs to the daemon command."
            }
            Self::DnsOrConnect { .. } => {
                "Cause: hostname cannot be resolved or network unreachable. Fix: check the server URL hostname and DNS/network configuration."
            }
            Self::Other { .. } => "Fix: check the server URL and network configuration.",
        }
    }
}

pub struct DaemonApiClient {
    config_store: Arc<ConfigStore>,
    client: OnceCell<Client>,
}

impl DaemonApiClient {
    pub fn new(config_store: Arc<ConfigStore>) -> Self {
        Self {
            config_store,
            client: OnceCell::new(),
        }
    }

    /// Get or lazily initialize the HTTP client
    async fn get_client(&self) -> Result<&Client, Error> {
        self.client
            .get_or_try_init(|| async {
                let allow_self_signed_certs =
                    self.config_store.get_allow_self_signed_certs().await?;

                Client::builder()
                    .danger_accept_invalid_certs(allow_self_signed_certs)
                    .connect_timeout(Duration::from_secs(10))
                    .timeout(Duration::from_secs(30))
                    .build()
                    .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {}", e))
            })
            .await
    }

    /// Build a request with standard daemon auth headers
    async fn build_request(&self, method: Method, path: &str) -> Result<RequestBuilder, Error> {
        let client = self.get_client().await?;
        let server_target = self.config_store.get_server_url().await?;
        let daemon_id = self.config_store.get_id().await?;
        let api_key = self
            .config_store
            .get_api_key()
            .await?
            .ok_or_else(|| anyhow::anyhow!("API key not set"))?;

        let url = format!("{}{}", server_target, path);

        Ok(client
            .request(method, &url)
            .header("X-Daemon-ID", daemon_id.to_string())
            .header("X-Daemon-Version", env!("CARGO_PKG_VERSION"))
            .header("Authorization", format!("Bearer {}", api_key)))
    }

    /// Check response status and handle API errors.
    /// On error responses, attempts to parse as ApiErrorResponse to preserve error codes.
    async fn check_response(
        &self,
        response: reqwest::Response,
        context: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let status = response.status();

        if !status.is_success() {
            // Try to parse as ApiErrorResponse to get error codes
            let body = response.text().await.unwrap_or_else(|_| String::new());
            if let Ok(error_response) = serde_json::from_str::<ApiErrorResponse>(&body) {
                return Err(error_response.into());
            }
            // Non-API response (HTML, plain text, etc.) — server URL is probably wrong
            let server_url = self.config_store.get_server_url().await.unwrap_or_default();
            bail!(
                "{}: Not a Scanopy server (HTTP {}). \
                 Cause: URL points to the wrong service. \
                 Fix: verify the server URL is correct. Targeting: {}",
                context,
                status,
                server_url
            );
        }

        let api_response: ApiResponse<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("{}: Failed to parse response: {}", context, e))?;

        if !api_response.is_success() {
            let error_msg = api_response
                .error()
                .map(str::to_string)
                .unwrap_or_else(|| format!("HTTP {}", status));

            bail!("{}: {}", context, error_msg);
        }

        Ok(api_response)
    }

    /// Classify a reqwest send error into a typed ConnectionError.
    /// Uses reqwest predicates and io::ErrorKind — no string matching.
    fn classify_connection_error(err: &reqwest::Error, url: &str) -> ConnectionError {
        if err.is_timeout() {
            return ConnectionError::Timeout {
                url: url.to_string(),
            };
        }

        if err.is_connect() {
            let mut source: Option<&(dyn StdError + 'static)> = err.source();
            while let Some(inner) = source {
                if let Some(io_err) = inner.downcast_ref::<std::io::Error>() {
                    return match io_err.kind() {
                        std::io::ErrorKind::ConnectionRefused => {
                            ConnectionError::ConnectionRefused {
                                url: url.to_string(),
                            }
                        }
                        std::io::ErrorKind::ConnectionReset
                        | std::io::ErrorKind::ConnectionAborted => {
                            ConnectionError::ConnectionReset {
                                url: url.to_string(),
                            }
                        }
                        _ => ConnectionError::DnsOrConnect {
                            url: url.to_string(),
                            detail: io_err.to_string(),
                        },
                    };
                }
                source = inner.source();
            }
        }

        if err.is_builder() {
            return ConnectionError::Tls {
                url: url.to_string(),
            };
        }

        ConnectionError::Other {
            url: url.to_string(),
            detail: err.to_string(),
        }
    }

    /// Execute request and parse ApiResponse, extracting data
    async fn execute<T: DaemonResponse>(
        &self,
        request: RequestBuilder,
        context: &str,
    ) -> Result<T, Error> {
        let server_url = self.config_store.get_server_url().await.unwrap_or_default();
        let response = request.send().await.map_err(|e| {
            let classified = Self::classify_connection_error(&e, &server_url);
            anyhow::Error::new(classified).context(context.to_string())
        })?;
        let api_response = self.check_response(response, context).await?;

        let data = api_response
            .into_data()
            .ok_or_else(|| anyhow::anyhow!("{}: No data in response", context))?;

        serde_json::from_value(data)
            .map_err(|e| anyhow::anyhow!("{}: Failed to parse response data: {}", context, e))
    }

    /// Execute request, check for errors, but ignore response data
    async fn execute_no_data(&self, request: RequestBuilder, context: &str) -> Result<(), Error> {
        let server_url = self.config_store.get_server_url().await.unwrap_or_default();
        let response = request.send().await.map_err(|e| {
            let classified = Self::classify_connection_error(&e, &server_url);
            anyhow::Error::new(classified).context(context.to_string())
        })?;
        self.check_response(response, context).await?;
        Ok(())
    }

    /// POST request expecting no response data
    pub async fn post_no_data<B: Serialize>(
        &self,
        path: &str,
        body: &B,
        context: &str,
    ) -> Result<(), Error> {
        let request = self.build_request(Method::POST, path).await?.json(body);
        self.execute_no_data(request, context).await
    }

    /// GET request
    pub async fn get<T: DaemonResponse>(&self, path: &str, context: &str) -> Result<T, Error> {
        let request = self.build_request(Method::GET, path).await?;
        self.execute(request, context).await
    }

    /// POST request with JSON body
    pub async fn post<B: Serialize, T: DaemonResponse>(
        &self,
        path: &str,
        body: &B,
        context: &str,
    ) -> Result<T, Error> {
        let request = self.build_request(Method::POST, path).await?.json(body);
        self.execute(request, context).await
    }

    /// Access config store for cases that need custom handling
    pub fn config(&self) -> &Arc<ConfigStore> {
        &self.config_store
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::vlans::handlers::{VlanDiscoveryResponse, VlanDiscoveryResponseItem};
    use uuid::Uuid;

    /// `execute` unwraps the `ApiResponse` envelope and hands the call site the
    /// inner `data`, so a `post`/`get` type parameter must be the INNER payload
    /// type — never `ApiResponse<Inner>`. Wrapping it made the VLAN upsert try to
    /// parse a second envelope out of `{"vlans":[...]}`, failing with
    /// `missing field 'success'` (GH #649). This pins the contract for the whole
    /// deserialize path, independent of any single call site.
    #[test]
    fn execute_unwrap_contract_uses_inner_payload_type() {
        // The real body the server sends for POST /api/v1/vlans/discovery.
        let server_body = ApiResponse::success(VlanDiscoveryResponse {
            vlans: vec![VlanDiscoveryResponseItem {
                vlan_number: 20,
                id: Uuid::nil(),
            }],
        });
        let body_json = serde_json::to_value(&server_body).unwrap();

        // Mirror `execute`: parse the envelope, then hand the call site `.data`.
        let envelope: ApiResponse<serde_json::Value> = serde_json::from_value(body_json).unwrap();
        let data = envelope.into_data().expect("server sends data");

        // Correct call-site type (the bare inner) deserializes cleanly.
        let ok: Result<VlanDiscoveryResponse, _> = serde_json::from_value(data.clone());
        assert!(
            ok.is_ok(),
            "inner payload must deserialize as VlanDiscoveryResponse"
        );
        assert_eq!(ok.unwrap().vlans[0].vlan_number, 20);

        // The old buggy double-envelope type fails exactly as the reporter saw.
        let bad: Result<ApiResponse<VlanDiscoveryResponse>, _> = serde_json::from_value(data);
        let err = bad.expect_err("double-envelope must not parse the inner payload");
        assert!(
            err.to_string().contains("missing field `success`"),
            "expected the GH #649 error, got: {err}"
        );
    }

    /// A handler never builds a failed `ApiResponse`; it returns an `ApiError`,
    /// which sends a real error status and an `ApiErrorResponse` body. The daemon
    /// parses that body into the success envelope and relies on `is_success()`
    /// and `error()` to surface the server's reason. Driven through the real
    /// `into_response`, so a field-name drift between the two types fails here
    /// instead of turning every server error into a generic "HTTP 400".
    #[tokio::test]
    async fn server_error_body_parses_into_the_envelope_with_its_message() {
        use crate::server::shared::types::api::ApiError;
        use axum::response::IntoResponse;

        let response = ApiError::bad_request("network is not on this daemon").into_response();
        assert!(response.status().is_client_error());

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let envelope: ApiResponse<serde_json::Value> = serde_json::from_slice(&body).unwrap();

        assert!(!envelope.is_success());
        assert_eq!(envelope.error(), Some("network is not on this daemon"));
        assert!(envelope.into_data().is_none());
    }
}
