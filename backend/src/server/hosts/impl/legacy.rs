//! Legacy API types for backwards compatibility with old daemons.
//!
//! Old daemons send `POST /api/hosts` with `LegacyHostWithServicesRequest` format
//! and expect `LegacyHostWithServicesResponse` back. This module provides the types
//! and transformations needed to handle these requests.
//!
//! This module is intentionally kept minimal and deletable. Once all daemons
//! are updated (target: 30 days post-release), this module should be removed.
//!
//! See: docs/design/daemon-backwards-compatibility.md

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use mac_address::MacAddress;
use std::net::IpAddr;

use crate::server::{
    bindings::r#impl::base::{Binding, BindingBase, BindingType},
    hosts::r#impl::{
        api::{DiscoveryHostRequest, HostResponse},
        base::Host,
    },
    interfaces::r#impl::base::{Interface, InterfaceBase},
    ports::r#impl::base::{Port, PortBase, PortType},
    services::{
        definitions::ServiceDefinitionRegistry,
        r#impl::{
            base::{Service, ServiceBase},
            definitions::DefaultServiceDefinition,
        },
    },
    shared::types::entities::EntitySource,
};

/// Legacy host request format from old daemons.
///
/// Old daemons send this to `POST /api/hosts`. The format has embedded
/// `interfaces`, `ports`, and `services` (as UUIDs) directly on the host object,
/// plus a separate `services` array with full service details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyHostWithServicesRequest {
    pub host: LegacyHost,
    pub services: Vec<LegacyService>,
}

/// Legacy host object from old daemons.
///
/// This matches the old `Host` format that was serialized with embedded
/// children and a `target` field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyHost {
    // Entity identity
    pub id: Uuid,
    #[serde(default = "default_timestamp")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "default_timestamp")]
    pub updated_at: DateTime<Utc>,

    // Host fields
    pub name: String,
    pub network_id: Uuid,
    #[serde(default)]
    pub hostname: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub tags: Vec<Uuid>,

    // Embedded children (old format had these directly on host)
    #[serde(default)]
    pub interfaces: Vec<LegacyInterface>,
    #[serde(default)]
    pub ports: Vec<LegacyPort>,
    /// Service IDs (old format had these as UUIDs, not full objects)
    #[serde(default)]
    pub services: Vec<serde_json::Value>,

    // Legacy field - ignored but must be accepted for backwards compat
    #[serde(default)]
    pub target: Option<serde_json::Value>,

    // Legacy field - also ignored
    #[serde(default)]
    pub source: Option<serde_json::Value>,

    // Legacy field - also ignored
    #[serde(default)]
    pub virtualization: Option<serde_json::Value>,
}

/// Legacy interface format from old daemons (missing network_id, host_id, position).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyInterface {
    pub id: Uuid,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
    pub subnet_id: Uuid,
    pub ip_address: IpAddr,
    #[serde(default)]
    pub mac_address: Option<MacAddress>,
    #[serde(default)]
    pub name: Option<String>,
}

impl LegacyInterface {
    /// Convert to new Interface format, filling in missing fields.
    pub fn into_interface(self, network_id: Uuid, host_id: Uuid) -> Interface {
        Interface {
            id: self.id,
            created_at: self.created_at.unwrap_or_else(Utc::now),
            updated_at: self.updated_at.unwrap_or_else(Utc::now),
            base: InterfaceBase {
                network_id,
                host_id,
                subnet_id: self.subnet_id,
                ip_address: self.ip_address,
                mac_address: self.mac_address,
                name: self.name,
                position: 0,
            },
        }
    }
}

/// Legacy port format from old daemons.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyPort {
    pub id: Uuid,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
    pub number: u16,
    #[serde(default)]
    pub protocol: Option<String>,
    #[serde(default, rename = "type")]
    pub port_type: Option<String>,
}

impl LegacyPort {
    /// Convert to new Port format, filling in missing fields.
    pub fn into_port(self, network_id: Uuid, host_id: Uuid) -> Port {
        let protocol = self.protocol.as_deref().unwrap_or("Tcp");
        let port_type = if protocol.eq_ignore_ascii_case("udp") {
            PortType::new_udp(self.number)
        } else {
            PortType::new_tcp(self.number)
        };

        Port {
            id: self.id,
            created_at: self.created_at.unwrap_or_else(Utc::now),
            updated_at: self.updated_at.unwrap_or_else(Utc::now),
            base: PortBase {
                network_id,
                host_id,
                port_type,
            },
        }
    }
}

