use crate::server::credentials::r#impl::types::CredentialAssignment;
use crate::server::hosts::r#impl::virtualization::HostVirtualization;
use crate::server::shared::entities::ChangeTriggersTopologyStaleness;
use crate::server::shared::types::api::deserialize_empty_string_as_none;
use crate::server::shared::types::entities::EntitySource;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::hash::Hash;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Base data for a Host entity (stored in database).
/// Child entities (interfaces, ports, services) are stored in their own tables
/// and queried by `host_id`. They are NOT stored on the host.
#[derive(Debug, Clone, Serialize, Validate, Deserialize, Eq, PartialEq, Hash, ToSchema)]
pub struct HostBase {
    #[validate(length(min = 0, max = 100))]
    pub name: String,
    pub network_id: Uuid,
    #[schema(required)]
    pub hostname: Option<String>,
    #[validate(length(min = 0, max = 500))]
    #[serde(deserialize_with = "deserialize_empty_string_as_none")]
    #[schema(required)]
    pub description: Option<String>,
    #[schema(read_only)]
    pub source: EntitySource,
    #[schema(required)]
    pub virtualization: Option<HostVirtualization>,
    pub hidden: bool,
    #[serde(default)]
    #[schema(required)]
    pub tags: Vec<Uuid>,
    // SNMP System MIB fields
    /// SNMP sysDescr.0 - full system description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sys_descr: Option<String>,
    /// SNMP sysObjectID.0 - vendor OID for device identification
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sys_object_id: Option<String>,
    /// SNMP sysLocation.0 - physical location
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sys_location: Option<String>,
    /// SNMP sysContact.0 - admin contact info
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sys_contact: Option<String>,
    /// URL for device management interface (manual or discovered)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub management_url: Option<String>,
    /// LLDP lldpLocChassisId - globally unique device identifier for deduplication
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chassis_id: Option<String>,
    /// SNMP sysName.0 - administratively-assigned hostname
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sys_name: Option<String>,
    /// ENTITY-MIB entPhysicalMfgName - hardware manufacturer
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manufacturer: Option<String>,
    /// ENTITY-MIB entPhysicalModelName - hardware model
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// ENTITY-MIB entPhysicalSerialNum - hardware serial number
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serial_number: Option<String>,
    /// Credential assignments for this host (hydrated from junction table).
    #[serde(default)]
    #[schema(required)]
    pub credential_assignments: Vec<CredentialAssignment>,
}

impl Default for HostBase {
    fn default() -> Self {
        Self {
            name: String::new(),
            network_id: Uuid::nil(),
            hostname: None,
            description: None,
            source: EntitySource::Unknown,
            virtualization: None,
            hidden: false,
            tags: Vec::new(),
            sys_descr: None,
            sys_object_id: None,
            sys_location: None,
            sys_contact: None,
            management_url: None,
            chassis_id: None,
            sys_name: None,
            manufacturer: None,
            model: None,
            serial_number: None,
            credential_assignments: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, Default, ToSchema, Validate)]
#[schema(example = crate::server::shared::types::examples::host)]
pub struct Host {
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: HostBase,
}

impl Hash for Host {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for Host {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Display for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {:?}", self.base.name, self.id)
    }
}

impl Host {
    pub fn new(base: HostBase) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: uuid::Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base,
        }
    }
}

impl ChangeTriggersTopologyStaleness<Host> for Host {
    fn triggers_staleness(&self, other: Option<Host>) -> bool {
        if let Some(other_host) = other {
            self.base.hostname != other_host.base.hostname
                || self.base.virtualization != other_host.base.virtualization
                || self.base.hidden != other_host.base.hidden
        } else {
            true
        }
    }
}
