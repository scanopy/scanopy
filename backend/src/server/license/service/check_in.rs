use std::sync::Arc;
use std::time::Duration;

use backon::{ExponentialBuilder, Retryable};
use chrono::{DateTime, Utc};
use rand::Rng;
use reqwest::StatusCode;

use super::super::key::{ConfiguredKey, LicenseKey};
use super::super::online::{
    ENTITLEMENT_PATH, EntitlementRequest, EntitlementResponse, InstanceUsage, SERVER_VERSION_HEADER,
};
use super::super::problem::{LicenseProblem, error_chain};
use super::super::types::LicenseStatus;
use super::LicenseService;
use crate::server::metrics::dependency::{Dependency, observe_http};
use crate::server::shared::types::api::{ApiErrorResponse, ApiResponse};
use crate::server::shared::types::error_codes::ErrorCode;

/// Base wait between online-key check-ins. Each wait adds a random delay of
/// up to `CHECK_IN_JITTER`, so instances don't check in at the same moment.
const CHECK_IN_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
const CHECK_IN_JITTER: Duration = Duration::from_secs(30 * 60);

/// Wait between attempts while the server has never had an entitlement. It is
/// read-only until one arrives, so a fixed firewall takes effect within minutes
/// rather than at the next interval.
const PENDING_CHECK_IN_INTERVAL: Duration = Duration::from_secs(5 * 60);
const PENDING_CHECK_IN_JITTER: Duration = Duration::from_secs(60);

/// Retry spacing for a check-in that reached nobody. A server that boots before
/// its network is up, or before the cloud it points at is listening, would
/// otherwise hold whatever status it started with until the next interval —
/// `Pending`, and unlicensed, on a first run.
pub(super) const CHECK_IN_RETRY_MIN_DELAY: Duration = Duration::from_secs(5);
const CHECK_IN_RETRY_MAX_DELAY: Duration = Duration::from_secs(10 * 60);
const CHECK_IN_RETRY_TIMES: usize = 6;

/// A check-in that obtained nothing. `detail` is for the log only.
#[derive(Debug, thiserror::Error)]
#[error("{}: {detail}", problem.log_message())]
pub(super) struct CheckInFailure {
    pub(super) problem: LicenseProblem,
    detail: String,
}

impl CheckInFailure {
    fn new(problem: LicenseProblem, detail: impl Into<String>) -> Self {
        Self {
            problem,
            detail: detail.into(),
        }
    }
}

impl LicenseService {
    /// Check in now, then again after each wait. Online keys only; the caller
    /// spawns this.
    pub async fn run_check_ins(self: Arc<Self>) {
        loop {
            let result = (|| self.attempt_check_in(&self.base_url))
                .retry(
                    ExponentialBuilder::default()
                        .with_min_delay(CHECK_IN_RETRY_MIN_DELAY)
                        .with_max_delay(CHECK_IN_RETRY_MAX_DELAY)
                        .with_max_times(CHECK_IN_RETRY_TIMES)
                        .with_jitter(),
                )
                .notify(|e, delay| {
                    tracing::debug!(
                        error = %e,
                        "License check-in failed; retrying in {delay:?}"
                    );
                })
                .await;

            let wait = self.next_check_in_wait().await;
            if let Err(e) = result {
                tracing::warn!(
                    url = %self.base_url,
                    next_attempt_in_minutes = wait.as_secs() / 60,
                    "License check-in retries used up: {e}"
                );
            }
            tokio::time::sleep(wait).await;
        }
    }

    /// A server waiting on its first entitlement tries again within minutes.
    /// Every other state, a refused key included, waits the full interval.
    pub(super) async fn next_check_in_wait(&self) -> Duration {
        let (base, jitter) = if matches!(self.current_status().await, LicenseStatus::Pending) {
            (PENDING_CHECK_IN_INTERVAL, PENDING_CHECK_IN_JITTER)
        } else {
            (CHECK_IN_INTERVAL, CHECK_IN_JITTER)
        };
        base + rand::rng().random_range(Duration::ZERO..=jitter)
    }

