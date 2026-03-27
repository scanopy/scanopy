use chrono::{DateTime, Utc};
use mac_address::MacAddress;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::{
    bindings::r#impl::base::{Binding, BindingBase, BindingType},
    credentials::r#impl::types::CredentialAssignment,
    hosts::r#impl::{
        base::{Host, HostBase},
        virtualization::HostVirtualization,
    },
    if_entries::r#impl::base::{IfAdminStatus, IfEntry, IfEntryBase, IfOperStatus},
    interfaces::r#impl::base::{Interface, InterfaceBase},
    ports::r#impl::base::{Port, PortBase, PortConfig, PortType, TransportProtocol},
    services::r#impl::{
        base::{Service, ServiceBase},
        definitions::ServiceDefinition,
        virtualization::ServiceVirtualization,
    },
    shared::position::PositionedInput,
    shared::types::entities::EntitySource,
};

// =============================================================================
// CONFLICT BEHAVIOR
// =============================================================================

/// How to handle host creation when a matching host already exists
/// (matched via interface MAC address or subnet+IP).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictBehavior {
    /// Return an error if a matching host is found.
    /// Used for API users who should edit the existing host instead.
    Error,
    /// Upsert: update the existing host with new data.
    /// Used for daemon discovery which is inherently rediscovering and adding data to the same host
    Upsert,
}

// =============================================================================
// INTERNAL API (daemon discovery)
// =============================================================================

/// Request type for daemon discovery - accepts full entities with IDs.
/// Used internally by daemons for host creation/upsert, NOT the external API.
/// This supports the discovery workflow where daemons manage entity IDs.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DiscoveryHostRequest {
    pub host: Host,
    pub interfaces: Vec<Interface>,
    pub ports: Vec<Port>,
    pub services: Vec<Service>,
    /// SNMP interface entries (ifTable data) - optional, populated when SNMP is enabled
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub if_entries: Vec<crate::server::if_entries::r#impl::base::IfEntry>,
}

// =============================================================================
// EXTERNAL API - CONSOLIDATED INPUT TYPES
// =============================================================================

/// Input for creating or updating an interface.
/// Used in both CreateHostRequest and UpdateHostRequest.
/// Client must provide a UUID for the interface.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InterfaceInput {
    /// Client-provided UUID for this interface
    pub id: Uuid,
    pub subnet_id: Uuid,
    #[schema(value_type = String)]
    pub ip_address: IpAddr,
    #[schema(value_type = Option<String>)]
    pub mac_address: Option<MacAddress>,
    pub name: Option<String>,
    /// Position in the host's interface list (for ordering).
    /// If omitted on create: appends to end of list.
    /// If omitted on update: existing interfaces keep their positions; new interfaces append.
    /// Must be all specified or all omitted across all interfaces in the request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<i32>,
}

impl InterfaceInput {
    /// Convert to Interface entity with the given host_id and network_id.
    /// Position must be resolved before calling this (via `resolve_and_validate_input_positions`).
    pub fn into_interface(self, host_id: Uuid, network_id: Uuid) -> Interface {
        let now = chrono::Utc::now();
        Interface {
            id: self.id,
            created_at: now,
            updated_at: now,
            base: InterfaceBase {
                network_id,
                host_id,
                subnet_id: self.subnet_id,
                ip_address: self.ip_address,
                mac_address: self.mac_address,
                name: self.name,
                position: self.position.unwrap_or(0),
            },
        }
    }
}

impl PositionedInput for InterfaceInput {
    fn position(&self) -> Option<i32> {
        self.position
    }

    fn set_position(&mut self, position: i32) {
        self.position = Some(position);
    }

    fn id(&self) -> Uuid {
        self.id
    }
}

/// Input for creating or updating a port.
/// Used in both CreateHostRequest and UpdateHostRequest.
/// Client must provide a UUID for the port.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PortInput {
    /// Client-provided UUID for this port
    pub id: Uuid,
    /// Port number (1-65535)
    pub number: u16,
    /// Transport protocol (Tcp or Udp)
    pub protocol: TransportProtocol,
}

impl PortInput {
    /// Convert to Port entity with the given host_id and network_id.
    pub fn into_port(self, host_id: Uuid, network_id: Uuid) -> Port {
        let now = chrono::Utc::now();
        Port {
            id: self.id,
            created_at: now,
            updated_at: now,
            base: PortBase {
                host_id,
                network_id,
                port_type: PortType::Custom(PortConfig {
                    number: self.number,
                    protocol: self.protocol,
                }),
            },
        }
    }
}