/// Legacy binding type from old daemons (same enum structure, used for serde)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LegacyBindingType {
    Interface {
        interface_id: Uuid,
    },
    Port {
        port_id: Uuid,
        #[serde(default)]
        interface_id: Option<Uuid>,
    },
}

impl LegacyBindingType {
    fn into_binding_type(self) -> BindingType {
        match self {
            LegacyBindingType::Interface { interface_id } => {
                BindingType::Interface { interface_id }
            }
            LegacyBindingType::Port {
                port_id,
                interface_id,
            } => BindingType::Port {
                port_id,
                interface_id,
            },
        }
    }
}

/// Legacy binding format from old daemons (missing service_id, network_id).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyBinding {
    pub id: Uuid,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(flatten)]
    pub binding_type: LegacyBindingType,
}

impl LegacyBinding {
    /// Convert to new Binding format, filling in service_id and network_id.
    pub fn into_binding(self, service_id: Uuid, network_id: Uuid) -> Binding {
        Binding {
            id: self.id,
            created_at: self.created_at.unwrap_or_else(Utc::now),
            updated_at: self.updated_at.unwrap_or_else(Utc::now),
            base: BindingBase {
                service_id,
                network_id,
                binding_type: self.binding_type.into_binding_type(),
            },
        }
    }
}

/// Legacy service format from old daemons (uses service_definition_id, legacy bindings).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyService {
    pub id: Uuid,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
    pub host_id: Uuid,
    pub network_id: Uuid,
    /// Old daemons send service_definition as string ID
    pub service_definition: String,
    pub name: String,
    #[serde(default)]
    pub bindings: Vec<LegacyBinding>,
    #[serde(default)]
    pub virtualization: Option<serde_json::Value>,
    #[serde(default)]
    pub source: Option<serde_json::Value>,
    #[serde(default)]
    pub tags: Vec<Uuid>,
}

impl LegacyService {
    /// Convert to new Service format.
    pub fn into_service(self) -> Service {
        let service_id = self.id;
        let network_id = self.network_id;

        // Convert legacy bindings, filling in service_id and network_id
        let bindings: Vec<Binding> = self
            .bindings
            .into_iter()
            .map(|b| b.into_binding(service_id, network_id))
            .collect();

        // Look up service definition by ID
        let service_definition = ServiceDefinitionRegistry::find_by_id(&self.service_definition)
            .unwrap_or_else(|| Box::new(DefaultServiceDefinition));

        Service {
            id: self.id,
            created_at: self.created_at.unwrap_or_else(Utc::now),
            updated_at: self.updated_at.unwrap_or_else(Utc::now),
            base: ServiceBase {
                host_id: self.host_id,
                network_id: self.network_id,
                service_definition,
                name: self.name,
                bindings,
                virtualization: None, // Old virtualization format ignored
                source: EntitySource::Discovery { metadata: vec![] },
                tags: self.tags,
                position: 0,
            },
        }
    }
}

fn default_timestamp() -> DateTime<Utc> {
    Utc::now()
}

impl LegacyHostWithServicesRequest {
    /// Transform legacy request to the new DiscoveryHostRequest format.
    ///
    /// Maps embedded children from the old nested format to the new flat structure.
    pub fn into_discovery_request(self) -> DiscoveryHostRequest {
        let LegacyHostWithServicesRequest { host, services } = self;

        let network_id = host.network_id;
        let host_id = host.id;

        // Convert legacy interfaces to new format
        let interfaces: Vec<Interface> = host
            .interfaces
            .into_iter()
            .map(|i| i.into_interface(network_id, host_id))
            .collect();

        // Convert legacy ports to new format
        let ports: Vec<Port> = host
            .ports
            .into_iter()
            .map(|p| p.into_port(network_id, host_id))
            .collect();

        // Convert legacy services to new format
        let services: Vec<Service> = services.into_iter().map(|s| s.into_service()).collect();

        // Build new Host entity from legacy host
        let new_host = Host {
            id: host.id,
            created_at: host.created_at,
            updated_at: host.updated_at,
            base: crate::server::hosts::r#impl::base::HostBase {
                name: host.name,
                network_id: host.network_id,
                hostname: host.hostname,
                description: host.description,
                source: crate::server::shared::types::entities::EntitySource::Discovery {
                    metadata: vec![],
                },
                virtualization: None,
                hidden: host.hidden,
                tags: host.tags,
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
                credential_assignments: vec![],
            },
        };

