//! Outbound calls to services the server depends on, counted by dependency and outcome.
//!
//! A wrong secret or an unreachable provider fails at the moment someone uses it, and the
//! failure reaches one user as a redirect, a toast or a log line. `http_requests_total` misses
//! it whenever the call runs outside a handler (subscribers, jobs) or the handler answers
//! without an `ErrorCode` (the OIDC callback redirects either way). This counter records every
//! outbound call at the client boundary instead:
//!
//! `scanopy_dependency_requests_total{dependency, outcome}`, and for calls made through
//! [`observe`] or [`observe_http`] their duration in
//! `scanopy_dependency_request_duration_seconds{dependency}`
//!
//! An integration gets it by sending through [`observe`] or [`observe_http`]. Each client
//! library's error type maps onto [`DependencyOutcome`] through [`ClassifyFailure`], so both
//! labels are fixed at compile time: no URL, id or provider message reaches a label.

use std::future::Future;
use std::time::Instant;

use openidconnect::core::CoreErrorResponseType;
use openidconnect::{DiscoveryError, HttpClientError, RequestTokenError, StandardErrorResponse};
use reqwest::StatusCode;
use stripe::StripeError;

use crate::server::shared::trusted_ca::is_untrusted_certificate;

/// The `dependency` label: a service outside the server that a call depends on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum Dependency {
    Oidc,
    /// The configured mail relay: Amazon SES on the cloud, the operator's relay when self-hosted.
    Smtp,
    /// Amazon SNS, which delivers SES bounce and complaint notifications.
    Sns,
    Stripe,
    Posthog,
    /// Scanopy Cloud's entitlement endpoint, called by self-hosted servers with an online key.
    LicenseServer,
    Github,
    /// The Scarf gateway in front of release downloads, called to count an MSI download.
    Scarf,
    /// The Amazon SES v2 API, which validates email addresses at signup.
    Ses,
}

/// The `outcome` label: how a call ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum DependencyOutcome {
    Ok,
    /// The provider rejected our credentials: a wrong secret, key or password.
    Auth,
    /// The provider refused this request for a reason other than credentials.
    Rejected,
    RateLimited,
    ServerError,
    Timeout,
    /// DNS, TCP or another transport failure before a response arrived.
    Connect,
    /// The TLS handshake failed, including a certificate signed by a CA the server doesn't trust.
    Tls,
    /// A response arrived but could not be parsed or verified.
    InvalidResponse,
}

impl DependencyOutcome {
    /// The outcome of an HTTP response with `status`.
    pub fn from_status(status: StatusCode) -> Self {
        match status.as_u16() {
            401 | 403 => Self::Auth,
            429 => Self::RateLimited,
            400..=499 => Self::Rejected,
            500..=599 => Self::ServerError,
            _ => Self::Ok,
        }
    }
}

/// Maps a client library's error onto a [`DependencyOutcome`].
pub trait ClassifyFailure {
    fn outcome(&self) -> DependencyOutcome;
}

/// Count one call to `dependency` that ended with `outcome`.
pub fn record(dependency: Dependency, outcome: DependencyOutcome) {
    let dependency: &'static str = dependency.into();
    let outcome: &'static str = outcome.into();
    metrics::counter!(
        "scanopy_dependency_requests_total",
        "dependency" => dependency,
        "outcome" => outcome,
    )
    .increment(1);
}

/// Count one call and record how long it took, in
/// `scanopy_dependency_request_duration_seconds{dependency}`. The outcome stays on the counter
/// only: on the histogram it would multiply every bucket.
fn record_timed(dependency: Dependency, outcome: DependencyOutcome, started: Instant) {
    record(dependency, outcome);
    let dependency: &'static str = dependency.into();
    metrics::histogram!(
        "scanopy_dependency_request_duration_seconds",
        "dependency" => dependency,
    )
    .record(started.elapsed().as_secs_f64());
}

/// Await `call`, count and time its outcome, and return its result unchanged.
pub async fn observe<T, E: ClassifyFailure>(
    dependency: Dependency,
    call: impl Future<Output = Result<T, E>>,
) -> Result<T, E> {
    let started = Instant::now();
    let result = call.await;
    let outcome = match &result {
        Ok(_) => DependencyOutcome::Ok,
        Err(e) => e.outcome(),
    };
    record_timed(dependency, outcome, started);
    result
}