    /// Present the online key to `{base_url}{ENTITLEMENT_PATH}`.
    ///
    /// - 200 swaps in the returned entitlement.
    /// - A 400 or 403 from the license server means it rejected the key: status
    ///   becomes `Invalid`.
    /// - Anything else, including transport errors and a 400 or 403 from a proxy
    ///   in the way, leaves status unchanged; the cached entitlement stays in
    ///   force until it expires.
    pub async fn check_in(&self, base_url: &str) {
        // `attempt_check_in` has already recorded and logged the failure.
        let _ = self.attempt_check_in(base_url).await;
    }

    /// One check-in, recording why it failed. A problem is logged when it
    /// changes, so a server retrying the same failure doesn't repeat itself
    /// between the summaries `run_check_ins` writes.
    async fn attempt_check_in(&self, base_url: &str) -> Result<(), CheckInFailure> {
        let result = self.try_check_in(base_url).await;
        if let Err(failure) = &result {
            let mut state = self.state.write().await;
            if state.problem != Some(failure.problem) {
                state.problem = Some(failure.problem);
                tracing::warn!(url = %base_url, "License check-in failed: {failure}");
            }
        }
        result
    }

    /// [`LicenseService::check_in`], reporting whether the cloud answered.
    ///
    /// `Err` means nothing was obtained — a transport error, or a response that
    /// carried no usable entitlement — and the caller may retry. A rejection is
    /// `Ok`: the cloud answered, and retrying will not change its verdict.
    async fn try_check_in(&self, base_url: &str) -> Result<(), CheckInFailure> {
        if !self.is_online() {
            return Ok(());
        }

        let request = EntitlementRequest {
            key: self.license_key.as_str().to_string(),
            instance_id: self.instance_id,
            non_production: Some(self.non_production),
        };
        let send = self
            .http
            .post(format!("{base_url}{ENTITLEMENT_PATH}"))
            .header(SERVER_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
            .json(&request)
            .send();
        let response = observe_http(Dependency::LicenseServer, send)
            .await
            .map_err(|e| {
                CheckInFailure::new(LicenseProblem::from_transport(&e), error_chain(&e))
            })?;

        let http_status = response.status();
        match http_status {
            StatusCode::OK => match response
                .json::<ApiResponse<EntitlementResponse>>()
                .await
                .map(ApiResponse::into_data)
            {
                Ok(Some(data)) => {
                    self.apply_entitlement(data.entitlement, data.instances)
                        .await;
                    Ok(())
                }
                Ok(None) => Err(CheckInFailure::new(
                    LicenseProblem::ServerError,
                    "check-in returned no entitlement",
                )),
                // A 200 this server can't read did not come from the license
                // server: a proxy login or block page is the usual source.
                Err(e) => Err(CheckInFailure::new(
                    LicenseProblem::Proxy,
                    format!("check-in returned an unreadable body: {e}"),
                )),
            },
            StatusCode::BAD_REQUEST | StatusCode::FORBIDDEN => {
                // The license server always answers with this shape. Anything
                // else came from a proxy or firewall, which says nothing about
                // the key, so the entitlement must survive it.
                let Ok(body) = response.json::<ApiErrorResponse>().await else {
                    return Err(CheckInFailure::new(
                        LicenseProblem::Proxy,
                        format!(
                            "check-in returned {http_status} from something other than the license server"
                        ),
                    ));
                };
                let code = body.code.as_deref();

                // A `license_locked` 403 comes from the called server's own
                // license guard, not from a verdict on this key: that server is
                // itself locked, or the base URL points somewhere unexpected.
                // Keep the cached entitlement and retry.
                if code == Some(ErrorCode::LicenseLocked.code()) {
                    return Err(CheckInFailure::new(
                        LicenseProblem::ServerError,
                        "check-in was refused by a locked server, which is not a verdict on this key",
                    ));
                }

                let problem = LicenseProblem::from_refusal_code(code);

                // Refused because the key already runs on as many servers of
                // this server's class as the plan covers: keep the counts so
                // the UI can say so.
                let instances = (problem == LicenseProblem::InstanceLimit)
                    .then_some(body.params.as_ref())
                    .flatten()
                    .and_then(|params| {
                        let count = |key: &str| {
                            params
                                .get(key)
                                .and_then(serde_json::Value::as_u64)
                                .and_then(|n| u32::try_from(n).ok())
                        };
                        let (active, limit) = (count("active")?, Some(count("limit")?));
                        Some(if self.non_production {
                            InstanceUsage {
                                non_production_active: active,
                                non_production_limit: limit,
                                ..InstanceUsage::default()
                            }
                        } else {
                            InstanceUsage {
                                active,
                                limit,
                                ..InstanceUsage::default()
                            }
                        })
                    });

                let reason = body.error.unwrap_or_else(|| {
                    format!("License key rejected by Scanopy Cloud ({http_status})")
                });
                self.reject(reason, problem, instances).await;
                Ok(())
            }
            _ => Err(CheckInFailure::new(
                LicenseProblem::from_status(http_status),
                format!("check-in returned {http_status}"),
            )),
        }
    }

    /// Swap in an entitlement the cloud returned, persist it, and reconcile org
    /// plans to it. An entitlement that fails verification (bad signature,
    /// another org) is a bad response rather than a verdict on the key, so it
    /// leaves state unchanged.
    async fn apply_entitlement(&self, token: String, instances: Option<InstanceUsage>) {
        let ConfiguredKey::Online(claims) = &self.key_type else {
            return;
        };
        let entitlement = LicenseKey::new(token.clone());
        let new_status = entitlement.validate_entitlement(claims);
        if let LicenseStatus::Invalid(reason) = &new_status {
            tracing::warn!(%reason, "Scanopy Cloud returned an entitlement that failed verification; keeping the current entitlement");
            self.state.write().await.problem = Some(LicenseProblem::ServerError);
            return;
        }

        let checked_at = Utc::now();
        {
            let mut state = self.state.write().await;
            if state.problem.take().is_some() {
                tracing::info!(url = %self.base_url, "License check-in succeeded after earlier failures");
            }
            state.entitlement = Some(entitlement);
            state.entitlement_at = Some(checked_at);
            state.rejection = None;
            state.instances = instances;
            Self::set_status(&mut state, new_status);
        }

        self.persist(Some(token), checked_at).await;
        self.reconcile_plans().await;
    }

    /// Record that the cloud rejected the key, and drop the cached entitlement
    /// so a restart can't bring the rejected key's tier back. `instances` is
    /// set when the refusal was for running on more servers than the plan
    /// covers.
    async fn reject(
        &self,
        reason: String,
        problem: LicenseProblem,
        instances: Option<InstanceUsage>,
    ) {
        tracing::warn!(%reason, "Scanopy Cloud rejected the license key: {}", problem.log_message());
        let checked_at = Utc::now();
        {
            let mut state = self.state.write().await;
            state.entitlement = None;
            state.entitlement_at = Some(checked_at);
            state.rejection = Some(reason.clone());
            state.problem = Some(problem);
            state.instances = instances;
            Self::set_status(&mut state, LicenseStatus::Invalid(reason));
        }

        self.persist(None, checked_at).await;
    }

    async fn persist(&self, entitlement: Option<String>, checked_at: DateTime<Utc>) {
        if let Err(e) = self
            .organization_service
            .store_license_entitlement(entitlement, checked_at)
            .await
        {
            tracing::warn!(error = %e, "Failed to persist the license entitlement");
        }
    }
}
