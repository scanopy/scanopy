use std::fmt::Display;

use crate::server::shared::{
    entities::ChangeTriggersTopologyStaleness,
    types::{api::deserialize_empty_string_as_none, entities::EntitySource},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Validate, Serialize, Deserialize, Eq, PartialEq, Hash, ToSchema)]
pub struct VlanBase {
    /// The 802.1Q VLAN number (1-4094)
    pub vlan_number: u16,
    /// Human-facing name for this VLAN.
    #[validate(length(
        min = 1,
        max = 100,
        message = "VLAN name must be between 1 and 100 characters"
    ))]
    pub name: String,
    /// Free-text notes about the VLAN.
    #[serde(default, deserialize_with = "deserialize_empty_string_as_none")]
    pub description: Option<String>,
    /// The site this entity belongs to.
    pub site_id: Uuid,
    /// The organization that owns this record.
    pub organization_id: Uuid,
    /// How this VLAN came to be known — discovered, imported, or created by hand.
    #[serde(default)]
    pub source: EntitySource,
    /// Subnets associated with this VLAN, derived from discovered interface
    /// native-VLAN data via the `subnet_vlans` junction. Hydrated by
    /// `VlanService` on read; it is not a column on `vlans`, so anything sent
    /// here on create/update is ignored by `to_params`.
    #[serde(default)]
    #[schema(read_only)]
    pub subnet_ids: Vec<Uuid>,
}

impl Default for VlanBase {
    fn default() -> Self {
        Self {
            vlan_number: 1,
            name: "Default".to_string(),
            description: None,
            site_id: Uuid::nil(),
            organization_id: Uuid::nil(),
            source: EntitySource::Manual,
            subnet_ids: Vec::new(),
        }
    }
}

#[derive(
    Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, Default, ToSchema, Validate,
)]
pub struct Vlan {
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
    /// When a discovery last observed this entity.
    #[serde(default)]
    #[schema(read_only)]
    pub last_seen_at: DateTime<Utc>,
    /// The most recent discovery that observed this entity.
    #[serde(default)]
    #[schema(read_only)]
    pub last_discovery_id: Option<Uuid>,
    /// The discovery that first observed this entity.
    #[serde(default)]
    #[schema(read_only)]
    pub first_discovery_id: Option<Uuid>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: VlanBase,
}

impl ChangeTriggersTopologyStaleness<Vlan> for Vlan {
    fn triggers_staleness(&self, other: Option<Vlan>) -> bool {
        match other {
            Some(prev) => self.base.name != prev.base.name,
            None => true,
        }
    }
}

impl Display for Vlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Vlan {} ({}): {}",
            self.base.vlan_number, self.base.name, self.id
        )
    }
}
