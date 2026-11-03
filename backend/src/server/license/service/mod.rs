use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use super::instances::service::LicenseInstanceService;
use super::key::{ConfiguredKey, LicenseKey};
use super::online::InstanceUsage;
use super::problem::LicenseProblem;
use super::types::{LicenseKeyType, LicenseStatus};
use crate::server::billing::plans::{get_community_plan, plan_for_license};
use crate::server::billing::types::base::BillingPlan;
use crate::server::organizations::service::OrganizationService;
use crate::server::shared::trusted_ca::TrustedCaBundle;

mod check_in;

/// Scanopy Cloud, which serves entitlements for online keys.
pub const CLOUD_BASE_URL: &str = "https://app.scanopy.net";

const CHECK_IN_TIMEOUT: Duration = Duration::from_secs(10);

pub struct LicenseService {
    /// The key from `SCANOPY_LICENSE_KEY`. Fixed for the life of the process.
    license_key: LicenseKey,
    key_type: ConfiguredKey,
    /// Where check-ins go. `CLOUD_BASE_URL` unless overridden for testing.
    base_url: String,
    /// This database's instance id, sent with each check-in. `None` for
    /// offline keys, and when it could not be read, in which case the cloud
    /// counts this server with the ones too old to send one.
    instance_id: Option<uuid::Uuid>,
    /// `SCANOPY_NON_PRODUCTION`, sent with each check-in so the server takes
    /// a non-production slot.
    non_production: bool,
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
    entitlement_at: Option<DateTime<Utc>>,
    /// Why the cloud rejected the online key (400/403). Cleared by the next
    /// entitlement.
    rejection: Option<String>,
    /// Why the last check-in failed, or why the key itself is unusable.
    /// Cleared by the next entitlement.
    problem: Option<LicenseProblem>,
    /// The instance count the cloud reported at the last check-in. In memory
    /// only: the check-in at startup fills it again.
    instances: Option<InstanceUsage>,
}

