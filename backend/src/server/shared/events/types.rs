use crate::{
    daemon::discovery::types::base::DiscoveryPhase,
    server::{
        auth::middleware::auth::AuthenticatedEntity, daemons::r#impl::api::DiscoveryUpdatePayload,
        discovery::r#impl::types::DiscoveryType, shared::entities::Entity,
    },
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{fmt::Display, net::IpAddr};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub enum Event {
    Entity(Box<EntityEvent>),
    Auth(AuthEvent),
    Billing(BillingEvent),
    Onboarding(OnboardingEvent),
    Discovery(DiscoverySessionEvent),
    Analytics(AnalyticsEvent),
}

#[derive(Debug, Clone, Serialize)]
pub enum EventOperation {
    EntityOperation(EntityOperation),
    AuthOperation(AuthOperation),
    BillingOperation(BillingOperation),
    OnboardingOperation(OnboardingOperation),
    DiscoveryOperation(DiscoveryPhase),
    AnalyticsOperation(AnalyticsOperation),
}

#[derive(Debug, Clone, Serialize)]
pub enum EventLogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl EventOperation {
    pub fn log_level(&self) -> EventLogLevel {
        match self {
            EventOperation::EntityOperation(entity_operation) => entity_operation.log_level(),
            EventOperation::AuthOperation(auth_operation) => auth_operation.log_level(),
            EventOperation::BillingOperation(op) => op.log_level(),
            EventOperation::OnboardingOperation(op) => op.log_level(),
            EventOperation::DiscoveryOperation(phase) => phase.log_level(),
            EventOperation::AnalyticsOperation(op) => op.log_level(),
        }
    }
}

impl Display for EventOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            EventOperation::EntityOperation(entity_operation) => entity_operation.to_string(),
            EventOperation::AuthOperation(auth_operation) => auth_operation.to_string(),
            EventOperation::BillingOperation(op) => op.to_string(),
            EventOperation::OnboardingOperation(op) => op.to_string(),
            EventOperation::DiscoveryOperation(phase) => phase.to_string(),
            EventOperation::AnalyticsOperation(op) => op.to_string(),
        };

        write!(f, "{}", string)
    }
}

impl From<EntityOperation> for EventOperation {
    fn from(value: EntityOperation) -> Self {
        Self::EntityOperation(value)
    }
}

impl From<AuthOperation> for EventOperation {
    fn from(value: AuthOperation) -> Self {
        Self::AuthOperation(value)
    }
}

impl From<BillingOperation> for EventOperation {
    fn from(value: BillingOperation) -> Self {
        Self::BillingOperation(value)
    }
}

impl From<OnboardingOperation> for EventOperation {
    fn from(value: OnboardingOperation) -> Self {
        Self::OnboardingOperation(value)
    }
}

impl From<DiscoveryPhase> for EventOperation {
    fn from(value: DiscoveryPhase) -> Self {
        Self::DiscoveryOperation(value)
    }
}

impl From<AnalyticsOperation> for EventOperation {
    fn from(value: AnalyticsOperation) -> Self {
        Self::AnalyticsOperation(value)
    }
}

impl Event {
    pub fn id(&self) -> Uuid {
        match self {
            Event::Auth(a) => a.id,
            Event::Entity(e) => e.id,
            Event::Billing(b) => b.id,
            Event::Onboarding(o) => o.id,
            Event::Discovery(d) => d.id,
            Event::Analytics(a) => a.id,
        }
    }

    pub fn org_id(&self) -> Option<Uuid> {
        match self {
            Event::Auth(a) => a.organization_id,
            Event::Entity(e) => e.organization_id,
            Event::Billing(b) => Some(b.organization_id),
            Event::Onboarding(o) => Some(o.organization_id),
            Event::Discovery(_) => None,
            Event::Analytics(a) => Some(a.organization_id),
        }
    }

    pub fn network_id(&self) -> Option<Uuid> {
        match self {
            Event::Auth(_) => None,
            Event::Entity(e) => e.network_id,
            Event::Billing(_) => None,
            Event::Onboarding(_) => None,
            Event::Discovery(d) => Some(d.network_id),
            Event::Analytics(_) => None,
        }
    }

    pub fn metadata(&self) -> serde_json::Value {
        match self {
            Event::Auth(e) => e.metadata.clone(),
            Event::Entity(e) => e.metadata.clone(),
            Event::Billing(e) => e.metadata.clone(),
            Event::Onboarding(e) => e.metadata.clone(),
            Event::Discovery(d) => d.metadata.clone(),
            Event::Analytics(a) => a.metadata.clone(),
        }
    }

