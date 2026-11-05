use std::fmt::Display;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::daemon::discovery::types::warnings::DiscoveryWarning;

/// One stored warning. `details` is the warning with its entity ids taken out; the ids live in the
/// columns beside it, where deleting the entity sets them to NULL rather than leaving them
/// dangling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryWarningRecordBase {
    pub discovery_id: Uuid,
    pub site_id: Uuid,
    pub credential_id: Option<Uuid>,
    pub host_id: Option<Uuid>,
    pub remote_host_id: Option<Uuid>,
    pub subnet_id: Option<Uuid>,
    pub details: DiscoveryWarning,
}

impl Default for DiscoveryWarningRecordBase {
    fn default() -> Self {
        Self {
            discovery_id: Uuid::nil(),
            site_id: Uuid::nil(),
            credential_id: None,
            host_id: None,
            remote_host_id: None,
            subnet_id: None,
            details: DiscoveryWarning::Unknown {
                detail: String::new(),
            },
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiscoveryWarningRecord {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub base: DiscoveryWarningRecordBase,
}

impl DiscoveryWarningRecord {
    pub fn new(base: DiscoveryWarningRecordBase) -> Self {
        Self {
            id: Uuid::new_v4(),
            created_at: crate::server::shared::clock::now(),
            base,
        }
    }

    /// Split a warning into its row: each id it carries moves into its column.
    pub fn from_warning(discovery_id: Uuid, site_id: Uuid, warning: &DiscoveryWarning) -> Self {
        let mut details = warning.clone();
        let slots = details.entity_id_slots();
        let take = |slot: Option<&mut Option<Uuid>>| slot.and_then(Option::take);
        let credential_id = take(slots.credential_id);
        let host_id = take(slots.host_id);
        let remote_host_id = take(slots.remote_host_id);
        let subnet_id = take(slots.subnet_id);
        Self::new(DiscoveryWarningRecordBase {
            discovery_id,
            site_id,
            credential_id,
            host_id,
            remote_host_id,
            subnet_id,
            details,
        })
    }

    /// The warning again, with the ids its columns still hold put back. A deleted entity's id
    /// comes back `None`.
    pub fn into_warning(self) -> DiscoveryWarning {
        let mut warning = self.base.details;
        let slots = warning.entity_id_slots();
        let put = |slot: Option<&mut Option<Uuid>>, id: Option<Uuid>| {
            if let Some(slot) = slot {
                *slot = id;
            }
        };
        put(slots.credential_id, self.base.credential_id);
        put(slots.host_id, self.base.host_id);
        put(slots.remote_host_id, self.base.remote_host_id);
        put(slots.subnet_id, self.base.subnet_id);
        warning
    }
}

impl Display for DiscoveryWarningRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DiscoveryWarning({}, discovery={})",
            self.base.details.code(),
            self.base.discovery_id
        )
    }
}

/// Where an assumed range came from: the far ends whose published addresses it was drawn around,
/// and the local devices that saw them. Read from the warning of the scan that created the range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct RangeEvidence {
    /// The run that created the range, so the reader can open its report.
    pub discovery_id: Uuid,
    /// The local devices that saw the far ends. A host deleted since is still listed here; it
    /// resolves to nothing.
    pub seen_by_host_ids: Vec<Uuid>,
    /// The addresses the far ends published for themselves, in the order observed.
    pub addresses: Vec<String>,
    /// The far ends' own `sysName`s, where they sent any.
    pub sys_names: Vec<String>,
    /// Whether a shared VLAN widened the range past the conventional prefix.
    pub widened_by_vlan: bool,
}
