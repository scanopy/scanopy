use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use rand::Rng;
use reqwest::StatusCode;
use tokio::sync::RwLock;

use super::key::{LicenseKey, LicenseKeyType, LicenseKeyTypeDiscriminants};
use super::online::{ENTITLEMENT_PATH, EntitlementRequest, EntitlementResponse};
use super::types::LicenseStatus;
use crate::server::billing::plans::plan_for_license;
use crate::server::billing::types::base::BillingPlan;
use crate::server::organizations::service::OrganizationService;
use crate::server::shared::types::api::{ApiErrorResponse, ApiResponse};

/// Scanopy Cloud, which serves entitlements for online keys.
pub const CLOUD_BASE_URL: &str = "https://app.scanopy.net";

/// Base wait between online-key check-ins. Each wait adds a random delay of
/// up to `CHECK_IN_JITTER`, so instances don't check in at the same moment.
const CHECK_IN_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
const CHECK_IN_JITTER: Duration = Duration::from_secs(30 * 60);
const CHECK_IN_TIMEOUT: Duration = Duration::from_secs(10);

pub struct LicenseService {
    /// The key from `SCANOPY_LICENSE_KEY`. Fixed for the life of the process.
    license_key: LicenseKey,
    key_type: LicenseKeyType,
    state: RwLock<LicenseState>,
    organization_service: Arc<OrganizationService>,
    http: reqwest::Client,
}

/// License state that changes at runtime. `revalidate` rewrites `status` on a
/// timer and check-ins swap `entitlement`, while every request reads `status`.
/// One lock keeps the fields consistent for readers.
struct LicenseState {
    status: LicenseStatus,
    /// Latest entitlement the cloud returned. Always `None` for offline keys.
    entitlement: Option<LicenseKey>,
    /// When the cloud last answered a check-in with an entitlement or a
    /// rejection.
    last_checked: Option<DateTime<Utc>>,
    /// Why the cloud rejected the online key (400/403). Cleared by the next
    /// entitlement.
    rejection: Option<String>,
}

impl LicenseState {
    fn derive_status(&self, key: &LicenseKey, key_type: &LicenseKeyType) -> LicenseStatus {
        match key_type {
            LicenseKeyType::Offline => key.validate(),
            LicenseKeyType::Online(claims) => match (&self.rejection, &self.entitlement) {
                (Some(reason), _) => LicenseStatus::Invalid(reason.clone()),
                (None, Some(entitlement)) => entitlement.validate_entitlement(claims),
                (None, None) => LicenseStatus::Pending,
            },
        }
    }
}

/// Plan a self-hosted org is entitled to: the licensed tier while the license
/// is valid, otherwise Community. `None` is a deployment with no license key.
pub async fn self_hosted_plan(license_service: Option<&LicenseService>) -> BillingPlan {
    match license_service {
        Some(license_service) => license_service.effective_plan().await,
        None => BillingPlan::default(),
    }
}

impl LicenseService {
    /// Create a license service for a configured key. Only construct one when a
    /// license key actually applies (`ServerConfig::effective_license_key` is
    /// `Some`) — a keyless deployment (community or cloud) has no
    /// `LicenseService` at all, which is what represents "licensing not
    /// required". An offline key is validated immediately. An online key starts
    /// from the entitlement its last check-in persisted, or `Pending`.
    pub async fn new(
        license_key: LicenseKey,
        organization_service: Arc<OrganizationService>,
    ) -> Self {
        let key_type = license_key.key_type();

        let (entitlement, last_checked) = match &key_type {
            LicenseKeyType::Offline => (None, None),
            LicenseKeyType::Online(claims) => {
                match organization_service.load_license_entitlement().await {
                    Ok(Some((token, checked_at))) => {
                        let entitlement = LicenseKey::new(token);
                        // A cached entitlement that no longer verifies (issued to
                        // a replaced key's org, or signed by a retired key) is
                        // discarded, and the key waits for its first check-in.
                        match entitlement.validate_entitlement(claims) {
                            LicenseStatus::Invalid(_) => (None, None),
                            _ => (Some(entitlement), checked_at),
                        }
                    }
                    Ok(None) => (None, None),
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to load the persisted license entitlement");
                        (None, None)
                    }
                }
            }
        };