    pub fn authentication(&self) -> AuthenticatedEntity {
        match self {
            Event::Auth(e) => e.authentication.clone(),
            Event::Entity(e) => e.authentication.clone(),
            Event::Billing(e) => e.authentication.clone(),
            Event::Onboarding(e) => e.authentication.clone(),
            Event::Discovery(d) => d.authentication.clone(),
            Event::Analytics(a) => a.authentication.clone(),
        }
    }

    pub fn operation(&self) -> EventOperation {
        match self {
            Event::Auth(e) => e.operation.clone().into(),
            Event::Entity(e) => e.operation.clone().into(),
            Event::Billing(e) => e.operation.clone().into(),
            Event::Onboarding(e) => e.operation.clone().into(),
            Event::Discovery(d) => d.phase.into(),
            Event::Analytics(a) => a.operation.clone().into(),
        }
    }

    pub fn log(&self) {
        match self {
            Event::Entity(event) => {
                let network_id_str = event
                    .network_id
                    .map(|n| n.to_string())
                    .unwrap_or("N/A".to_string());
                let org_id_str = event
                    .organization_id
                    .map(|n| n.to_string())
                    .unwrap_or("N/A".to_string());

                tracing::info!(
                    entity_type = %event.entity_type,
                    entity_id = %event.entity_id,
                    network_id = %network_id_str,
                    organization_id = %org_id_str,
                    operation = %event.operation,
                );
            }
            Event::Auth(event) => {
                let user_id_str = event
                    .user_id
                    .map(|n| n.to_string())
                    .unwrap_or("N/A".to_string());
                let user_agent_str = event
                    .user_agent
                    .as_ref()
                    .map(|u| u.to_owned())
                    .unwrap_or("unknown".to_string());
                let org_id_str = event
                    .organization_id
                    .map(|u| u.to_string())
                    .unwrap_or("None".to_string());

                tracing::info!(
                    ip = %event.ip_address,
                    organization_id = %org_id_str,
                    user_id = %user_id_str,
                    user_agent = %user_agent_str,
                    operation = %event.operation,
                );
            }
            Event::Billing(event) => {
                tracing::info!(
                    organization_id = %event.organization_id,
                    operation = %event.operation,
                );
            }
            Event::Onboarding(event) => {
                tracing::info!(
                    organization_id = %event.organization_id,
                    operation = %event.operation,
                );
            }
            Event::Discovery(event) => {
                tracing::info!(
                    phase = %event.phase,
                    session_id = %event.session_id
                )
            }
            Event::Analytics(event) => {
                tracing::info!(
                    organization_id = %event.organization_id,
                    operation = %event.operation,
                );
            }
        }
    }
}

impl Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::Auth(a) => write!(f, "{a}"),
            Event::Entity(e) => write!(f, "{e}"),
            Event::Billing(b) => write!(f, "{b}"),
            Event::Onboarding(o) => write!(f, "{o}"),
            Event::Discovery(d) => write!(f, "{d}"),
            Event::Analytics(a) => write!(f, "{a}"),
        }
    }
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Event::Auth(a1), Event::Auth(a2)) => a1 == a2,
            (Event::Entity(e1), Event::Entity(e2)) => e1 == e2,
            (Event::Analytics(a1), Event::Analytics(a2)) => a1 == a2,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum AuthOperation {
    // User Auth
    Register,
    LoginSuccess,
    LoginFailed,
    PasswordResetRequested,
    PasswordResetCompleted,
    PasswordChanged,
    EmailVerified,
    OidcLinked,
    OidcUnlinked,
    EmailChangeRequested,
    EmailChanged,
    LoggedOut,

    // Api Key Auth
    RotateKey,
    ApiKeyAuthFailed,
}