/// Input for creating or updating a service.
/// Used in both CreateHostRequest and UpdateHostRequest.
/// Client must provide a UUID for the service.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ServiceInput {
    /// Client-provided UUID for this service
    pub id: Uuid,
    /// Service definition ID (e.g., "Nginx", "PostgreSQL")
    #[schema(value_type = String)]
    pub service_definition: Box<dyn ServiceDefinition>,
    /// Display name for this service
    pub name: String,
    /// Bindings that associate this service with ports/interfaces
    #[serde(default)]
    pub bindings: Vec<BindingInput>,
    /// Container/VM virtualization info if applicable
    pub virtualization: Option<ServiceVirtualization>,
    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<Uuid>,
    /// Position in the host's service list (for ordering).
    /// If omitted on create: appends to end of list.
    /// If omitted on update: existing services keep their positions; new services append.
    /// Must be all specified or all omitted across all services in the request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<i32>,
}

impl ServiceInput {
    /// Convert to Service entity with the given host_id, network_id, and source.
    /// Position must be resolved before calling this (via `resolve_and_validate_input_positions`).
    pub fn into_service(self, host_id: Uuid, network_id: Uuid, source: EntitySource) -> Service {
        let now = chrono::Utc::now();
        let service_id = self.id;

        // Convert binding inputs to full bindings
        let bindings: Vec<Binding> = self
            .bindings
            .into_iter()
            .map(|b| b.into_binding(service_id, network_id))
            .collect();

        Service {
            id: self.id,
            created_at: now,
            updated_at: now,
            base: ServiceBase {
                host_id,
                network_id,
                service_definition: self.service_definition,
                name: self.name,
                bindings,
                virtualization: self.virtualization,
                source,
                tags: self.tags,
                position: self.position.unwrap_or(0),
            },
        }
    }
}

impl PositionedInput for ServiceInput {
    fn position(&self) -> Option<i32> {
        self.position
    }

    fn set_position(&mut self, position: i32) {
        self.position = Some(position);
    }

    fn id(&self) -> Uuid {
        self.id
    }
}

/// Input for creating or updating a binding within a service.
/// Used in both CreateHostRequest and UpdateHostRequest.
/// Client must provide a UUID for the binding.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum BindingInput {
    /// Bind to an interface (service is present at this interface without a specific port)
    #[schema(title = "Interface")]
    Interface {
        /// Client-provided UUID for this binding
        id: Uuid,
        interface_id: Uuid,
    },
    /// Bind to a port (optionally on a specific interface)
    #[schema(title = "Port")]
    Port {
        /// Client-provided UUID for this binding
        id: Uuid,
        port_id: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        /// null = bind to all interfaces
        interface_id: Option<Uuid>,
    },
}

impl BindingInput {
    /// Get the client-provided ID for this binding
    pub fn id(&self) -> Uuid {
        match self {
            BindingInput::Interface { id, .. } => *id,
            BindingInput::Port { id, .. } => *id,
        }
    }

    /// Convert to a full Binding with the given service_id and network_id.
    pub fn into_binding(self, service_id: Uuid, network_id: Uuid) -> Binding {
        let (id, binding_type) = match self {
            BindingInput::Interface { id, interface_id } => {
                (id, BindingType::Interface { interface_id })
            }
            BindingInput::Port {
                id,
                port_id,
                interface_id,
            } => (
                id,
                BindingType::Port {
                    port_id,
                    interface_id,
                },
            ),
        };

        Binding {
            id,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            base: BindingBase::new(service_id, network_id, binding_type),
        }
    }
}

// =============================================================================
// EXTERNAL API - IF ENTRY INPUT
// =============================================================================