/// [`observe`] for a raw reqwest send. A response counts by its status, and is returned
/// untouched so the caller keeps its own status handling. The time runs to the response
/// headers; reading the body is the caller's.
pub async fn observe_http(
    dependency: Dependency,
    call: impl Future<Output = reqwest::Result<reqwest::Response>>,
) -> reqwest::Result<reqwest::Response> {
    let started = Instant::now();
    let result = call.await;
    let outcome = match &result {
        Ok(response) => DependencyOutcome::from_status(response.status()),
        Err(e) => e.outcome(),
    };
    record_timed(dependency, outcome, started);
    result
}

/// A failed call to a dependency, for callers that pass errors on as `anyhow` and need a
/// handler further up to recognise a provider failure (`downcast_ref::<DependencyError>()`).
/// Displays as `message` alone, so logs and user-facing text read as they did before.
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct DependencyError {
    pub dependency: Dependency,
    pub outcome: DependencyOutcome,
    pub message: String,
}

impl DependencyError {
    pub fn new(dependency: Dependency, outcome: DependencyOutcome, message: String) -> Self {
        Self {
            dependency,
            outcome,
            message,
        }
    }
}

impl ClassifyFailure for reqwest::Error {
    fn outcome(&self) -> DependencyOutcome {
        if self.is_timeout() {
            DependencyOutcome::Timeout
        } else if is_untrusted_certificate(self) {
            DependencyOutcome::Tls
        } else if let Some(status) = self.status() {
            DependencyOutcome::from_status(status)
        } else if self.is_decode() || self.is_body() {
            DependencyOutcome::InvalidResponse
        } else {
            DependencyOutcome::Connect
        }
    }
}

impl ClassifyFailure for HttpClientError<reqwest::Error> {
    fn outcome(&self) -> DependencyOutcome {
        match self {
            HttpClientError::Reqwest(e) => e.outcome(),
            _ => DependencyOutcome::Connect,
        }
    }
}

/// The OIDC token endpoint. `invalid_client` is a wrong client id or secret.
impl ClassifyFailure
    for RequestTokenError<
        HttpClientError<reqwest::Error>,
        StandardErrorResponse<CoreErrorResponseType>,
    >
{
    fn outcome(&self) -> DependencyOutcome {
        match self {
            RequestTokenError::ServerResponse(response) => match response.error() {
                CoreErrorResponseType::InvalidClient
                | CoreErrorResponseType::UnauthorizedClient => DependencyOutcome::Auth,
                _ => DependencyOutcome::Rejected,
            },
            RequestTokenError::Request(e) => e.outcome(),
            RequestTokenError::Parse(..) | RequestTokenError::Other(_) => {
                DependencyOutcome::InvalidResponse
            }
        }
    }
}

impl ClassifyFailure for DiscoveryError<HttpClientError<reqwest::Error>> {
    fn outcome(&self) -> DependencyOutcome {
        match self {
            DiscoveryError::Request(e) => e.outcome(),
            DiscoveryError::Response(status, ..) => DependencyOutcome::from_status(*status),
            _ => DependencyOutcome::InvalidResponse,
        }
    }
}

impl ClassifyFailure for lettre::transport::smtp::Error {
    fn outcome(&self) -> DependencyOutcome {
        if self.is_timeout() {
            return DependencyOutcome::Timeout;
        }
        if self.is_tls() || is_untrusted_certificate(self) {
            return DependencyOutcome::Tls;
        }
        match self.status().map(u16::from) {
            // 530 authentication required, 534 mechanism too weak, 535 credentials invalid.
            Some(530 | 534 | 535) => DependencyOutcome::Auth,
            // SES answers 454 when the account's sending rate is exceeded.
            Some(454) => DependencyOutcome::RateLimited,
            _ if self.is_permanent() => DependencyOutcome::Rejected,
            _ if self.is_transient() => DependencyOutcome::ServerError,
            _ => DependencyOutcome::Connect,
        }
    }
}

