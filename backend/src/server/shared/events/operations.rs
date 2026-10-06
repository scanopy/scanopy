//! `Operation` impls for the event-layer operation types: log level, event name, attribution, and
//! the metadata a consumer outside the server receives.

use std::borrow::Cow;

use serde::Serialize;
use uuid::Uuid;

use crate::daemon::discovery::types::base::{DiscoveryPhase, DiscoveryTerminalReason};
use crate::server::{
    discovery::r#impl::types::DiscoveryType,
    shared::{
        events::{
            registry::to_snake_case,
            traits::{
                Attribution, AuthScope, DiscoveryScope, EntityEventFilter, EntityEventFlags,
                EntityScope, EventFilter, EventFlags, Operation, OrgScope,
            },
            types::{
                AnalyticsOperation, AnalyticsOperationDiscriminants, AuthOperation,
                AuthOperationDiscriminants, BillingOperation, BillingOperationDiscriminants,
                EntityOperation, EntityOperationDiscriminants, EventLogLevel, LabelColor,
                OnboardingOperation, OnboardingOperationDiscriminants,
            },
        },
        types::metadata::HasId,
    },
};

impl Operation for BillingOperation {
    type Scope = OrgScope;
    type Flags = EventFlags;
    type Filter = EventFilter<BillingOperation>;
    fn log_level(&self) -> EventLogLevel {
        EventLogLevel::Info
    }

    fn event_name(&self, _scope: &OrgScope) -> Cow<'static, str> {
        self.to_string().into()
    }
}

impl Operation for OnboardingOperation {
    type Scope = OrgScope;
    type Flags = EventFlags;
    type Filter = EventFilter<OnboardingOperation>;
    fn log_level(&self) -> EventLogLevel {
        EventLogLevel::Info
    }

    fn event_name(&self, _scope: &OrgScope) -> Cow<'static, str> {
        self.to_string().into()
    }
}

impl Operation for AnalyticsOperation {
    type Scope = OrgScope;
    type Flags = EventFlags;
    type Filter = EventFilter<AnalyticsOperation>;
    fn log_level(&self) -> EventLogLevel {
        EventLogLevel::Debug
    }

    fn event_name(&self, _scope: &OrgScope) -> Cow<'static, str> {
        self.to_string().into()
    }

    /// An email goes to one user, so its send lands on the same person as their click; a share
    /// view has no viewer identity and belongs to the org.
    fn attribution(&self, _scope: &OrgScope) -> Attribution {
        match self {
            Self::EmailSent { user_id, .. } => Attribution::User(*user_id),
            Self::TopologyShareViewed { .. } | Self::TopologyEmbedViewed { .. } => {
                Attribution::Organization
            }
        }
    }
}

impl Operation for AuthOperation {
    type Scope = AuthScope;
    type Flags = EventFlags;
    type Filter = EventFilter<AuthOperation>;
    fn log_level(&self) -> EventLogLevel {
        match self {
            AuthOperation::LoginFailed { .. } | AuthOperation::ApiKeyAuthFailed { .. } => {
                EventLogLevel::Warn
            }
            _ => EventLogLevel::Info,
        }
    }

    fn event_name(&self, _scope: &AuthScope) -> Cow<'static, str> {
        match self {
            AuthOperation::LoginSuccess { .. } => "login".into(),
            _ => self.to_string().into(),
        }
    }

    /// The user the scope names: during login or registration nobody is authenticated yet.
    fn attribution(&self, scope: &AuthScope) -> Attribution {
        scope.user_id.map_or(Attribution::Actor, Attribution::User)
    }
}

/// Entity events send the entity's id, never the entity.
#[derive(Serialize)]
struct EntityMetadata {
    entity_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    site_id: Option<Uuid>,
}

impl Operation for EntityOperation {
    type Scope = EntityScope;
    type Flags = EntityEventFlags;
    type Filter = EntityEventFilter;
    fn log_level(&self) -> EventLogLevel {
        EventLogLevel::Info
    }

    /// The entity type and the operation, e.g. `"subnet_created"`.
    fn event_name(&self, scope: &EntityScope) -> Cow<'static, str> {
        format!(
            "{}_{}",
            to_snake_case(scope.entity_discriminant().as_ref()),
            self
        )
        .into()
    }

    fn log_color(&self) -> LabelColor {
        match self {
            EntityOperation::Created => LabelColor::Green,
            EntityOperation::Updated => LabelColor::Blue,
            EntityOperation::Deleted => LabelColor::Red,
            EntityOperation::Get | EntityOperation::GetAll => LabelColor::Neutral,
        }
    }

    fn metadata<'a>(&'a self, scope: &'a EntityScope) -> impl Serialize + Send + 'a {
        EntityMetadata {
            entity_id: scope.entity_id(),
            site_id: match scope {
                EntityScope::Site { site_id, .. } => Some(*site_id),
                EntityScope::Org { .. } => None,
            },
        }
    }
}

