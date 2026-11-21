//! The event a dependency losing a member is.
//!
//! A dependency's members outlive what they name: when a member's binding changes hands or its
//! service is deleted, the member stays and names an id that no longer resolves, and the user
//! decides what replaces it. This says it happened, in the same warning category as
//! [`SubnetCorrection`]: a row changed on its own initiative, and an operator who sees a
//! dependency come up short should find out why without turning up the log level.
//!
//! Published by the dependency service's subscriber on a binding's or a service's `Deleted`, read
//! from the reason it carries, one per affected member.
//!
//! [`SubnetCorrection`]: crate::server::subnets::r#impl::correction_events::SubnetCorrection

use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use strum::IntoDiscriminant;
use strum_macros::{AsRefStr, EnumDiscriminants};
use uuid::Uuid;

use crate::server::shared::events::EventFlags;
use crate::server::shared::events::registry::to_snake_case;
use crate::server::shared::events::traits::{
    EventFilter, EventScope, Operation, ScopeOrganization,
};
use crate::server::shared::events::types::EventLogLevel;

/// What happened to what a dependency member names.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, EnumDiscriminants)]
#[strum_discriminants(derive(Hash, AsRefStr, Serialize, Deserialize))]
pub enum DependencyMemberChange {
    /// The member's binding changed hands: another service answers on its port now.
    MemberMoved {
        /// The service that took the port.
        to_service_id: Uuid,
    },
    /// The member's binding was released: nothing answered on its port past the stale window.
    MemberReleased,
    /// The member's service was deleted, or the service its binding belonged to.
    MemberDeleted,
}

/// The dependency and the member that changed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DependencyMemberChangeScope {
    pub site_id: Uuid,
    pub dependency_id: Uuid,
    /// The service the member names, or the one its binding belonged to.
    pub service_id: Uuid,
    /// The binding the member names, for a dependency of bindings.
    pub binding_id: Option<Uuid>,
}

impl Operation for DependencyMemberChange {
    type Scope = DependencyMemberChangeScope;
    type Flags = EventFlags;
    type Filter = EventFilter<DependencyMemberChange>;

    fn log_level(&self) -> EventLogLevel {
        EventLogLevel::Warn
    }

    /// `dependency_member_moved`, `dependency_member_released` or `dependency_member_deleted`.
    fn event_name(&self, _scope: &DependencyMemberChangeScope) -> Cow<'static, str> {
        format!("dependency_{}", to_snake_case(self.discriminant().as_ref())).into()
    }

    fn metadata<'a>(
        &'a self,
        scope: &'a DependencyMemberChangeScope,
        _flags: &'a Self::Flags,
    ) -> impl Serialize + Send + 'a {
        DependencyMemberChangeMetadata {
            scope,
            change: self,
        }
    }
}

impl EventScope for DependencyMemberChangeScope {
    fn organization(&self) -> Option<ScopeOrganization> {
        Some(ScopeOrganization::Site(self.site_id))
    }
}

/// What leaves the server about a member change: ids only.
#[derive(Serialize)]
struct DependencyMemberChangeMetadata<'a> {
    #[serde(flatten)]
    scope: &'a DependencyMemberChangeScope,
    change: &'a DependencyMemberChange,
}