/// Input for creating an SNMP interface entry (ifTable data).
/// Used in CreateHostRequest. Server assigns UUIDs since nothing references
/// IfEntry IDs at creation time (neighbor resolution is done server-side).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IfEntryInput {
    /// SNMP ifIndex - stable identifier within device
    pub if_index: i32,
    /// SNMP ifDescr - interface description (e.g., GigabitEthernet0/1)
    pub if_descr: String,
    /// SNMP ifAlias - user-configured description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub if_alias: Option<String>,
    /// SNMP ifType - IANAifType integer (6=ethernet, 24=loopback, etc.)
    #[serde(default)]
    pub if_type: Option<i32>,
    /// Interface speed in bits per second
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed_bps: Option<i64>,
    /// SNMP ifAdminStatus
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admin_status: Option<IfAdminStatus>,
    /// SNMP ifOperStatus
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oper_status: Option<IfOperStatus>,
    /// MAC address from SNMP ifPhysAddress
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    pub mac_address: Option<MacAddress>,
    /// Optional FK to Interface - links this SNMP port to its IP assignment
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interface_id: Option<Uuid>,
}

impl IfEntryInput {
    /// Convert to IfEntry entity with the given host_id and network_id.
    pub fn into_if_entry(self, host_id: Uuid, network_id: Uuid) -> IfEntry {
        let now = chrono::Utc::now();
        IfEntry {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id,
                network_id,
                if_index: self.if_index,
                if_descr: self.if_descr,
                if_name: None,
                if_alias: self.if_alias,
                if_type: self.if_type.unwrap_or(1), // 1 = other
                speed_bps: self.speed_bps,
                admin_status: self.admin_status.unwrap_or_default(),
                oper_status: self.oper_status.unwrap_or_default(),
                mac_address: self.mac_address,
                interface_id: self.interface_id,
                // Neighbor resolution fields - not set from API, resolved server-side
                neighbor: None,
                lldp_chassis_id: None,
                lldp_port_id: None,
                lldp_sys_name: None,
                lldp_port_desc: None,
                lldp_mgmt_addr: None,
                lldp_sys_desc: None,
                cdp_device_id: None,
                cdp_port_id: None,
                cdp_platform: None,
                cdp_address: None,
                fdb_macs: None,
            },
        }
    }
}

// =============================================================================
// EXTERNAL API - CREATE REQUEST
// =============================================================================

/// Request type for creating a host with its associated interfaces, ports, and services.
/// Server assigns `host_id`, `network_id`, and `source` to all children.
/// Client must provide UUIDs for all entities, enabling services to reference
/// interfaces/ports by ID in the same request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[schema(example = crate::server::shared::types::examples::create_host_request)]
pub struct CreateHostRequest {
    // Host fields
    #[validate(length(max = 100, message = "Name must be 100 characters or less"))]
    pub name: String,
    pub network_id: Uuid,
    pub hostname: Option<String>,
    #[validate(length(max = 500, message = "Description must be 500 characters or less"))]
    pub description: Option<String>,
    pub virtualization: Option<HostVirtualization>,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    #[schema(required)]
    pub tags: Vec<Uuid>,

    // SNMP System MIB fields
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sys_descr: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sys_object_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sys_location: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sys_contact: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub management_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chassis_id: Option<String>,
    #[serde(default)]
    pub credential_assignments: Vec<CredentialAssignment>,

    /// Interfaces to create with this host (client provides UUIDs)
    #[serde(default)]
    pub interfaces: Vec<InterfaceInput>,
    /// Ports to create with this host (client provides UUIDs)
    #[serde(default)]
    pub ports: Vec<PortInput>,
    /// Services to create with this host (can reference interfaces/ports by their UUIDs)
    #[serde(default)]
    pub services: Vec<ServiceInput>,
    /// SNMP interface entries (ifTable data) - server assigns UUIDs
    #[serde(default)]
    pub if_entries: Vec<IfEntryInput>,
}

// =============================================================================
// UPDATE REQUEST TYPE
// =============================================================================

/// Request type for updating a host with its children.
/// Uses the same input types as CreateHostRequest.
/// Server will sync children (create new, update existing, delete removed) only if provided.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct UpdateHostRequest {
    pub id: Uuid,
    #[validate(length(max = 100, message = "Name must be 100 characters or less"))]
    pub name: String,
    pub hostname: Option<String>,
    #[validate(length(max = 500, message = "Description must be 500 characters or less"))]
    pub description: Option<String>,
    pub virtualization: Option<HostVirtualization>,
    pub hidden: bool,
    #[serde(default)]
    #[schema(required)]
    pub tags: Vec<Uuid>,
    /// Optional: expected updated_at timestamp for optimistic locking.
    #[serde(default)]
    pub expected_updated_at: Option<DateTime<Utc>>,

    /// Interfaces to sync with this host.
    /// If Some, server will create/update/delete to match this list.
    /// If None, existing interfaces are preserved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interfaces: Option<Vec<InterfaceInput>>,

    /// Ports to sync with this host.
    /// If Some, server will create/update/delete to match this list.
    /// If None, existing ports are preserved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ports: Option<Vec<PortInput>>,

    /// Services to sync with this host.
    /// If Some, server will create/update/delete to match this list.
    /// If None, existing services are preserved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub services: Option<Vec<ServiceInput>>,

    /// Credential assignments for this host.
    /// If provided, replaces all existing credential assignments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_assignments: Option<Vec<CredentialAssignment>>,
}