/// A discovery session's identifiers and outcome. `discovery_type` is the type's name only:
/// the full value carries the scanned subnets and hosts.
#[derive(Serialize)]
struct DiscoveryMetadata<'a> {
    session_id: Uuid,
    site_id: Uuid,
    daemon_id: Uuid,
    discovery_type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    discovery_subnet_scan: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_reason: Option<&'a str>,
    /// Splits stalls from failures the daemon reported, which share `discovery_failed`.
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<DiscoveryTerminalReason>,
}

impl Operation for DiscoveryPhase {
    type Scope = DiscoveryScope;
    type Flags = EventFlags;
    type Filter = EventFilter<DiscoveryPhase>;
    fn log_level(&self) -> EventLogLevel {
        match self {
            DiscoveryPhase::Failed => EventLogLevel::Warn,
            _ => EventLogLevel::Info,
        }
    }

    fn event_name(&self, _scope: &DiscoveryScope) -> Cow<'static, str> {
        format!("discovery_{}", to_snake_case(self.id())).into()
    }

    fn metadata<'a>(&'a self, scope: &'a DiscoveryScope) -> impl Serialize + Send + 'a {
        DiscoveryMetadata {
            session_id: scope.session_id,
            site_id: scope.site_id,
            daemon_id: scope.daemon_id,
            discovery_type: (&scope.discovery_type).into(),
            discovery_subnet_scan: match &scope.discovery_type {
                DiscoveryType::Network { subnet_ids, .. } => Some(subnet_ids.is_some()),
                _ => None,
            },
            error_reason: scope.error_reason.as_deref(),
            reason: scope.reason,
        }
    }
}