        let mut state = LicenseState {
            status: LicenseStatus::Pending,
            entitlement,
            last_checked,
            rejection: None,
        };
        state.status = state.derive_status(&license_key, &key_type);

        Self {
            license_key,
            key_type,
            state: RwLock::new(state),
            organization_service,
            http: reqwest::Client::builder()
                .timeout(CHECK_IN_TIMEOUT)
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Get the current license status.
    pub async fn current_status(&self) -> LicenseStatus {
        self.state.read().await.status.clone()
    }

    pub fn key_type(&self) -> LicenseKeyTypeDiscriminants {
        LicenseKeyTypeDiscriminants::from(&self.key_type)
    }

    pub fn is_online(&self) -> bool {
        matches!(self.key_type, LicenseKeyType::Online(_))
    }

    /// When the cloud last answered a check-in. Always `None` for offline keys.
    pub async fn last_checked(&self) -> Option<DateTime<Utc>> {
        self.state.read().await.last_checked
    }

    /// The plan the current license entitles: the licensed tier when `Valid`,
    /// otherwise the Community default. An invalid or expired license also
    /// locks the server via the license-guard middleware.
    pub async fn effective_plan(&self) -> BillingPlan {
        match &self.state.read().await.status {
            LicenseStatus::Valid(claims) => plan_for_license(claims),
            _ => BillingPlan::default(),
        }
    }

    /// Recompute the status from the held key and entitlement. Called by the
    /// periodic background task to catch time-based expiry transitions
    /// without requiring a restart.
    pub async fn revalidate(&self) {
        let mut state = self.state.write().await;
        let new_status = state.derive_status(&self.license_key, &self.key_type);
        Self::set_status(&mut state, new_status);
    }

    fn set_status(state: &mut LicenseState, new_status: LicenseStatus) {
        let was_locked = state.status.is_locked();
        let now_locked = new_status.is_locked();

        if was_locked != now_locked {
            if now_locked {
                tracing::warn!(
                    target: "server",
                    "License status changed to locked: {}",
                    new_status.as_api_string()
                );
            } else {
                tracing::info!(
                    target: "server",
                    "License status changed to {}",
                    new_status.as_api_string()
                );
            }
        }

        state.status = new_status;
    }

    /// Move every org onto the tier the license entitles, emitting
    /// `LicenseReconciled` for each org that changes. Runs only while the
    /// license is `Valid`, so an expired or pending license never moves plans.
    pub async fn reconcile_plans(&self) {
        let LicenseStatus::Valid(claims) = self.current_status().await else {
            return;
        };
        if let Err(e) = self
            .organization_service
            .reconcile_self_hosted_license_plans(plan_for_license(&claims))
            .await
        {
            tracing::error!(error = %e, "Failed to reconcile self-hosted org plans to license entitlement");
        }
    }

    /// Check in now, then every `CHECK_IN_INTERVAL` plus jitter. Online keys
    /// only; the caller spawns this.
    pub async fn run_check_ins(self: Arc<Self>) {
        loop {
            self.check_in(CLOUD_BASE_URL).await;
            let jitter = rand::rng().random_range(Duration::ZERO..=CHECK_IN_JITTER);
            tokio::time::sleep(CHECK_IN_INTERVAL + jitter).await;
        }
    }

    /// Present the online key to `{base_url}{ENTITLEMENT_PATH}`.
    ///
    /// - 200 swaps in the returned entitlement.
    /// - 400 or 403 means the cloud rejected the key: status becomes `Invalid`.
    /// - Anything else, including transport errors, leaves status unchanged;
    ///   the cached entitlement stays in force until it expires.
    pub async fn check_in(&self, base_url: &str) {
        if !self.is_online() {
            return;
        }

        let request = EntitlementRequest {
            key: self.license_key.as_str().to_string(),
        };
        let response = match self
            .http
            .post(format!("{base_url}{ENTITLEMENT_PATH}"))
            .json(&request)
            .send()
            .await
        {
            Ok(response) => response,
            Err(e) => {
                tracing::debug!(error = %e, "License check-in failed; keeping the current entitlement");
                return;
            }
        };

        let http_status = response.status();
        match http_status {
            StatusCode::OK => match response.json::<ApiResponse<EntitlementResponse>>().await {
                Ok(ApiResponse {
                    data: Some(data), ..
                }) => self.apply_entitlement(data.entitlement).await,
                Ok(_) => tracing::debug!(
                    "License check-in returned no entitlement; keeping the current entitlement"
                ),
                Err(e) => tracing::debug!(
                    error = %e,
                    "License check-in returned an unreadable body; keeping the current entitlement"
                ),
            },
            StatusCode::BAD_REQUEST | StatusCode::FORBIDDEN => {
                let reason = response
                    .json::<ApiErrorResponse>()
                    .await
                    .ok()
                    .and_then(|body| body.error)
                    .unwrap_or_else(|| {
                        format!("License key rejected by Scanopy Cloud ({http_status})")
                    });
                self.reject(reason).await;
            }
            _ => tracing::debug!(
                status = %http_status,
                "License check-in failed; keeping the current entitlement"
            ),
        }
    }

    /// Swap in an entitlement the cloud returned, persist it, and reconcile org
    /// plans to it. An entitlement that fails verification (bad signature,
    /// another org) is a bad response rather than a verdict on the key, so it
    /// leaves state unchanged.
    async fn apply_entitlement(&self, token: String) {
        let LicenseKeyType::Online(claims) = &self.key_type else {
            return;
        };
        let entitlement = LicenseKey::new(token.clone());
        let new_status = entitlement.validate_entitlement(claims);
        if let LicenseStatus::Invalid(reason) = &new_status {
            tracing::warn!(%reason, "Scanopy Cloud returned an entitlement that failed verification; keeping the current entitlement");
            return;
        }

        let checked_at = Utc::now();
        {
            let mut state = self.state.write().await;
            state.entitlement = Some(entitlement);
            state.last_checked = Some(checked_at);
            state.rejection = None;
            Self::set_status(&mut state, new_status);
        }

        self.persist(Some(token), checked_at).await;
        self.reconcile_plans().await;
    }

    /// Record that the cloud rejected the key, and drop the cached entitlement
    /// so a restart can't bring the rejected key's tier back.
    async fn reject(&self, reason: String) {
        tracing::warn!(%reason, "Scanopy Cloud rejected the license key");
        let checked_at = Utc::now();
        {
            let mut state = self.state.write().await;
            state.entitlement = None;
            state.last_checked = Some(checked_at);
            state.rejection = Some(reason.clone());
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

#[cfg(test)]
mod tests {
    use super::super::key::test_support::{ORG_ID, license_token, online_key};
    use super::super::types::{LicenseClaims, LicensePlan};
    use super::*;
    use crate::server::billing::plans::{get_self_hosted_plus_plan, get_self_hosted_standard_plan};
    use crate::server::shared::events::bus::EventBus;
    use crate::server::shared::storage::generic::GenericPostgresStorage;
    use crate::server::shared::types::api::ApiError;
    use axum::Json;
    use axum::response::{IntoResponse, Response};
    use sqlx::postgres::PgPoolOptions;

    fn claims(iat: i64, exp: i64, intended_exp: i64) -> LicenseClaims {
        LicenseClaims {
            sub: "scanopy-license".to_string(),
            iss: "scanopy".to_string(),
            iat,
            exp,
            intended_exp,
            org_id: None,
            plan: None,
        }
    }

    /// An org service whose database is unreachable. Loading and persisting
    /// the entitlement fail fast and are logged, which leaves only the
    /// in-memory behaviour under test.
    fn unreachable_org_service() -> Arc<OrganizationService> {
        let pool = PgPoolOptions::new()
            .acquire_timeout(Duration::from_millis(100))
            .connect_lazy("postgres://scanopy@127.0.0.1:1/unreachable")
            .expect("lazy pool builds without connecting");
        Arc::new(OrganizationService::new(
            Arc::new(GenericPostgresStorage::new(pool)),
            Arc::new(EventBus::new()),
        ))
    }

    async fn service(key: LicenseKey) -> LicenseService {
        LicenseService::new(key, unreachable_org_service()).await
    }

    /// A stand-in for the cloud entitlement endpoint that answers every
    /// request with `respond()`. Returns its base URL.
    async fn cloud(respond: impl Fn() -> Response + Clone + Send + Sync + 'static) -> String {
        let app = axum::Router::new().route(
            ENTITLEMENT_PATH,
            axum::routing::post(move || {
                let response = respond();
                async move { response }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{addr}")
    }

    async fn entitlement_cloud_for(org_id: &str, plan: LicensePlan, intended_days: i64) -> String {
        let entitlement = license_token(Some(org_id), Some(plan), intended_days);
        cloud(move || {
            Json(ApiResponse::success(EntitlementResponse {
                entitlement: entitlement.clone(),
            }))
            .into_response()
        })
        .await
    }

    async fn entitlement_cloud(plan: LicensePlan, intended_days: i64) -> String {
        entitlement_cloud_for(ORG_ID, plan, intended_days).await
    }

    async fn error_cloud(status: StatusCode) -> String {
        cloud(move || match status {
            StatusCode::FORBIDDEN => {
                ApiError::forbidden("License key version retired").into_response()
            }
            StatusCode::BAD_REQUEST => {
                ApiError::bad_request("Malformed license key").into_response()
            }
            _ => (status, "unavailable").into_response(),
        })
        .await
    }

    /// A base URL nothing listens on.
    async fn unreachable_cloud() -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        format!("http://{addr}")
    }

    #[tokio::test]
    async fn garbage_key_is_invalid() {
        // A key is present but does not verify => commercial deployment, locked.
        let service = service(LicenseKey::new("not-a-jwt".to_string())).await;
        let status = service.current_status().await;
        assert!(status.is_locked());
        assert_eq!(status.as_api_string(), "invalid");
    }

    #[tokio::test]
    async fn offline_key_ignores_check_in() {
        let service = service(LicenseKey::new(license_token(
            None,
            Some(LicensePlan::Standard),
            30,
        )))
        .await;
        service
            .check_in(&error_cloud(StatusCode::FORBIDDEN).await)
            .await;
        assert!(matches!(
            service.current_status().await,
            LicenseStatus::Valid(_)
        ));
        assert_eq!(
            service.effective_plan().await,
            get_self_hosted_standard_plan()
        );
    }

    #[tokio::test]
    async fn online_key_without_entitlement_is_pending_and_locked() {
        let service = service(online_key(ORG_ID)).await;
        let status = service.current_status().await;
        assert!(matches!(status, LicenseStatus::Pending));
        assert!(status.is_locked());
        assert_eq!(service.effective_plan().await, BillingPlan::default());
        assert!(service.last_checked().await.is_none());
    }

    #[tokio::test]
    async fn check_in_swaps_entitlement_without_restart() {
        let service = service(online_key(ORG_ID)).await;

        service
            .check_in(&entitlement_cloud(LicensePlan::Standard, 30).await)
            .await;
        let first = service.current_status().await;
        assert!(matches!(first, LicenseStatus::Valid(_)));
        assert_eq!(
            service.effective_plan().await,
            get_self_hosted_standard_plan()
        );
        assert!(service.last_checked().await.is_some());

        // A renewal and a tier change in the cloud arrive on the next check-in.
        service
            .check_in(&entitlement_cloud(LicensePlan::Plus, 365).await)
            .await;
        let second = service.current_status().await;
        assert_eq!(service.effective_plan().await, get_self_hosted_plus_plan());
        assert!(second.intended_expiry_date() > first.intended_expiry_date());
    }

    #[tokio::test]
    async fn failed_check_ins_leave_status_unchanged() {
        let service = service(online_key(ORG_ID)).await;
        service
            .check_in(&error_cloud(StatusCode::SERVICE_UNAVAILABLE).await)
            .await;
        service.check_in(&unreachable_cloud().await).await;
        assert!(matches!(
            service.current_status().await,
            LicenseStatus::Pending
        ));

        service
            .check_in(&entitlement_cloud(LicensePlan::Plus, 30).await)
            .await;
        let valid = service.current_status().await;
        let checked = service.last_checked().await;

        service
            .check_in(&error_cloud(StatusCode::INTERNAL_SERVER_ERROR).await)
            .await;
        service.check_in(&unreachable_cloud().await).await;
        let after = service.current_status().await;
        assert!(matches!(after, LicenseStatus::Valid(_)));
        assert_eq!(after.expiry_date(), valid.expiry_date());
        assert_eq!(service.last_checked().await, checked);
    }

    #[tokio::test]
    async fn rejected_key_is_invalid_until_the_cloud_returns_an_entitlement() {
        for rejection in [StatusCode::FORBIDDEN, StatusCode::BAD_REQUEST] {
            let service = service(online_key(ORG_ID)).await;
            service
                .check_in(&entitlement_cloud(LicensePlan::Plus, 30).await)
                .await;

            service.check_in(&error_cloud(rejection).await).await;
            let status = service.current_status().await;
            assert!(matches!(status, LicenseStatus::Invalid(_)), "{rejection}");
            assert!(status.is_locked());

            // Revalidation keeps the rejection; only a new entitlement clears it.
            service.revalidate().await;
            assert!(service.current_status().await.is_locked());
            service
                .check_in(&entitlement_cloud(LicensePlan::Plus, 30).await)
                .await;
            assert!(matches!(
                service.current_status().await,
                LicenseStatus::Valid(_)
            ));
        }
    }

    #[tokio::test]
    async fn entitlement_for_another_org_leaves_status_unchanged() {
        let service = service(online_key(ORG_ID)).await;
        service
            .check_in(&entitlement_cloud(LicensePlan::Standard, 30).await)
            .await;
        let before = service.current_status().await;

        service
            .check_in(
                &entitlement_cloud_for(
                    "5d2b3c4e-0000-4000-8000-000000000000",
                    LicensePlan::Plus,
                    365,
                )
                .await,
            )
            .await;
        let after = service.current_status().await;
        assert!(matches!(after, LicenseStatus::Valid(_)));
        assert_eq!(after.expiry_date(), before.expiry_date());
        assert_eq!(
            service.effective_plan().await,
            get_self_hosted_standard_plan()
        );
    }

    #[tokio::test]
    async fn entitlement_swap_reconciles_plans_and_survives_restart() {
        use crate::server::auth::middleware::auth::AuthenticatedEntity;
        use crate::server::organizations::r#impl::base::{Organization, OrganizationBase};
        use crate::server::shared::events::traits::Subscriber;
        use crate::server::shared::events::types::BillingOperation;
        use crate::server::shared::services::traits::CrudService;
        use crate::server::shared::storage::traits::Storable;
        use crate::tests::test_storage;

        let (storage, _container) = test_storage().await;
        let event_bus = Arc::new(EventBus::new());
        let orgs = Arc::new(OrganizationService::new(
            storage.organizations.clone(),
            event_bus.clone(),
        ));
        let subscriber: Arc<dyn Subscriber<BillingOperation>> = orgs.clone();
        event_bus.register(subscriber, "organization_service").await;
        let org = orgs
            .create(
                Organization::new(OrganizationBase {
                    plan: Some(BillingPlan::default()),
                    ..Default::default()
                }),
                AuthenticatedEntity::System,
            )
            .await
            .unwrap();

        let license = LicenseService::new(online_key(ORG_ID), orgs.clone()).await;
        assert!(matches!(
            license.current_status().await,
            LicenseStatus::Pending
        ));
        license
            .check_in(&entitlement_cloud(LicensePlan::Plus, 30).await)
            .await;

        // The org subscriber applies `LicenseReconciled` from its event batch.
        let mut stored = orgs.get_by_id(&org.id).await.unwrap().unwrap();
        for _ in 0..100 {
            if stored.base.plan == Some(get_self_hosted_plus_plan()) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
            stored = orgs.get_by_id(&org.id).await.unwrap().unwrap();
        }
        assert_eq!(stored.base.plan, Some(get_self_hosted_plus_plan()));
        assert!(stored.base.license_entitlement.is_some());

        // Restart with the cloud unreachable: the persisted entitlement applies.
        let restarted = LicenseService::new(online_key(ORG_ID), orgs.clone()).await;
        assert!(matches!(
            restarted.current_status().await,
            LicenseStatus::Valid(_)
        ));
        assert_eq!(
            restarted.effective_plan().await,
            get_self_hosted_plus_plan()
        );
        assert!(restarted.last_checked().await.is_some());
    }

    #[test]
    fn license_claims_json_roundtrip_preserves_intended_exp() {
        let original = claims(1_700_000_000, 1_800_000_000, 1_799_395_200);
        let encoded = serde_json::to_string(&original).unwrap();
        let decoded: LicenseClaims = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.exp, original.exp);
        assert_eq!(decoded.intended_exp, 1_799_395_200);
    }

    #[test]
    fn license_claims_rejects_missing_intended_exp() {
        let json_missing_intended_exp = r#"{
            "sub": "scanopy-license",
            "iss": "scanopy",
            "iat": 1700000000,
            "exp": 1800000000
        }"#;
        assert!(serde_json::from_str::<LicenseClaims>(json_missing_intended_exp).is_err());
    }

    #[test]
    fn in_grace_period_true_between_intended_and_hard() {
        let now = 1_700_000_000;
        let status = LicenseStatus::Valid(claims(
            now - 86_400 * 30,
            now + 86_400 * 6, // hard exp: 6 days from now
            now - 86_400,     // intended exp: 1 day ago
        ));
        assert!(status.in_grace_period_at(now));
    }

    #[test]
    fn in_grace_period_false_before_intended_expiry() {
        let now = 1_700_000_000;
        let status =
            LicenseStatus::Valid(claims(now - 86_400, now + 86_400 * 372, now + 86_400 * 365));
        assert!(!status.in_grace_period_at(now));
    }

    #[test]
    fn in_grace_period_false_after_hard_expiry() {
        let now = 1_700_000_000;
        // Even if status happens to be Valid at construction time, the
        // grace window ends at `exp`.
        let status = LicenseStatus::Valid(claims(
            now - 86_400 * 400,
            now - 86_400,     // hard exp: 1 day ago
            now - 86_400 * 8, // intended exp: 8 days ago
        ));
        assert!(!status.in_grace_period_at(now));
    }

    #[test]
    fn in_grace_period_false_when_expired_variant() {
        let now = 1_700_000_000;
        let status =
            LicenseStatus::Expired(claims(now - 86_400 * 400, now - 86_400, now - 86_400 * 8));
        assert!(!status.in_grace_period_at(now));
    }

    #[test]
    fn intended_expiry_date_uses_intended_exp_not_hard_exp() {
        let now = 1_700_000_000;
        let status =
            LicenseStatus::Valid(claims(now - 86_400, now + 86_400 * 372, now + 86_400 * 365));
        assert_ne!(status.intended_expiry_date(), status.expiry_date());
    }
}