impl LicenseState {
    fn derive_status(&self, key: &LicenseKey, key_type: &ConfiguredKey) -> LicenseStatus {
        match key_type {
            ConfiguredKey::Offline => key.validate(),
            ConfiguredKey::Online(claims) => match (&self.rejection, &self.entitlement) {
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
        None => get_community_plan(),
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
        base_url: Option<String>,
        trusted_ca: Option<&TrustedCaBundle>,
        instance_service: Option<&LicenseInstanceService>,
        non_production: bool,
    ) -> Self {
        let key_type = license_key.key_type();

        let instance_id = match (&key_type, instance_service) {
            (ConfiguredKey::Online(_), Some(instance_service)) => {
                match instance_service.server_instance_id().await {
                    Ok(id) => Some(id),
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to read this server's instance id; check-ins will not send one");
                        None
                    }
                }
            }
            _ => None,
        };

        let (entitlement, entitlement_at) = match &key_type {
            ConfiguredKey::Offline => (None, None),
            ConfiguredKey::Online(claims) => {
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
            entitlement_at,
            rejection: None,
            problem: None,
            instances: None,
        };
        state.status = state.derive_status(&license_key, &key_type);
        // A key that fails verification takes the offline path and never checks
        // in, so this is the only place that can name the problem.
        if matches!(state.status, LicenseStatus::Invalid(_)) {
            state.problem = Some(LicenseProblem::KeyMalformed);
        }

        let base_url = base_url.unwrap_or_else(|| CLOUD_BASE_URL.to_string());
        if base_url != CLOUD_BASE_URL {
            tracing::info!(%base_url, "License check-ins are pointed at an override URL");
        }

        Self {
            license_key,
            key_type,
            base_url,
            instance_id,
            non_production,
            state: RwLock::new(state),
            organization_service,
            http: TrustedCaBundle::apply(
                trusted_ca,
                reqwest::Client::builder().timeout(CHECK_IN_TIMEOUT),
            )
            .build()
            .expect("Failed to create HTTP client"),
        }
    }

    /// Get the current license status.
    pub async fn current_status(&self) -> LicenseStatus {
        self.state.read().await.status.clone()
    }

    pub fn key_type(&self) -> LicenseKeyType {
        self.key_type.key_type()
    }

    pub fn is_online(&self) -> bool {
        matches!(self.key_type, ConfiguredKey::Online(_))
    }

    /// The id this server sends with each check-in. `None` for offline keys,
    /// or when it could not be read at startup.
    pub fn instance_id(&self) -> Option<uuid::Uuid> {
        self.instance_id
    }

    /// How many server instances the key is running on, against how many it
    /// covers, as of the last check-in. Always `None` for offline keys.
    pub async fn instance_usage(&self) -> Option<InstanceUsage> {
        self.state.read().await.instances
    }

    /// When the cloud last answered a check-in. Always `None` for offline keys.
    pub async fn entitlement_at(&self) -> Option<DateTime<Utc>> {
        self.state.read().await.entitlement_at
    }

    /// Why this server is locked, for the UI. A licensed server riding out a
    /// failed check-in on its entitlement has nothing to act on, so it reports
    /// none. The log still records that failure.
    pub async fn problem(&self) -> Option<LicenseProblem> {
        let state = self.state.read().await;
        state.status.is_locked().then_some(state.problem).flatten()
    }

    /// Where check-ins go.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// The plan the current license entitles: the licensed tier when `Valid`,
    /// otherwise Community. An invalid or expired license also
    /// locks the server via the license-guard middleware.
    pub async fn effective_plan(&self) -> BillingPlan {
        match &self.state.read().await.status {
            LicenseStatus::Valid(claims) => plan_for_license(claims),
            _ => get_community_plan(),
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
}

#[cfg(test)]
mod check_in_tests;

#[cfg(test)]
mod tests {
    use super::super::key::test_support::{ORG_ID, license_token, online_key};
    use super::super::online::{ENTITLEMENT_PATH, EntitlementResponse};
    use super::super::types::{LicenseClaims, LicensePlan};
    use super::check_in::CHECK_IN_RETRY_MIN_DELAY;
    use super::*;
    use crate::server::billing::plans::{get_self_hosted_plus_plan, get_self_hosted_standard_plan};
    use crate::server::shared::events::bus::EventBus;
    use crate::server::shared::storage::generic::GenericPostgresStorage;
    use crate::server::shared::types::api::{ApiError, ApiResponse};
    use crate::server::shared::types::error_codes::ErrorCode;
    use axum::Json;
    use axum::response::{IntoResponse, Response};
    use reqwest::StatusCode;
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
            paid_through: None,
        }
    }

    /// An org service whose database is unreachable. Loading and persisting
    /// the entitlement fail fast and are logged, which leaves only the
    /// in-memory behaviour under test.
    pub(super) fn unreachable_org_service() -> Arc<OrganizationService> {
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
        LicenseService::new(key, unreachable_org_service(), None, None, None, false).await
    }

    /// A stand-in for the cloud entitlement endpoint that answers every
    /// request with `respond()`. Returns its base URL.
    pub(super) async fn cloud(
        respond: impl Fn() -> Response + Clone + Send + Sync + 'static,
    ) -> String {
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
                instances: None,
            }))
            .into_response()
        })
        .await
    }

    pub(super) async fn entitlement_cloud(plan: LicensePlan, intended_days: i64) -> String {
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

    /// A server whose own license guard refuses the request: 403 carrying
    /// `license_locked`, which says nothing about the key we presented.
    async fn locked_cloud() -> String {
        cloud(|| ApiError::license_locked().into_response()).await
    }

    /// A base URL nothing listens on.
    pub(super) async fn unreachable_cloud() -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        format!("http://{addr}")
    }

    /// A server that boots before the cloud it points at is listening gets one
    /// failed check-in. Without a retry it would hold that status until the
    /// next interval, six hours later.
    #[tokio::test]
    async fn a_failed_first_check_in_retries_instead_of_waiting_an_interval() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let entitlement = license_token(Some(ORG_ID), Some(LicensePlan::Plus), 30);
        let attempts = Arc::new(AtomicUsize::new(0));
        let seen = attempts.clone();
        let base_url = cloud(move || {
            if seen.fetch_add(1, Ordering::SeqCst) == 0 {
                return (StatusCode::SERVICE_UNAVAILABLE, "starting up").into_response();
            }
            Json(ApiResponse::success(EntitlementResponse {
                entitlement: entitlement.clone(),
                instances: None,
            }))
            .into_response()
        })
        .await;

        let license = Arc::new(
            LicenseService::new(
                online_key(ORG_ID),
                unreachable_org_service(),
                Some(base_url),
                None,
                None,
                false,
            )
            .await,
        );
        assert!(matches!(
            license.current_status().await,
            LicenseStatus::Pending
        ));

        tokio::spawn(license.clone().run_check_ins());
        // The retry lands within CHECK_IN_RETRY_MIN_DELAY, jittered. Give it
        // twice that before calling it a regression, rather than hanging. Wait
        // on the status, not the attempt count: the cloud counts the request
        // before the service has applied its response.
        let deadline = tokio::time::Instant::now() + CHECK_IN_RETRY_MIN_DELAY * 2;
        while !matches!(license.current_status().await, LicenseStatus::Valid(_))
            && tokio::time::Instant::now() < deadline
        {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        assert!(
            matches!(license.current_status().await, LicenseStatus::Valid(_)),
            "the retried check-in should have applied the entitlement (attempts: {})",
            attempts.load(Ordering::SeqCst)
        );
    }

    /// Over the instance limit is a warning: the server keeps its entitlement
    /// and its plan, and only reports the count.
    #[tokio::test]
    async fn an_instance_over_the_limit_stays_licensed() {
        let over = InstanceUsage {
            active: 2,
            limit: Some(1),
            ..InstanceUsage::default()
        };
        let entitlement = license_token(Some(ORG_ID), Some(LicensePlan::Plus), 30);
        let base_url = cloud(move || {
            Json(ApiResponse::success(EntitlementResponse {
                entitlement: entitlement.clone(),
                instances: Some(over),
            }))
            .into_response()
        })
        .await;

        let license = service(online_key(ORG_ID)).await;
        license.check_in(&base_url).await;

        assert!(matches!(
            license.current_status().await,
            LicenseStatus::Valid(_)
        ));
        assert_eq!(license.effective_plan().await, get_self_hosted_plus_plan());
        assert_eq!(license.instance_usage().await, Some(over));
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
        assert_eq!(service.effective_plan().await, get_community_plan());
        assert!(service.entitlement_at().await.is_none());
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
        assert!(service.entitlement_at().await.is_some());

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
        let checked = service.entitlement_at().await;

        service
            .check_in(&error_cloud(StatusCode::INTERNAL_SERVER_ERROR).await)
            .await;
        service.check_in(&unreachable_cloud().await).await;
        let after = service.current_status().await;
        assert!(matches!(after, LicenseStatus::Valid(_)));
        assert_eq!(after.expiry_date(), valid.expiry_date());
        assert_eq!(service.entitlement_at().await, checked);
    }

    #[tokio::test]
    async fn a_locked_server_does_not_invalidate_the_key() {
        // A server that is itself license-locked answers 403 `license_locked`
        // from its own guard. That is not a verdict on this key, so the cached
        // entitlement stays in force and the next cycle retries.
        let service = service(online_key(ORG_ID)).await;
        service
            .check_in(&entitlement_cloud(LicensePlan::Plus, 30).await)
            .await;
        let valid = service.current_status().await;
        let checked = service.entitlement_at().await;

        service.check_in(&locked_cloud().await).await;

        let after = service.current_status().await;
        assert!(matches!(after, LicenseStatus::Valid(_)));
        assert_eq!(after.expiry_date(), valid.expiry_date());
        assert_eq!(service.entitlement_at().await, checked);
    }

    /// A server refused for running past the plan's limit is read-only and
    /// keeps the counts the cloud sent, for the banner that says why.
    #[tokio::test]
    async fn a_server_refused_for_the_instance_limit_is_locked_with_the_counts() {
        let base_url = cloud(|| {
            ApiError::coded(
                StatusCode::FORBIDDEN,
                ErrorCode::LicenseInstanceLimitReached {
                    active: 2,
                    limit: 1,
                },
            )
            .into_response()
        })
        .await;

        let license = service(online_key(ORG_ID)).await;
        license.check_in(&base_url).await;

        assert!(license.current_status().await.is_locked());
        assert_eq!(
            license.instance_usage().await,
            Some(InstanceUsage {
                active: 2,
                limit: Some(1),
                ..InstanceUsage::default()
            })
        );
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
        use crate::server::shared::services::traits::CrudService;
        use crate::server::shared::storage::traits::Storable;
        use crate::tests::test_storage;

        let (storage, _container) = test_storage().await;
        let event_bus = Arc::new(EventBus::new());
        let orgs = Arc::new(OrganizationService::new(
            storage.organizations.clone(),
            event_bus.clone(),
        ));
        let org = orgs
            .create(
                Organization::new(OrganizationBase {
                    plan: get_community_plan(),
                    ..Default::default()
                }),
                AuthenticatedEntity::System,
            )
            .await
            .unwrap();

        let license =
            LicenseService::new(online_key(ORG_ID), orgs.clone(), None, None, None, false).await;
        assert!(matches!(
            license.current_status().await,
            LicenseStatus::Pending
        ));
        license
            .check_in(&entitlement_cloud(LicensePlan::Plus, 30).await)
            .await;

        // The check-in writes the reconciled plan before it returns.
        let stored = orgs.get_by_id(&org.id).await.unwrap().unwrap();
        assert_eq!(stored.base.plan, get_self_hosted_plus_plan());
        assert!(stored.base.license_entitlement.is_some());

        // Restart with the cloud unreachable: the persisted entitlement applies.
        let restarted =
            LicenseService::new(online_key(ORG_ID), orgs.clone(), None, None, None, false).await;
        assert!(matches!(
            restarted.current_status().await,
            LicenseStatus::Valid(_)
        ));
        assert_eq!(
            restarted.effective_plan().await,
            get_self_hosted_plus_plan()
        );
        assert!(restarted.entitlement_at().await.is_some());
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
    fn valid_through_is_paid_through_else_intended_exp() {
        let without = claims(1_700_000_000, 1_800_000_000, 1_799_366_400);
        assert_eq!(
            LicenseStatus::Valid(without.clone())
                .valid_through()
                .map(|d| d.timestamp()),
            Some(1_799_366_400)
        );

        let with = LicenseClaims {
            paid_through: Some(1_798_761_600),
            ..without
        };
        assert_eq!(
            LicenseStatus::Valid(with)
                .valid_through()
                .map(|d| d.timestamp()),
            Some(1_798_761_600)
        );
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
