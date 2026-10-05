use std::fmt::Display;

use crate::server::shared::entities::ChangeTriggersTopologyStaleness;
use crate::server::shared::types::Color;
use crate::server::shared::types::entities::EntitySource;
use crate::server::topology::types::edges::EdgeStyle;
use crate::server::{
    dependencies::r#impl::types::DependencyType,
    shared::types::api::deserialize_empty_string_as_none,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// The members of a dependency: either service-level or binding-level.
/// Bindings are all-or-nothing: either every position has a binding (full L3 detail)
/// or none do (Application-level only).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
#[serde(tag = "type")]
pub enum DependencyMembers {
    /// Application-level only. Ordered list of service IDs.
    #[schema(title = "Services")]
    Services {
        /// The services in the chain, in order.
        service_ids: Vec<Uuid>,
    },
    /// Full L3 detail. Ordered list of binding IDs (one per service in the chain).
    #[schema(title = "Bindings")]
    Bindings {
        /// The bindings in the chain, in order — one per service.
        binding_ids: Vec<Uuid>,
    },
}

impl Default for DependencyMembers {
    fn default() -> Self {
        Self::Services {
            service_ids: Vec::new(),
        }
    }
}

impl DependencyMembers {
    /// Get ordered service IDs regardless of variant.
    /// For Bindings variant, this returns the service_ids that were stored alongside bindings.
    /// Note: for Bindings variant, caller must have the junction table data to get service_ids.
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Services { service_ids } => service_ids.is_empty(),
            Self::Bindings { binding_ids } => binding_ids.is_empty(),
        }
    }

    pub fn is_bindings(&self) -> bool {
        matches!(self, Self::Bindings { .. })
    }
}

#[derive(
    Debug, Clone, Serialize, Validate, Deserialize, PartialEq, Eq, Hash, Default, ToSchema,
)]
pub struct DependencyBase {
    /// Human-facing name for this dependency.
    #[validate(length(min = 0, max = 100))]
    pub name: String,
    /// The site this entity belongs to.
    pub site_id: Uuid,
    /// Free-text notes about the dependency.
    #[serde(deserialize_with = "deserialize_empty_string_as_none")]
    #[validate(length(min = 0, max = 500))]
    pub description: Option<String>,
    /// What type of relationship this dependency records.
    pub dependency_type: DependencyType,
    /// Members of this dependency: either service IDs or binding IDs.
    #[serde(default)]
    #[schema(required)]
    pub members: DependencyMembers,
    #[serde(default)]
    #[schema(read_only)]
    /// Will be automatically set to Manual for creation through API
    pub source: EntitySource,
    /// Colour the dependency edge is drawn in.
    pub color: Color,
    /// Line style the dependency edge is drawn with.
    #[serde(default)]
    #[schema(required)]
    pub edge_style: EdgeStyle,
    /// Tags assigned to this entity.
    #[serde(default)]
    #[schema(required)]
    pub tags: Vec<Uuid>,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema, Validate,
)]
#[schema(example = crate::server::shared::types::examples::dependency)]
pub struct Dependency {
    /// Server-assigned unique identifier.
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    /// When this record was first created.
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    /// When this record was last modified.
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    /// Start of the interval this revision was current for (SCD2 history).
    #[serde(default)]
    #[schema(read_only)]
    pub valid_from: DateTime<Utc>,
    /// End of the interval this revision was current for. `null` while it is the live revision.
    #[serde(default)]
    #[schema(read_only)]
    pub valid_to: Option<DateTime<Utc>>,
    /// Stable identifier shared by every revision of the same entity across its history.
    #[serde(default)]
    #[schema(read_only)]
    pub lineage_id: Option<Uuid>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: DependencyBase,
}

impl Display for Dependency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Dependency {}: {}", self.base.name, self.id)
    }
}

impl Dependency {
    /// Get binding IDs (only for Bindings variant).
    pub fn binding_ids(&self) -> Vec<Uuid> {
        match &self.base.members {
            DependencyMembers::Bindings { binding_ids } => binding_ids.clone(),
            DependencyMembers::Services { .. } => Vec::new(),
        }
    }
}

impl ChangeTriggersTopologyStaleness<Dependency> for Dependency {
    fn triggers_staleness(&self, other: Option<Dependency>) -> bool {
        match other {
            Some(other) => self.base.members != other.base.members,
            None => true,
        }
    }
}