// =============================================================================
// RESPONSE TYPE
// =============================================================================

/// Response type for host endpoints.
/// Includes children (interfaces, ports, services, if_entries).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = crate::server::shared::types::examples::host_response)]
pub struct HostResponse {
    // Host identity
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // Host fields
    pub name: String,
    pub network_id: Uuid,
    pub hostname: Option<String>,
    pub description: Option<String>,
    pub source: EntitySource,
    pub virtualization: Option<HostVirtualization>,
    pub hidden: bool,
    pub tags: Vec<Uuid>,

    // SNMP System MIB fields
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sys_descr: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sys_object_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sys_location: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sys_contact: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub management_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chassis_id: Option<String>,
    #[serde(default)]
    pub credential_assignments: Vec<CredentialAssignment>,

    // Children (fetched by service layer)
    pub interfaces: Vec<Interface>,
    pub ports: Vec<Port>,
    pub services: Vec<Service>,
    /// SNMP ifTable entries
    pub if_entries: Vec<IfEntry>,
}

impl HostResponse {
    /// Convert HostResponse back to a Host entity (without children).
    /// Uses exhaustive destructuring to ensure compile error if HostResponse changes.
    pub fn to_host(&self) -> Host {
        // Exhaustive destructuring of HostResponse
        let HostResponse {
            id,
            created_at,
            updated_at,
            name,
            network_id,
            hostname,
            description,
            source,
            virtualization,
            hidden,
            tags,
            sys_descr,
            sys_object_id,
            sys_location,
            sys_contact,
            management_url,
            chassis_id,
            credential_assignments,
            interfaces: _,
            ports: _,
            services: _,
            if_entries: _,
        } = self;

        Host {
            id: *id,
            created_at: *created_at,
            updated_at: *updated_at,
            base: HostBase {
                name: name.clone(),
                network_id: *network_id,
                hostname: hostname.clone(),
                description: description.clone(),
                source: source.clone(),
                virtualization: virtualization.clone(),
                hidden: *hidden,
                tags: tags.clone(),
                sys_descr: sys_descr.clone(),
                sys_object_id: sys_object_id.clone(),
                sys_location: sys_location.clone(),
                sys_contact: sys_contact.clone(),
                management_url: management_url.clone(),
                chassis_id: chassis_id.clone(),
                sys_name: None,
                manufacturer: None,
                model: None,
                serial_number: None,
                credential_assignments: credential_assignments.clone(),
            },
        }
    }

    /// Build HostResponse from a Host and its children.
    /// Uses exhaustive destructuring to ensure compile error if Host/HostBase changes.
    pub fn from_host_with_children(
        host: Host,
        interfaces: Vec<Interface>,
        ports: Vec<Port>,
        services: Vec<Service>,
        if_entries: Vec<IfEntry>,
    ) -> Self {
        // Exhaustive destructuring of Host
        let Host {
            id,
            created_at,
            updated_at,
            base,
        } = host;

        // Exhaustive destructuring of HostBase
        // If a field is added to HostBase, this will fail to compile
        let crate::server::hosts::r#impl::base::HostBase {
            name,
            network_id,
            hostname,
            description,
            source,
            virtualization,
            hidden,
            tags,
            sys_descr,
            sys_object_id,
            sys_location,
            sys_contact,
            management_url,
            chassis_id,
            sys_name: _,
            manufacturer: _,
            model: _,
            serial_number: _,
            credential_assignments,
        } = base;

        Self {
            id,
            created_at,
            updated_at,
            name,
            network_id,
            hostname,
            description,
            source,
            virtualization,
            hidden,
            tags,
            sys_descr,
            sys_object_id,
            sys_location,
            sys_contact,
            management_url,
            chassis_id,
            credential_assignments,
            interfaces,
            ports,
            services,
            if_entries,
        }
    }
}