// Convenience aliases for the discriminant types so consumers don't have to
// write `<BillingOperation as IntoDiscriminant>::Discriminant`.
pub type BillingDiscriminant = BillingOperationDiscriminants;
pub type OnboardingDiscriminant = OnboardingOperationDiscriminants;
pub type AnalyticsDiscriminant = AnalyticsOperationDiscriminants;
pub type AuthDiscriminant = AuthOperationDiscriminants;
pub type EntityDiscriminant = EntityOperationDiscriminants;

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::daemon::discovery::types::warnings::DiscoveryWarning;
    use crate::server::{
        auth::middleware::auth::AuthenticatedEntity,
        digest::payload::{DiscoveryDigestOperation, DiscoveryDigestPayload, DiscoveryDigestScope},
        discovery::r#impl::warning_events::DiscoveryWarningScope,
        shared::{
            attribution::AttributeSource,
            events::{
                traits::{Event, EventProperties},
                types::AuthMethod,
            },
            types::examples,
        },
        subnets::r#impl::correction_events::{SubnetCorrection, SubnetCorrectionScope},
        users::r#impl::permissions::UserOrgPermissions,
    };
    use serde_json::Value;
    use strum::IntoEnumIterator;

    /// One event, as every consumer outside the server receives it.
    pub(crate) struct Sample {
        pub name: String,
        pub attribution: Attribution,
        pub properties: EventProperties<Value>,
    }

    fn sample<Op: Operation>(event: Event<Op>, organization_id: Uuid) -> Sample {
        Sample {
            name: event.name().into_owned(),
            attribution: event.attribution(),
            properties: event
                .properties(Some(organization_id))
                .map_metadata(|m| serde_json::to_value(m).expect("metadata serializes")),
        }
    }

    /// An event of every operation type, belonging to `organization_id`; site-scoped events
    /// resolve their site to it.
    pub(crate) fn samples(organization_id: Uuid) -> Vec<Sample> {
        let site_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let daemon_id = Uuid::new_v4();
        let system = || AuthenticatedEntity::System;
        let warning = DiscoveryWarning::InterfaceSetCutShort {
            address: "192.0.2.1".parse().expect("address"),
            collected: 3,
        };
        let now = chrono::Utc::now();

        vec![
            sample(
                Event::new(
                    EntityScope::Org {
                        organization_id,
                        entity_id: organization_id,
                        entity_type: examples::organization().into(),
                    },
                    EntityOperation::Deleted,
                    system(),
                ),
                organization_id,
            ),
            sample(
                Event::new(
                    EntityScope::Site {
                        site_id,
                        entity_id: Uuid::new_v4(),
                        entity_type: examples::host().into(),
                    },
                    EntityOperation::Created,
                    system(),
                ),
                organization_id,
            ),
            sample(
                Event::new(
                    AuthScope {
                        user_id: Some(Uuid::new_v4()),
                        organization_id: Some(organization_id),
                        ip_address: "192.0.2.2".parse().expect("address"),
                        user_agent: None,
                    },
                    AuthOperation::LoginSuccess {
                        method: AuthMethod::Password,
                        via_register_flow: false,
                    },
                    AuthenticatedEntity::Anonymous,
                ),
                organization_id,
            ),
            sample(
                Event::new(
                    OrgScope { organization_id },
                    BillingOperation::LicenseActivated {
                        server_version: Some("0.17.20".to_string()),
                    },
                    system(),
                ),
                organization_id,
            ),
            sample(
                Event::new(
                    OrgScope { organization_id },
                    OnboardingOperation::OnboardingModalCompleted,
                    system(),
                ),
                organization_id,
            ),
            sample(
                Event::new(
                    OrgScope { organization_id },
                    AnalyticsOperation::TopologyShareViewed {
                        share_id: Uuid::new_v4(),
                        has_password: false,
                    },
                    AuthenticatedEntity::Anonymous,
                ),
                organization_id,
            ),
            sample(
                Event::new(
                    DiscoveryScope {
                        site_id,
                        session_id,
                        daemon_id,
                        discovery_type: DiscoveryType::default(),
                        error_reason: None,
                        reason: None,
                    },
                    DiscoveryPhase::Complete,
                    system(),
                ),
                organization_id,
            ),
            sample(
                Event::new(
                    DiscoveryWarningScope::new(site_id, session_id, daemon_id, warning.clone()),
                    warning.code(),
                    system(),
                ),
                organization_id,
            ),
            sample(
                Event::new(
                    SubnetCorrectionScope {
                        site_id,
                        subnet_id: Uuid::new_v4(),
                        from_cidr: "192.0.2.0/25".to_string(),
                        to_cidr: "192.0.2.0/24".to_string(),
                        from_source: AttributeSource::default(),
                        to_source: AttributeSource::default(),
                    },
                    SubnetCorrection::Widened,
                    system(),
                ),
                organization_id,
            ),
            sample(
                Event::new(
                    DiscoveryDigestScope {
                        organization_id,
                        site_id,
                    },
                    DiscoveryDigestOperation::Computed {
                        payload: Box::new(DiscoveryDigestPayload {
                            session_id,
                            site_id,
                            site_name: "HQ".to_string(),
                            started_at: now,
                            finished_at: now,
                            stale_after_hours: 24,
                            subnets_scanned: vec![],
                            hosts_added: vec![],
                            hosts_stale: vec![],
                            hosts_changed: vec![],
                            vlans_added: vec![],
                            vlans_stale: vec![],
                            recipients: vec![],
                        }),
                    },
                    system(),
                ),
                organization_id,
            ),
        ]
    }

    /// Every operation type yields the same top level: who acted, this server's version, its
    /// organization, and its own fields under `metadata`.
    #[test]
    fn every_event_carries_the_common_properties_and_metadata() {
        let organization_id = Uuid::new_v4();
        for sample in samples(organization_id) {
            let properties = serde_json::to_value(&sample.properties).expect("serializes");
            assert!(properties["auth_type"].is_string(), "{}", sample.name);
            assert!(properties["server_version"].is_string(), "{}", sample.name);
            assert_eq!(
                properties["organization_id"],
                Value::String(organization_id.to_string()),
                "{}",
                sample.name
            );
            assert!(properties["metadata"].is_object(), "{}", sample.name);
        }
    }

    /// A site the lookup could not resolve still leaves the acting user's organization.
    #[test]
    fn an_unresolved_site_falls_back_to_the_actors_organization() {
        let organization_id = Uuid::new_v4();
        let event = Event::new(
            EntityScope::Site {
                site_id: Uuid::new_v4(),
                entity_id: Uuid::new_v4(),
                entity_type: examples::host().into(),
            },
            EntityOperation::Created,
            AuthenticatedEntity::ApiKey {
                api_key_id: Uuid::new_v4(),
                user_id: Uuid::new_v4(),
                organization_id,
                permissions: UserOrgPermissions::Member,
                site_ids: vec![],
            },
        );

        assert_eq!(
            event.properties(None).actor.organization_id,
            Some(organization_id)
        );
    }

    /// Each phase is its own event; two sharing a name would merge in every consumer.
    #[test]
    fn every_discovery_phase_has_its_own_name() {
        let scope = DiscoveryScope {
            site_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            daemon_id: Uuid::new_v4(),
            discovery_type: DiscoveryType::default(),
            error_reason: None,
            reason: None,
        };
        let names: std::collections::HashSet<_> = DiscoveryPhase::iter()
            .map(|phase| phase.event_name(&scope))
            .collect();
        assert_eq!(names.len(), DiscoveryPhase::iter().count());
    }
}
