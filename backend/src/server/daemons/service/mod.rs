//! Daemon service for managing daemon lifecycle, processing, and polling.
//!
//! This service consolidates:
//! - CRUD operations for daemons
//! - Processing logic for daemon data (formerly in processor.rs)
//! - Polling loop for ServerPoll mode (formerly in poller.rs)
//! - HTTP client for daemon communication

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Error, Result};
use async_trait::async_trait;
use backon::{ExponentialBuilder, Retryable};
use chrono::{DateTime, Utc};
use futures::future::join_all;
use secrecy::{ExposeSecret, SecretString};
use semver::Version;
use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::daemon::runtime::state::{
    BufferedEntities, CreatedEntitiesPayload, DaemonStatus, DiscoveryPollResponse,
};
use crate::daemon::runtime::types::InitializeDaemonRequest;
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::billing::types::base::{LimitSource, LimitType};
use crate::server::credentials::r#impl::mapping::IntegrationTarget;
use crate::server::credentials::r#impl::types::CredentialTypeDiscriminants;
use crate::server::credentials::service::CredentialService;
use crate::server::daemon_api_keys::r#impl::base::{DaemonApiKey, DaemonApiKeyBase};
use crate::server::daemon_api_keys::service::DaemonApiKeyService;
use crate::server::daemons::r#impl::api::{
    DaemonDiscoveryRequest, DaemonRegistrationRequest, DaemonRegistrationResponse,
    DiscoveryUpdatePayload, FirstContactRequest, LegacyCapabilities, ProvisionDaemonRequest,
    ServerCapabilities,
};
use crate::server::daemons::r#impl::base::{Daemon, DaemonBase, DaemonMode};
use crate::server::daemons::r#impl::interfaced_subnets::DaemonInterfacedSubnetStorage;
use crate::server::daemons::r#impl::version::{
    DaemonVersionPolicy, pre_interface_to_ip_address_rename, supports_server_provisioned_identity,
    supports_unified_discovery,
};
use crate::server::discovery::r#impl::base::{Discovery, DiscoveryBase};
use crate::server::discovery::r#impl::scan_settings::ScanSettings;
use crate::server::discovery::r#impl::types::{DiscoveryType, HostNamingFallback, RunType};
use crate::server::discovery::service::DiscoveryService;
use crate::server::email::service::EmailService;
use crate::server::hosts::r#impl::base::{Host, HostBase};
use crate::server::hosts::r#impl::name::HostName;
use crate::server::hosts::r#impl::name::HostNameSources;
use crate::server::hosts::service::{HostLimitContext, HostService};
use crate::server::networks::r#impl::Network;
use crate::server::networks::service::NetworkService;
use crate::server::organizations::service::OrganizationService;
use crate::server::shared::api_key_common::{ApiKeyType, generate_api_key_for_storage};
use crate::server::shared::entities::ChangeTriggersTopologyStaleness;
use crate::server::shared::events::bus::EventBus;
use crate::server::shared::events::traits::{EntityEventFlags, EntityScope, Event, OrgScope};
use crate::server::shared::events::types::{
    BillingOperation, EntityOperation, OnboardingOperation, OnboardingOperationDiscriminants,
};
use crate::server::shared::legacy::rewrite_response_for_legacy_daemon;
use crate::server::shared::services::traits::{CrudService, EventBusService};
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::generic::GenericPostgresStorage;
use crate::server::shared::storage::traits::{Entity, Storable, Storage};
use crate::server::shared::types::api::{ApiError, ApiResponse};
use crate::server::shared::types::entities::EntitySource;
use crate::server::subnets::r#impl::base::Subnet;
use crate::server::subnets::service::SubnetService;
use crate::server::tags::entity_tags::EntityTagService;
use crate::server::users::service::UserService;

/// Weekly Sunday midnight cron schedule for default discovery jobs
const WEEKLY_SUNDAY_MIDNIGHT_CRON: &str = "0 0 0 * * 0";

/// Default polling interval in seconds
const DEFAULT_POLL_INTERVAL_SECS: u64 = 30;

/// Number of consecutive failures before marking daemon as unreachable
const UNREACHABLE_THRESHOLD: usize = 5;

/// Maximum number of concurrent daemon polls
const MAX_CONCURRENT_POLLS: usize = 10;

/// Number of days after a standby → active transition during which the
/// daily inactivity check skips the daemon. Matches the 30-day inactivity
/// window so the grace covers at least one full scheduled-discovery
/// cycle regardless of cadence.
pub const STANDBY_GRACE_PERIOD_DAYS: i64 = 30;

