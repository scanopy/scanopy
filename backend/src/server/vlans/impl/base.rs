use std::fmt::Display;

use crate::attributed_value;
use crate::server::shared::{
    attribution::{self as attribution, AttributeSource, string_schema},
    entities::ChangeTriggersTopologyStaleness,
    types::{api::deserialize_empty_string_as_none, entities::EntitySource},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::{Validate, ValidationError};

attributed_value! {
    /// A VLAN's human-facing name.
    ///
    /// Refreshable: a VLAN renamed on the switch or the controller is the same VLAN, and a re-read
    /// from the same source should follow it. Two different sources at one rung do not take turns,
    /// and nothing discovery reads displaces a name typed into Scanopy.
    pub struct VlanNameValue(String) as VlanName {
        key: "name",
        source_key: "name_source",
        schema_name: "VlanName",
        refreshable: true,
        // A VLAN without a name is not one the API has ever accepted.
        required: true,
        blank: |value: &str| value.trim().is_empty(),
        schema: string_schema("Human-facing name for this VLAN."),
    }
}

/// The 1-100 character bound the API has always enforced. A custom validator because the derive
/// cannot see a length through the carrier.
fn validate_vlan_name(name: &VlanName) -> Result<(), ValidationError> {
    let length = name.value().0.chars().count();
    if !(1..=100).contains(&length) {
        let mut error = ValidationError::new("length");
        error.message = Some("VLAN name must be between 1 and 100 characters".into());
        return Err(error);
    }
    Ok(())
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize, Eq, PartialEq, Hash, ToSchema)]
pub struct VlanBase {
    /// The 802.1Q VLAN number (1-4094)
    pub vlan_number: u16,
    /// Human-facing name for this VLAN, with the source that named it.
    ///
    /// Serialises as the two flat keys `name` and `name_source`, so a client that sends only
    /// `name` still deserialises, at `Unspecified`.
    #[serde(flatten, deserialize_with = "attribution::required")]
    #[schema(value_type = VlanName)]
    #[validate(custom(function = "validate_vlan_name"))]
    pub name: VlanName,
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
    #[serde(
        default,
        deserialize_with = "crate::server::shared::types::api::ignored"
    )]
    #[schema(read_only)]
    pub subnet_ids: Vec<Uuid>,
    /// Interfaces that serve this VLAN, as their native VLAN or one of their
    /// member VLANs (`Interface::native_vlan_id` / `Interface::vlan_ids`).
    /// Hydrated by `VlanService` on read; not a column on `vlans`.
    #[serde(
        default,
        deserialize_with = "crate::server::shared::types::api::ignored"
    )]
    #[schema(read_only)]
    pub interface_ids: Vec<Uuid>,
    /// Tags assigned to this entity.
    #[serde(default)]
    #[schema(required)]
    pub tags: Vec<Uuid>,
}

impl Default for VlanBase {
    fn default() -> Self {
        Self {
            vlan_number: 1,
            name: VlanName::new(
                VlanNameValue("Default".to_string()),
                AttributeSource::Manual,
            ),
            description: None,
            site_id: Uuid::nil(),
            organization_id: Uuid::nil(),
            source: EntitySource::Manual,
            subnet_ids: Vec::new(),
            interface_ids: Vec::new(),
            tags: Vec::new(),
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

impl crate::server::shared::events::log::EntityLog for Vlan {}

impl ChangeTriggersTopologyStaleness<Vlan> for Vlan {
    fn triggers_staleness(&self, other: Option<Vlan>) -> bool {
        match other {
            Some(prev) => self.base.name.value() != prev.base.name.value(),
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