        DiscoveryHostRequest {
            host: new_host,
            interfaces,
            ports,
            services,
            if_entries: vec![], // Legacy requests don't include SNMP data
        }
    }
}

/// Legacy host response format expected by old daemons.
///
/// Old daemons parse the response as `HostWithServicesRequest` (they used the
/// same type for request and response), expecting `host` and `services` fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyHostWithServicesResponse {
    pub host: LegacyHostResponse,
    pub services: Vec<Service>,
}

/// Legacy host response object matching old daemon expectations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyHostResponse {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub name: String,
    pub network_id: Uuid,
    pub hostname: Option<String>,
    pub description: Option<String>,
    pub hidden: bool,
    pub tags: Vec<Uuid>,

    // Embedded children (old format expected these on host)
    pub interfaces: Vec<Interface>,
    pub ports: Vec<Port>,
    /// Service IDs (old format expected just UUIDs here)
    pub services: Vec<Uuid>,

    // Legacy fields - must be included for old daemons
    pub target: LegacyTarget,
    pub source: serde_json::Value,
}

/// Legacy target field - always "None" in new responses
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LegacyTarget {
    #[serde(rename = "type")]
    pub target_type: String,
}

impl LegacyHostWithServicesResponse {
    /// Build legacy response from new HostResponse format.
    ///
    /// Transforms the flat response structure back to the nested format
    /// that old daemons expect.
    pub fn from_host_response(response: HostResponse) -> Self {
        let service_ids: Vec<Uuid> = response.services.iter().map(|s| s.id).collect();

        LegacyHostWithServicesResponse {
            host: LegacyHostResponse {
                id: response.id,
                created_at: response.created_at,
                updated_at: response.updated_at,
                name: response.name,
                network_id: response.network_id,
                hostname: response.hostname,
                description: response.description,
                hidden: response.hidden,
                tags: response.tags,
                interfaces: response.interfaces,
                ports: response.ports,
                services: service_ids,
                target: LegacyTarget {
                    target_type: "None".to_string(),
                },
                source: serde_json::json!({"type": "Discovery", "metadata": []}),
            },
            services: response.services,
        }
    }
}

/// Request body type that accepts both new and legacy formats.
///
/// Uses `#[serde(untagged)]` to try deserializing as New first, then Legacy.
/// This allows the same endpoint to handle both formats transparently.
#[derive(Debug, Clone)]
pub enum HostCreateRequestBody {
    /// New format from updated daemons and API users
    New(super::api::CreateHostRequest),
    /// Legacy format from old daemons
    Legacy(LegacyHostWithServicesRequest),
}

impl<'de> Deserialize<'de> for HostCreateRequestBody {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;

        // Try new format first
        match serde_json::from_value::<super::api::CreateHostRequest>(value.clone()) {
            Ok(new) => return Ok(Self::New(new)),
            Err(e) => tracing::debug!("Not new format: {}", e),
        }

        // Try legacy format
        match serde_json::from_value::<LegacyHostWithServicesRequest>(value.clone()) {
            Ok(legacy) => return Ok(Self::Legacy(legacy)),
            Err(e) => tracing::warn!("Legacy format parse error: {}", e),
        }

        // Neither format matched - provide helpful error
        tracing::warn!(payload = %value, "Invalid host create request format");
        Err(serde::de::Error::custom(
            "Invalid request format: could not parse as CreateHostRequest or legacy HostWithServicesRequest",
        ))
    }
}

/// Response type that serializes as the appropriate format based on request type.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)] // Temporary backwards-compat module, will be removed
pub enum HostCreateResponse {
    /// New format response
    New(HostResponse),
    /// Legacy format response for old daemons
    Legacy(LegacyHostWithServicesResponse),
}
