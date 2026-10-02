use crate::daemon::shared::config::ConfigStore;
use crate::daemon::shared::forward_compat::DaemonResponse;
use crate::server::shared::trusted_ca::{TrustedCaBundle, is_untrusted_certificate};
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
                "Cause: the server's certificate is self-signed or signed by a CA the daemon doesn't trust. Fix: set --trusted-ca-bundle (SCANOPY_TRUSTED_CA_BUNDLE) to your CA's PEM file, or add --allow-self-signed-certs to the daemon command."
            }
            Self::DnsOrConnect { .. } => {
                "Cause: hostname cannot be resolved or network unreachable. Fix: check the server URL hostname and DNS/network configuration."
            }
            Self::Other { .. } => "Fix: check the server URL and network configuration.",
        }
    }
}

/// A non-success HTTP response from the server, classified by status and body. When the body is
/// the standard `ApiErrorResponse`, that response stays in the error chain under this one, so
/// `downcast_ref::<ApiErrorResponse>()` still finds its code.
#[derive(Debug)]
pub struct ServerResponseError {
    pub context: String,
    pub status: reqwest::StatusCode,
    /// The server's message, or the start of a non-API body.
    pub message: Option<String>,
    pub r#type: ServerResponseErrorType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerResponseErrorType {
    /// A 4xx: the server read the request and refused it. Sending the same request again gets the
    /// same answer.
    Rejected,
    /// A 5xx: the server failed handling the request.
    ServerFault,
    /// The response didn't come from Scanopy: an HTML page, or a 404 without an API body.
    NotScanopy { server_url: String },
}

impl std::error::Error for ServerResponseError {}

impl std::fmt::Display for ServerResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            context, status, ..
        } = self;
        let detail = self
            .message
            .as_deref()
            .map(|m| format!(": {m}"))
            .unwrap_or_default();
        match &self.r#type {
            ServerResponseErrorType::Rejected => write!(
                f,
                "{context}: The server rejected this payload (HTTP {status}){detail}"
            ),
            ServerResponseErrorType::ServerFault => {
                write!(f, "{context}: Server error (HTTP {status}){detail}")
            }
            ServerResponseErrorType::NotScanopy { server_url } => write!(
                f,
                "{context}: Not a Scanopy server (HTTP {status}). \
                 Cause: URL points to the wrong service. \
                 Fix: verify the server URL is correct. Targeting: {server_url}"
            ),
        }
    }
}