impl ClassifyFailure for StripeError {
    fn outcome(&self) -> DependencyOutcome {
        match self {
            StripeError::Stripe(_, status) => StatusCode::from_u16(*status)
                .map(DependencyOutcome::from_status)
                .unwrap_or(DependencyOutcome::InvalidResponse),
            StripeError::Timeout => DependencyOutcome::Timeout,
            StripeError::ClientError(_) => DependencyOutcome::Connect,
            StripeError::JSONDeserialize(_) => DependencyOutcome::InvalidResponse,
            StripeError::ConfigError(_) => DependencyOutcome::Rejected,
        }
    }
}

/// posthog-rs reports a 401 as `Connection("Unexpected HTTP status 401: ...")`, so a wrong
/// PostHog key counts as `connect`.
impl ClassifyFailure for posthog_rs::Error {
    fn outcome(&self) -> DependencyOutcome {
        match self {
            posthog_rs::Error::RateLimit => DependencyOutcome::RateLimited,
            posthog_rs::Error::BadRequest(_) => DependencyOutcome::Rejected,
            posthog_rs::Error::ServerError { .. } => DependencyOutcome::ServerError,
            posthog_rs::Error::Connection(_) => DependencyOutcome::Connect,
            _ => DependencyOutcome::InvalidResponse,
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use axum::{Router, http::StatusCode as AxumStatus, routing::get};

    /// A recorder for the current thread, and the rendered output after the test's calls.
    /// `#[tokio::test]` runs on one thread, so calls awaited inside the test are recorded.
    pub(crate) struct TestRecorder {
        recorder: metrics_exporter_prometheus::PrometheusRecorder,
    }

    impl TestRecorder {
        pub(crate) fn new() -> Self {
            Self {
                recorder: crate::server::metrics::prometheus_builder().build_recorder(),
            }
        }

        pub(crate) fn install(&self) -> metrics::LocalRecorderGuard<'_> {
            metrics::set_default_local_recorder(&self.recorder)
        }

        pub(crate) fn count(&self, dependency: &str, outcome: &str) -> bool {
            self.recorder.handle().render().contains(&format!(
                r#"scanopy_dependency_requests_total{{dependency="{dependency}",outcome="{outcome}"}} 1"#
            ))
        }

        pub(crate) fn render(&self) -> String {
            self.recorder.handle().render()
        }
    }

    #[tokio::test]
    async fn observe_http_counts_the_status_and_leaves_the_response_readable() {
        let router = Router::new().route(
            "/",
            get(|| async { (AxumStatus::SERVICE_UNAVAILABLE, "down for maintenance") }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });

        let recorder = TestRecorder::new();
        let _guard = recorder.install();
        let response = observe_http(Dependency::Github, reqwest::get(&url))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(response.text().await.unwrap(), "down for maintenance");
        assert!(
            recorder.count("github", "server_error"),
            "{}",
            recorder.render()
        );
        assert!(
            recorder.render().contains(
                r#"scanopy_dependency_request_duration_seconds_count{dependency="github"} 1"#
            ),
            "{}",
            recorder.render()
        );
    }

    #[tokio::test]
    async fn observe_http_counts_a_refused_connection() {
        // Bind and drop, so nothing listens on the port.
        let port = {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            listener.local_addr().unwrap().port()
        };
        let recorder = TestRecorder::new();
        let _guard = recorder.install();
        let result = observe_http(
            Dependency::Sns,
            reqwest::get(format!("http://127.0.0.1:{port}/")),
        )
        .await;

        assert!(result.is_err());
        assert!(recorder.count("sns", "connect"), "{}", recorder.render());
    }

    /// The token endpoint's answer to a wrong client secret is a credentials failure, which is
    /// what the auth alert fires on; a bad authorization code is the user's request, not ours.
    #[test]
    fn invalid_client_is_a_credentials_failure_and_invalid_grant_is_not() {
        type TokenError = RequestTokenError<
            HttpClientError<reqwest::Error>,
            StandardErrorResponse<CoreErrorResponseType>,
        >;
        let response =
            |error| TokenError::ServerResponse(StandardErrorResponse::new(error, None, None));
        assert_eq!(
            response(CoreErrorResponseType::InvalidClient).outcome(),
            DependencyOutcome::Auth
        );
        assert_eq!(
            response(CoreErrorResponseType::InvalidGrant).outcome(),
            DependencyOutcome::Rejected
        );
    }
}