impl AuthOperation {
    fn log_level(&self) -> EventLogLevel {
        match self {
            AuthOperation::LoginFailed | AuthOperation::ApiKeyAuthFailed => EventLogLevel::Warn,
            _ => EventLogLevel::Info,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthEvent {
    pub id: Uuid,
    pub user_id: Option<Uuid>, // None for failed login with unknown user
    pub organization_id: Option<Uuid>,
    pub operation: AuthOperation,
    pub timestamp: DateTime<Utc>,
    pub ip_address: IpAddr,
    pub user_agent: Option<String>,
    pub metadata: serde_json::Value,
    pub authentication: AuthenticatedEntity,
}

impl AuthEvent {
    /// Create a new AuthEvent, automatically deriving auth_method from authentication
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: Uuid,
        user_id: Option<Uuid>,
        organization_id: Option<Uuid>,
        operation: AuthOperation,
        timestamp: DateTime<Utc>,
        ip_address: IpAddr,
        user_agent: Option<String>,
        metadata: serde_json::Value,
        authentication: AuthenticatedEntity,
    ) -> Self {
        Self {
            id,
            user_id,
            organization_id,
            operation,
            timestamp,
            ip_address,
            user_agent,
            metadata,
            authentication,
        }
    }
}

impl PartialEq for AuthEvent {
    fn eq(&self, other: &Self) -> bool {
        self.user_id == other.user_id
            && self.organization_id == other.organization_id
            && self.operation == other.operation
            && self.ip_address == other.ip_address
            && self.user_agent == other.user_agent
            && self.metadata == other.metadata
            && self.authentication == other.authentication
    }
}

impl Display for AuthEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ id: {}, operation: {}, ip: {}, user_agent: {}, authentication: {} }}",
            self.id,
            self.operation,
            self.ip_address,
            self.user_agent.clone().unwrap_or("unknown".to_string()),
            self.authentication
        )
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum EntityOperation {
    Get,
    GetAll,
    Created,
    Updated,
    Deleted,
}

impl EntityOperation {
    fn log_level(&self) -> EventLogLevel {
        EventLogLevel::Info
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EntityEvent {
    pub id: Uuid,
    pub entity_type: Entity,
    pub entity_id: Uuid,
    pub network_id: Option<Uuid>, // Some entities might belong to an org, not a network (ie users)
    pub organization_id: Option<Uuid>, // Some entities might belong to a network, not an org
    pub operation: EntityOperation,
    pub timestamp: DateTime<Utc>,
    pub authentication: AuthenticatedEntity,
    pub metadata: serde_json::Value,
}

impl EntityEvent {
    /// Create a new EntityEvent, automatically deriving auth_method from authentication
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: Uuid,
        entity_type: Entity,
        entity_id: Uuid,
        network_id: Option<Uuid>,
        organization_id: Option<Uuid>,
        operation: EntityOperation,
        timestamp: DateTime<Utc>,
        authentication: AuthenticatedEntity,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            id,
            entity_type,
            entity_id,
            network_id,
            organization_id,
            operation,
            timestamp,
            authentication,
            metadata,
        }
    }
}

impl PartialEq for EntityEvent {
    fn eq(&self, other: &Self) -> bool {
        self.entity_id == other.entity_id
            && self.network_id == other.network_id
            && self.organization_id == other.organization_id
            && self.operation == other.operation
            && self.authentication == other.authentication
            && self.metadata == other.metadata
    }
}

impl Display for EntityEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ id: {}, entity_type: {}, entity_id: {}, operation: {} }}",
            self.id, self.entity_type, self.entity_id, self.operation
        )
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash, strum::Display, utoipa::ToSchema)]
#[strum(serialize_all = "snake_case")]
pub enum BillingOperation {
    CheckoutStarted,
    CheckoutCompleted,
    TrialStarted,
    TrialEnded,
    TrialWillEnd,
    SubscriptionCancelled,
    PlanChanged,
    PaymentFailed,
    PaymentActionRequired,
    PaymentRecovered,
    FeatureLimitHit,
}

impl BillingOperation {
    fn log_level(&self) -> EventLogLevel {
        EventLogLevel::Info
    }
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, strum::Display, utoipa::ToSchema,
)]
#[strum(serialize_all = "snake_case")]
pub enum OnboardingOperation {
    OrgCreated,
    OnboardingModalCompleted,
    PlanSelected,
    FirstDaemonRegistered,
    FirstTopologyRebuild,
    FirstDiscoveryCompleted,
    FirstHostDiscovered,
    SecondNetworkCreated,
    FirstTagCreated,
    FirstGroupCreated,
    FirstUserApiKeyCreated,
    FirstSnmpCredentialCreated,
    FirstCredentialCreated,
    InviteSent,
    InviteAccepted,
    ProfileCompleted,
    ReferralSourceCompleted,
}