impl ServerResponseError {
    /// Whether the server refused the request itself, so resending it unchanged can't succeed.
    pub fn is_rejection(error: &Error) -> bool {
        error
            .downcast_ref::<Self>()
            .is_some_and(|e| e.r#type == ServerResponseErrorType::Rejected)
    }
}

/// Longest stretch of a non-API body quoted back in an error.
const MAX_BODY_EXCERPT: usize = 300;

/// Classify a non-success response. Only a response that can't have come from Scanopy is called
/// "Not a Scanopy server": every Scanopy route answers in JSON, so an HTML page or a bare 404
/// means the URL reaches something else. A 4xx with any other body is the server refusing the
/// request (axum's plain-text rejection on an older server is one), and says so with the body.
fn classify_failure(
    status: reqwest::StatusCode,
    body: &str,
    context: &str,
    server_url: &str,
) -> Error {
    let api_error = serde_json::from_str::<ApiErrorResponse>(body).ok();
    let trimmed = body.trim();
    let is_html = trimmed.starts_with('<');
    let r#type = if api_error.is_none() && (is_html || status == reqwest::StatusCode::NOT_FOUND) {
        ServerResponseErrorType::NotScanopy {
            server_url: server_url.to_string(),
        }
    } else if status.is_client_error() {
        ServerResponseErrorType::Rejected
    } else if status.is_server_error() {
        ServerResponseErrorType::ServerFault
    } else {
        ServerResponseErrorType::NotScanopy {
            server_url: server_url.to_string(),
        }
    };
    let message = match &api_error {
        Some(api_error) => api_error.error.clone(),
        None if trimmed.is_empty() || is_html => None,
        None => Some(trimmed.chars().take(MAX_BODY_EXCERPT).collect()),
    };
    let error = ServerResponseError {
        context: context.to_string(),
        status,
        message,
        r#type,
    };
    match api_error {
        Some(api_error) => Error::from(api_error).context(error),
        None => Error::new(error),
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

                let trusted_ca = self.config_store.get_trusted_ca().await;

                TrustedCaBundle::apply(
                    trusted_ca.as_deref(),
                    Client::builder()
                        .danger_accept_invalid_certs(allow_self_signed_certs)
                        .connect_timeout(Duration::from_secs(10))
                        .timeout(Duration::from_secs(30)),
                )
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
            let body = response.text().await.unwrap_or_else(|_| String::new());
            let server_url = self.config_store.get_server_url().await.unwrap_or_default();
            return Err(classify_failure(status, &body, context, &server_url));
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

        // rustls rejects the certificate inside an `io::Error` of kind InvalidData, which the
        // io-kind match below would report as a DNS/network failure.
        if is_untrusted_certificate(err) {
            return ConnectionError::Tls {
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

    fn classified(status: u16, body: &str) -> Error {
        classify_failure(
            reqwest::StatusCode::from_u16(status).unwrap(),
            body,
            "Failed to report discovery update",
            "https://scanopy.example",
        )
    }

    fn response_type(error: &Error) -> ServerResponseErrorType {
        error
            .downcast_ref::<ServerResponseError>()
            .expect("classified as a server response")
            .r#type
            .clone()
    }

    /// An older server answers a payload it can't parse with axum's plain-text 422. That is the
    /// server refusing the request, and the daemon reports the server's reason instead of sending
    /// the user to check the URL.
    #[test]
    fn a_plain_text_422_is_a_rejection_carrying_the_servers_reason() {
        let error = classified(
            422,
            "Failed to deserialize the JSON body into the target type: missing field `waited_ms`",
        );

        assert_eq!(response_type(&error), ServerResponseErrorType::Rejected);
        assert!(ServerResponseError::is_rejection(&error));
        assert!(error.to_string().contains("missing field `waited_ms`"));
    }

    /// An API error body keeps its code reachable for callers that branch on it, and its status
    /// decides whether the request is worth sending again.
    #[tokio::test]
    async fn an_api_error_body_keeps_its_code_and_is_typed_by_status() {
        use crate::server::shared::types::api::ApiError;
        use axum::response::IntoResponse;

        for (api_error, expected) in [
            (
                ApiError::bad_request("bad payload"),
                ServerResponseErrorType::Rejected,
            ),
            (
                ApiError::internal_error("database down"),
                ServerResponseErrorType::ServerFault,
            ),
        ] {
            let response = api_error.into_response();
            let status = response.status();
            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            let error = classified(status.as_u16(), std::str::from_utf8(&body).unwrap());

            assert_eq!(response_type(&error), expected);
            assert!(error.downcast_ref::<ApiErrorResponse>().is_some());
        }
    }

    /// Only a response no Scanopy route could send is called "Not a Scanopy server".
    #[test]
    fn html_or_a_bare_404_is_not_a_scanopy_server() {
        for error in [
            classified(404, ""),
            classified(404, "Not Found"),
            classified(502, "<html><body>Bad Gateway</body></html>"),
            classified(400, "<!DOCTYPE html><html></html>"),
        ] {
            assert!(matches!(
                response_type(&error),
                ServerResponseErrorType::NotScanopy { .. }
            ));
            assert!(!ServerResponseError::is_rejection(&error));
        }
    }

    /// A server certificate the daemon doesn't trust is reported as a TLS problem, not as
    /// DNS/network, so startup logging points at the CA bundle. A refused connection keeps
    /// its own classification.
    #[tokio::test]
    async fn untrusted_server_certificate_is_classified_as_tls() {
        use crate::server::shared::trusted_ca::test_support::serve_test_tls;

        let port = serve_test_tls().await;
        let url = format!("https://localhost:{port}/");
        let err = reqwest::Client::new().get(&url).send().await.unwrap_err();
        assert!(matches!(
            DaemonApiClient::classify_connection_error(&err, &url),
            ConnectionError::Tls { .. }
        ));

        let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let closed_url = format!("https://localhost:{}/", closed.local_addr().unwrap().port());
        drop(closed);
        let err = reqwest::Client::new()
            .get(&closed_url)
            .send()
            .await
            .unwrap_err();
        assert!(matches!(
            DaemonApiClient::classify_connection_error(&err, &closed_url),
            ConnectionError::ConnectionRefused { .. }
        ));
    }
}