/// Returns true if a daemon with the given `standby_cleared_at` timestamp
/// is currently within its post-reactivation grace window. Pulled out as
/// a free function so it can be unit-tested without standing up a
/// `DaemonService`.
pub(crate) fn is_within_standby_grace(
    standby_cleared_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> bool {
    standby_cleared_at.is_some_and(|t| t > now - chrono::Duration::days(STANDBY_GRACE_PERIOD_DAYS))
}

pub struct DaemonService {
    // Storage and core dependencies
    daemon_storage: Arc<GenericPostgresStorage<Daemon>>,
    interfaced_subnet_storage: DaemonInterfacedSubnetStorage,
    client: reqwest::Client,
    event_bus: Arc<EventBus>,
    entity_tag_service: Arc<EntityTagService>,

    // Direct dependencies (passed to constructor)
    pub(super) discovery_service: Arc<DiscoveryService>,
    credential_service: Arc<CredentialService>,
    subnet_service: Arc<SubnetService>,
    network_service: Arc<NetworkService>,
    organization_service: Arc<OrganizationService>,
    user_service: Arc<UserService>,
    daemon_api_key_service: Arc<DaemonApiKeyService>,

    // Lazy dependency (set after construction to break circular dependency)
    // HostService uses DaemonService, and DaemonService uses HostService
    host_service: std::sync::OnceLock<Arc<HostService>>,

    // Polling state
    poll_semaphore: Arc<Semaphore>,

    // Deployment mode — gates the cloud-only SSRF guard on outbound daemon URLs.
    deployment_type: crate::server::config::DeploymentType,
}

impl EventBusService<Daemon> for DaemonService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, entity: &Daemon) -> Option<Uuid> {
        Some(entity.base.network_id)
    }

    fn get_organization_id(&self, _entity: &Daemon) -> Option<Uuid> {
        None
    }

    fn suppress_logs(&self, current: Option<&Daemon>, updated: Option<&Daemon>) -> bool {
        match (current, updated) {
            (Some(current), Some(updated)) => updated.suppress_logs(current),
            _ => false,
        }
    }
}

#[async_trait]
impl CrudService<Daemon> for DaemonService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Daemon>> {
        &self.daemon_storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        Some(&self.entity_tag_service)
    }

    async fn delete(
        &self,
        id: &Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<(), anyhow::Error> {
        // Clean up in-memory discovery session state (queued/pending/terminal)
        // to prevent stale references after deletion.
        self.discovery_service.clear_sessions_for_daemon(id).await;

        // Delegate to the default CrudService delete implementation
        let entity = self
            .get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("{} with id {} not found", Daemon::table_name(), id))?;

        self.storage().delete(id).await?;

        let trigger_stale = entity.triggers_staleness(None);
        let suppress_logs = self.suppress_logs(Some(&entity), None);

        if let Some(entity_tag_service) = self.entity_tag_service() {
            entity_tag_service
                .remove_all_for_entity(entity.id(), <Daemon as Entity>::entity_type())
                .await?;
        }

        if let Some(scope) = EntityScope::from_ids(
            *id,
            entity.clone().into(),
            self.get_network_id(&entity),
            self.get_organization_id(&entity),
        ) {
            self.event_bus()
                .publish(
                    Event::new(scope, EntityOperation::Deleted, authentication).with_flags(
                        EntityEventFlags {
                            trigger_stale,
                            suppress_logs,
                            ..Default::default()
                        },
                    ),
                )
                .await?;
        }

        Ok(())
    }
}

mod http;
pub(crate) use http::DaemonHttpError;
mod lifecycle;
mod monitoring;
mod polling;
mod processing;
mod provisioning;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grace_window_none_is_not_within_grace() {
        let now = Utc::now();
        assert!(!is_within_standby_grace(None, now));
    }

    #[test]
    fn grace_window_recent_clear_is_within_grace() {
        let now = Utc::now();
        let one_hour_ago = now - chrono::Duration::hours(1);
        assert!(is_within_standby_grace(Some(one_hour_ago), now));
    }

    #[test]
    fn grace_window_just_inside_boundary_is_within_grace() {
        let now = Utc::now();
        let just_inside =
            now - chrono::Duration::days(STANDBY_GRACE_PERIOD_DAYS) + chrono::Duration::hours(1);
        assert!(is_within_standby_grace(Some(just_inside), now));
    }

    #[test]
    fn grace_window_at_exact_boundary_is_expired() {
        let now = Utc::now();
        let exactly_at_boundary = now - chrono::Duration::days(STANDBY_GRACE_PERIOD_DAYS);
        // strict `>`, not `>=`
        assert!(!is_within_standby_grace(Some(exactly_at_boundary), now));
    }

    #[test]
    fn grace_window_past_boundary_is_expired() {
        let now = Utc::now();
        let past_boundary = now - chrono::Duration::days(STANDBY_GRACE_PERIOD_DAYS + 1);
        assert!(!is_within_standby_grace(Some(past_boundary), now));
    }
}