impl OnboardingOperation {
    fn log_level(&self) -> EventLogLevel {
        EventLogLevel::Info
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BillingEvent {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub operation: BillingOperation,
    pub timestamp: DateTime<Utc>,
    pub authentication: AuthenticatedEntity,
    pub metadata: serde_json::Value,
}

impl BillingEvent {
    pub fn new(
        id: Uuid,
        organization_id: Uuid,
        operation: BillingOperation,
        timestamp: DateTime<Utc>,
        authentication: AuthenticatedEntity,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            id,
            organization_id,
            operation,
            timestamp,
            authentication,
            metadata,
        }
    }
}

impl Display for BillingEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ id: {}, organization_id: {}, operation: {}, authentication: {} }}",
            self.id, self.organization_id, self.operation, self.authentication
        )
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OnboardingEvent {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub operation: OnboardingOperation,
    pub timestamp: DateTime<Utc>,
    pub authentication: AuthenticatedEntity,
    pub metadata: serde_json::Value,
}

impl OnboardingEvent {
    pub fn new(
        id: Uuid,
        organization_id: Uuid,
        operation: OnboardingOperation,
        timestamp: DateTime<Utc>,
        authentication: AuthenticatedEntity,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            id,
            organization_id,
            operation,
            timestamp,
            authentication,
            metadata,
        }
    }
}

impl Display for OnboardingEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ id: {}, organization_id: {}, operation: {}, authentication: {} }}",
            self.id, self.organization_id, self.operation, self.authentication
        )
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum AnalyticsOperation {
    TopologyShareViewed,
    TopologyEmbedViewed,
}

impl AnalyticsOperation {
    fn log_level(&self) -> EventLogLevel {
        EventLogLevel::Debug
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AnalyticsEvent {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub operation: AnalyticsOperation,
    pub timestamp: DateTime<Utc>,
    pub authentication: AuthenticatedEntity,
    pub metadata: serde_json::Value,
}

impl AnalyticsEvent {
    pub fn new(
        id: Uuid,
        organization_id: Uuid,
        operation: AnalyticsOperation,
        timestamp: DateTime<Utc>,
        authentication: AuthenticatedEntity,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            id,
            organization_id,
            operation,
            timestamp,
            authentication,
            metadata,
        }
    }
}

impl Display for AnalyticsEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ id: {}, organization_id: {}, operation: {}, authentication: {} }}",
            self.id, self.organization_id, self.operation, self.authentication
        )
    }
}

impl DiscoveryPhase {
    fn log_level(&self) -> EventLogLevel {
        EventLogLevel::Info
    }
}

impl DiscoveryUpdatePayload {
    pub fn into_discovery_event(&self) -> DiscoverySessionEvent {
        let mut metadata = json!({});
        if let Some(ref error) = self.error {
            metadata["error_reason"] = json!(error);
        }
        DiscoverySessionEvent {
            id: Uuid::new_v4(),
            network_id: self.network_id,
            session_id: self.session_id,
            daemon_id: self.daemon_id,
            discovery_type: self.discovery_type.clone(),
            phase: self.phase,
            timestamp: Utc::now(),
            authentication: AuthenticatedEntity::System,
            metadata,
        }
    }

    pub fn into_discovery_event_with_auth(
        &self,
        auth: AuthenticatedEntity,
    ) -> DiscoverySessionEvent {
        let mut event = self.into_discovery_event();
        event.authentication = auth;
        event
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DiscoverySessionEvent {
    pub id: Uuid,
    pub network_id: Uuid,
    pub session_id: Uuid,
    pub daemon_id: Uuid,
    pub discovery_type: DiscoveryType,
    pub phase: DiscoveryPhase,
    pub timestamp: DateTime<Utc>,
    pub authentication: AuthenticatedEntity,
    pub metadata: serde_json::Value,
}

impl DiscoverySessionEvent {
    /// Create a new DiscoverySessionEvent.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: Uuid,
        session_id: Uuid,
        network_id: Uuid,
        daemon_id: Uuid,
        phase: DiscoveryPhase,
        discovery_type: DiscoveryType,
        timestamp: DateTime<Utc>,
        authentication: AuthenticatedEntity,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            id,
            session_id,
            network_id,
            daemon_id,
            discovery_type,
            phase,
            timestamp,
            authentication,
            metadata,
        }
    }
}

impl Display for DiscoverySessionEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ id: {}, session_id: {}, phase: {}, authentication: {} }}",
            self.id, self.session_id, self.phase, self.authentication
        )
    }
}
