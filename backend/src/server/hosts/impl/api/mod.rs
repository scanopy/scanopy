//! The shapes a host takes on the wire.
//!
//! Request and response types here; the conversions between `HostResponse` and `Host` are in
//! [`conversion`], which is the half that grew when every attribute started carrying its source.
//! Both directions destructure exhaustively, so a field added to either side fails to compile
//! until it is handled in both — which is what keeps a column from being collected, stored and
//! then silently dropped on the way back out.

mod conversion;

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
        attributes::{
            HostChassisIdValue, HostFirmwareRevisionValue, HostHostnameValue,
            HostManagementUrlValue, HostManufacturerValue, HostModelValue, HostSerialNumberValue,
            HostSoftwareRevisionValue, HostSysContactValue, HostSysDescrValue,
            HostSysLocationValue, HostSysNameValue, HostSysObjectIdValue,
        },
        base::{Host, HostBase},
        name::host_name_from_parts,
        virtualization::HostVirtualization,
    },
    interfaces::r#impl::base::{
        IfAdminStatus, IfOperStatus, Interface, InterfaceBase, InterfaceDataComplete,
    },
    interfaces::r#impl::wire::DiscoveryInterface,
    ip_addresses::r#impl::base::{IPAddress, IPAddressBase, MacEvidence, MacEvidenceValue},
    ports::r#impl::base::{Port, PortBase, PortConfig, PortType, TransportProtocol},
    services::r#impl::{
        base::{Service, ServiceBase},
        definitions::ServiceDefinition,
        virtualization::ServiceVirtualization,
    },
    shared::attribution::{self as attribution, AttributeSource, Attributed},
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
///
/// ## Backwards compatibility (daemons < v0.16.0)
///
/// Pre-v0.16.0 daemons send the old field layout:
///   - `interfaces` → IPAddress data (now `ip_addresses`)
///   - `if_entries` → SNMP Interface data (now `interfaces`)
///
/// The custom deserializer detects the old layout (missing `ip_addresses` field)
/// and remaps fields automatically. This can be removed once all daemons are ≥ v0.16.0.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(into = "DiscoveryHostRequestWire")]
pub struct DiscoveryHostRequest {
    /// The host as observed by the daemon.
    pub host: Host,
    /// IP addresses observed on the host.
    pub ip_addresses: Vec<IPAddress>,
    /// Open ports observed on the host.
    pub ports: Vec<Port>,
    /// Services identified on the host.
    pub services: Vec<Service>,
    /// SNMP interface entries (ifTable data) - optional, populated when SNMP is enabled.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interfaces: Vec<crate::server::interfaces::r#impl::base::Interface>,
    /// Integration-derived subnets (e.g., Docker bridge networks) — created during
    /// create_with_children after service dedup so virtualization.service_id is correct.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subnets: Vec<crate::server::subnets::r#impl::base::Subnet>,
    /// Whether `interfaces` is a complete, authoritative ifTable. When false (a partial SNMP walk
    /// cut short by timeout/error), the server must NOT prune interfaces missing from this scan —
    /// otherwise a transient partial walk tears down the host's L2 topology (#649). Daemons that
    /// predate this field omit it; it defaults to true so their behavior is unchanged.
    #[serde(default = "default_interfaces_complete")]
    pub interfaces_complete: bool,
    /// Which groups of per-interface data (LLDP, CDP, FDB, VLAN membership) this scan read in
    /// full. A group the daemon could not finish reading must not overwrite what is already
    /// stored: a cut-short walk returns the same empty result as a device with nothing to report,
    /// and for the neighbour fields that also drops the row out of L2 resolution for good.
    /// Daemons predating this field omit it; it defaults to all-complete so they behave as before.
    #[serde(default)]
    pub interface_data_complete: InterfaceDataComplete,
    /// Whether any interface in this submission arrived in a wire shape a current daemon no
    /// longer produces — today, the pre-#701 scalar LLDP/CDP fields (see
    /// [`DiscoveryInterface`](crate::server::interfaces::r#impl::wire::DiscoveryInterface)).
    ///
    /// Set by the deserializer, never by a daemon: it is `skip`ped on the wire in both
    /// directions and exists only to carry the observation from the boundary, where the raw
    /// shape is visible, to the discovery session, where a warning can be attached. Read once,
    /// in the discovery handlers, then discarded.
    #[serde(skip)]
    pub superseded_wire_shape: bool,
}

/// Serde default for `interfaces_complete`: absent (old daemon) ⇒ treat as a complete/authoritative
/// interface set, preserving pre-#649-fix behavior. Only a new daemon that explicitly reports a
/// partial walk sends `false`.
fn default_interfaces_complete() -> bool {
    true
}

/// Wire format for DiscoveryHostRequest — handles both old and new field layouts.
/// Backwards compat for daemons < v0.16.0 that send `interfaces` for IPAddress data
/// and `if_entries` for SNMP Interface data.
#[derive(Deserialize, Serialize)]
struct DiscoveryHostRequestWire {
    host: Host,
    /// New field name (v0.16.0+). Missing in old payloads.
    #[serde(default)]
    ip_addresses: Option<Vec<IPAddress>>,
    ports: Vec<Port>,
    services: Vec<Service>,
    /// In new payloads: SNMP Interface data.
    /// In old payloads (< v0.16.0): IPAddress data (remapped by From impl).
    #[serde(default)]
    interfaces: Vec<serde_json::Value>,
    /// Old field name for SNMP Interface data (< v0.16.0). Absent in new payloads.
    ///
    /// Untyped for the same reason `interfaces` is: both are read as `DiscoveryInterface` in the
    /// branch below, and that type is deserialize-only while this struct is also the outgoing
    /// wire format. A daemon this old also predates #701, so its interfaces carry the scalar
    /// LLDP/CDP shape and need the same translation the new-format branch applies.
    #[serde(default)]
    if_entries: Vec<serde_json::Value>,
    #[serde(default)]
    subnets: Vec<crate::server::subnets::r#impl::base::Subnet>,
    #[serde(default = "default_interfaces_complete")]
    interfaces_complete: bool,
    #[serde(default)]
    interface_data_complete: InterfaceDataComplete,
}

impl From<DiscoveryHostRequest> for DiscoveryHostRequestWire {
    fn from(req: DiscoveryHostRequest) -> Self {
        Self {
            host: req.host,
            ip_addresses: Some(req.ip_addresses),
            ports: req.ports,
            services: req.services,
            interfaces: req
                .interfaces
                .into_iter()
                .map(|i| serde_json::to_value(i).unwrap())
                .collect(),
            if_entries: vec![],
            subnets: req.subnets,
            interfaces_complete: req.interfaces_complete,
            interface_data_complete: req.interface_data_complete,
        }
    }
}

impl<'de> serde::Deserialize<'de> for DiscoveryHostRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = DiscoveryHostRequestWire::deserialize(deserializer)?;

        /// Read one submission's interfaces, translating any superseded per-interface wire shape
        /// into the current one and reporting whether it had to.
        fn read_interfaces<E: serde::de::Error>(
            raw: Vec<serde_json::Value>,
        ) -> Result<
            (
                Vec<crate::server::interfaces::r#impl::base::Interface>,
                bool,
            ),
            E,
        > {
            let wire: Vec<DiscoveryInterface> = raw
                .into_iter()
                .map(|v| serde_json::from_value(v).map_err(serde::de::Error::custom))
                .collect::<Result<_, E>>()?;

            let superseded = wire
                .iter()
                .any(DiscoveryInterface::submitted_legacy_neighbor_evidence);

            Ok((wire.into_iter().map(Into::into).collect(), superseded))
        }

        if let Some(ip_addresses) = wire.ip_addresses {
            // New format (v0.16.0+): ip_addresses present, interfaces = SNMP data
            let (interfaces, superseded_wire_shape) = read_interfaces(wire.interfaces)?;

            Ok(DiscoveryHostRequest {
                host: wire.host,
                ip_addresses,
                ports: wire.ports,
                services: wire.services,
                interfaces,
                subnets: wire.subnets,
                interfaces_complete: wire.interfaces_complete,
                interface_data_complete: wire.interface_data_complete,
                superseded_wire_shape,
            })
        } else {
            // Old format (< v0.16.0): interfaces = IPAddress data, if_entries = SNMP data
            let ip_addresses: Vec<IPAddress> = wire
                .interfaces
                .into_iter()
                .map(|v| serde_json::from_value(v).map_err(serde::de::Error::custom))
                .collect::<Result<_, _>>()?;

            let (interfaces, _) = read_interfaces(wire.if_entries)?;

            Ok(DiscoveryHostRequest {
                host: wire.host,
                ip_addresses,
                ports: wire.ports,
                services: wire.services,
                interfaces,
                subnets: wire.subnets,
                interfaces_complete: wire.interfaces_complete,
                interface_data_complete: wire.interface_data_complete,
                // Reaching this branch at all is the stronger signal: only a pre-0.16.0 daemon
                // sends this layout, whatever its interfaces happened to carry.
                superseded_wire_shape: true,
            })
        }
    }
}

#[cfg(test)]
mod discovery_request_interfaces_complete_tests {
    use super::*;

    fn request_with(interfaces_complete: bool) -> DiscoveryHostRequest {
        DiscoveryHostRequest {
            host: Host::default(),
            ip_addresses: vec![],
            ports: vec![],
            services: vec![],
            interfaces: vec![],
            subnets: vec![],
            interfaces_complete,
            interface_data_complete: InterfaceDataComplete::default(),
            superseded_wire_shape: false,
        }
    }

    #[test]
    fn absent_field_defaults_to_complete_for_old_daemons() {
        // GH #649: daemons predating this field omit it. It must default to true so their behavior
        // is identical to before the fix (server still prunes) — the no-regression contract.
        let mut json = serde_json::to_value(request_with(false)).expect("serializes");
        json.as_object_mut()
            .expect("wire is a JSON object")
            .remove("interfaces_complete");
        let parsed: DiscoveryHostRequest =
            serde_json::from_value(json).expect("deserializes without the field");
        assert!(
            parsed.interfaces_complete,
            "an absent interfaces_complete must default to true (old-daemon compatibility)"
        );
    }

    #[test]
    fn explicit_incomplete_survives_wire_round_trip() {
        // A new daemon signalling a partial walk must reach the server as false, or the prune gate
        // can't protect the L2 topology.
        let json = serde_json::to_value(request_with(false)).expect("serializes");
        let parsed: DiscoveryHostRequest = serde_json::from_value(json).expect("deserializes");
        assert!(!parsed.interfaces_complete);
    }
}

// =============================================================================
// EXTERNAL API - CONSOLIDATED INPUT TYPES
// =============================================================================

/// Input for creating or updating an interface.
/// Used in both CreateHostRequest and UpdateHostRequest.
/// Client must provide a UUID for the interface.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IPAddressInput {
    /// Client-provided UUID for this interface
    pub id: Uuid,
    /// The subnet this entity belongs to.
    pub subnet_id: Uuid,
    /// IPv4 or IPv6 address.
    #[schema(value_type = String, example = "192.168.1.10")]
    pub ip_address: IpAddr,
    /// MAC address, when known.
    #[schema(value_type = Option<String>, pattern = r"^(?:[0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}$", example = "a4:bb:6d:12:34:56")]
    pub mac_address: Option<MacAddress>,
    /// Human-facing name for this IP address.
    pub name: Option<String>,
    /// Position in the host's interface list (for ordering).
    /// If omitted on create: appends to end of list.
    /// If omitted on update: existing ip_addresses keep their positions; new ip_addresses append.
    /// Must be all specified or all omitted across all ip_addresses in the request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<i32>,
}

impl IPAddressInput {
    /// Convert to IPAddress entity with the given host_id and network_id.
    /// Position must be resolved before calling this (via `resolve_and_validate_input_positions`).
    pub fn into_ip_address(self, host_id: Uuid, network_id: Uuid) -> IPAddress {
        let now = chrono::Utc::now();
        IPAddress {
            id: self.id,
            created_at: now,
            updated_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            base: IPAddressBase {
                network_id,
                host_id,
                subnet_id: self.subnet_id,
                ip_address: self.ip_address,
                // A MAC arriving through the API is one a person entered, which is the only way
                // this path is reached — discovery submits through `DiscoveryHostRequest`.
                mac_address: self
                    .mac_address
                    .map(|m| MacEvidence::new(MacEvidenceValue(m), AttributeSource::Manual)),
                name: self.name,
                position: self.position.unwrap_or(0),
            },
        }
    }
}

impl PositionedInput for IPAddressInput {
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
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
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
    /// Container identity (name, id, compose project) if this service is a container.
    pub virtualization_metadata: Option<ServiceVirtualization>,
    /// The container runtime service hosting this container, if any.
    #[serde(default)]
    pub virtualization_service_id: Option<Uuid>,
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
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: self.id,
            created_at: now,
            updated_at: now,
            base: ServiceBase {
                host_id,
                network_id,
                service_definition: self.service_definition,
                name: self.name,
                bindings,
                virtualization_metadata: self.virtualization_metadata,
                virtualization_service_id: self.virtualization_service_id,
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
    #[schema(title = "IPAddress")]
    IPAddress {
        /// Client-provided UUID for this binding
        id: Uuid,
        /// The IP address the service is present at.
        ip_address_id: Uuid,
    },
    /// Bind to a port (optionally on a specific ip_address)
    #[schema(title = "Port")]
    Port {
        /// Client-provided UUID for this binding
        id: Uuid,
        /// The port the service listens on.
        port_id: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        /// null = bind to all ip_addresses
        ip_address_id: Option<Uuid>,
    },
}

impl BindingInput {
    /// Get the client-provided ID for this binding
    pub fn id(&self) -> Uuid {
        match self {
            BindingInput::IPAddress { id, .. } => *id,
            BindingInput::Port { id, .. } => *id,
        }
    }

    /// Convert to a full Binding with the given service_id and network_id.
    pub fn into_binding(self, service_id: Uuid, network_id: Uuid) -> Binding {
        let (id, binding_type) = match self {
            BindingInput::IPAddress { id, ip_address_id } => {
                (id, BindingType::IPAddress { ip_address_id })
            }
            BindingInput::Port {
                id,
                port_id,
                ip_address_id,
            } => (
                id,
                BindingType::Port {
                    port_id,
                    ip_address_id,
                },
            ),
        };

        let now = chrono::Utc::now();
        Binding {
            id,
            created_at: now,
            updated_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            base: BindingBase::new(service_id, network_id, binding_type),
        }
    }
}

// =============================================================================
// EXTERNAL API - IF ENTRY INPUT
// =============================================================================

/// Input for manually creating or updating an interface entry.
/// Used in `UpdateHostRequest`, synced the same way as `ip_addresses`/`ports`/`services`:
/// a client-provided `id` that already exists on this host is updated, one that doesn't is
/// created, and an existing row missing from the list is deleted.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InterfaceInput {
    /// Client-provided UUID for this interface.
    pub id: Uuid,
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
    #[schema(value_type = Option<String>, pattern = r"^(?:[0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}$", example = "a4:bb:6d:12:34:56")]
    pub mac_address: Option<MacAddress>,
    /// Optional FK to Interface - links this SNMP port to its IP assignment
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ip_address_id: Option<Uuid>,
}

impl InterfaceInput {
    /// Convert to Interface entity with the given host_id and network_id.
    pub fn into_interface(self, host_id: Uuid, network_id: Uuid) -> Interface {
        let now = chrono::Utc::now();
        Interface {
            id: self.id,
            created_at: now,
            updated_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            display_name: None,
            base: InterfaceBase {
                host_id,
                network_id,
                if_index: Some(self.if_index),
                if_descr: Some(self.if_descr),
                if_name: None,
                if_alias: self.if_alias,
                // Straight through. These were coerced to "other"/Up/Up, which recorded a
                // guess as though the caller had reported it; absent now means absent.
                if_type: self.if_type,
                speed_bps: self.speed_bps,
                admin_status: self.admin_status,
                oper_status: self.oper_status,
                // Interfaces reach this path only through the API, so a MAC here was entered.
                mac_address: self
                    .mac_address
                    .map(|m| MacEvidence::new(MacEvidenceValue(m), AttributeSource::Manual)),
                ip_address_id: self.ip_address_id,
                // Not an SNMP walk — no ipAddrTable to read, so this signal is unavailable here.
                ip_configured: false,
                // Neighbor resolution — not set from API, resolved server-side.
                neighbor_candidates: Vec::new(),
                fdb_macs: None,
                native_vlan_id: None,
                vlan_ids: None,
            },
        }
    }
}

// =============================================================================
// EXTERNAL API - CREATE REQUEST
// =============================================================================

/// Request type for creating a host with its associated ip_addresses, ports, and services.
/// Server assigns `host_id`, `network_id`, and `source` to all children.
/// Client must provide UUIDs for all entities, enabling services to reference
/// ip_addresses/ports by ID in the same request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[schema(example = crate::server::shared::types::examples::create_host_request)]
pub struct CreateHostRequest {
    // Host fields
    /// Human-facing name for the host.
    #[validate(length(max = 100, message = "Name must be 100 characters or less"))]
    pub name: String,
    /// The network this entity belongs to.
    pub network_id: Uuid,
    /// Hostname as resolved or reported by the host.
    pub hostname: Option<String>,
    /// Free-text notes about the host.
    #[validate(length(max = 500, message = "Description must be 500 characters or less"))]
    pub description: Option<String>,
    /// How the host is virtualized, when it is a VM or container guest.
    pub virtualization_metadata: Option<HostVirtualization>,
    /// The hypervisor service this VM runs on.
    #[serde(default)]
    pub virtualization_service_id: Option<Uuid>,
    /// Hide the host from topology views without deleting it.
    #[serde(default)]
    pub hidden: bool,
    /// Tags assigned to this entity.
    #[serde(default)]
    #[schema(required)]
    pub tags: Vec<Uuid>,

    // SNMP System MIB fields
    /// SNMP sysDescr — the device's own description of itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sys_descr: Option<String>,
    /// SNMP sysObjectID — the vendor's identifier for the device model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sys_object_id: Option<String>,
    /// SNMP sysLocation — physical location as configured on the device.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sys_location: Option<String>,
    /// SNMP sysContact — administrative contact as configured on the device.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sys_contact: Option<String>,
    /// Link to the host's own management interface.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub management_url: Option<String>,
    /// LLDP chassis identifier, used to match the host to its neighbours.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chassis_id: Option<String>,
    /// Credentials to scan this host with.
    #[serde(default)]
    pub credential_assignments: Vec<CredentialAssignment>,

    /// Interfaces to create with this host (client provides UUIDs)
    #[serde(default)]
    pub ip_addresses: Vec<IPAddressInput>,
    /// Ports to create with this host (client provides UUIDs)
    #[serde(default)]
    pub ports: Vec<PortInput>,
    /// Services to create with this host (can reference ip_addresses/ports by their UUIDs)
    #[serde(default)]
    pub services: Vec<ServiceInput>,
    /// SNMP interface entries (ifTable data) - server assigns UUIDs
    #[serde(default)]
    pub interfaces: Vec<InterfaceInput>,
}

// =============================================================================
// UPDATE REQUEST TYPE
// =============================================================================

/// Request type for updating a host with its children.
/// Uses the same input types as CreateHostRequest.
/// Server will sync children (create new, update existing, delete removed) only if provided.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct UpdateHostRequest {
    /// Server-assigned unique identifier.
    pub id: Uuid,
    /// Human-facing name for the host.
    #[validate(length(max = 100, message = "Name must be 100 characters or less"))]
    pub name: String,
    /// Hostname as resolved or reported by the host.
    pub hostname: Option<String>,
    /// Free-text notes about the host.
    #[validate(length(max = 500, message = "Description must be 500 characters or less"))]
    pub description: Option<String>,
    /// How the host is virtualized, when it is a VM or container guest.
    pub virtualization_metadata: Option<HostVirtualization>,
    /// The hypervisor service this VM runs on.
    #[serde(default)]
    pub virtualization_service_id: Option<Uuid>,
    /// Hide the host from topology views without deleting it.
    pub hidden: bool,
    /// Tags assigned to this entity.
    #[serde(default)]
    #[schema(required)]
    pub tags: Vec<Uuid>,
    /// Optional: expected updated_at timestamp for optimistic locking.
    #[serde(default)]
    pub expected_updated_at: Option<DateTime<Utc>>,

    /// Interfaces to sync with this host.
    /// If Some, server will create/update/delete to match this list.
    /// If None, existing ip_addresses are preserved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ip_addresses: Option<Vec<IPAddressInput>>,

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

    /// Interfaces to sync with this host.
    /// If Some, server will create/update/delete to match this list.
    /// If None, existing interfaces are preserved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interfaces: Option<Vec<InterfaceInput>>,

    /// Credential assignments for this host.
    /// If provided, replaces all existing credential assignments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_assignments: Option<Vec<CredentialAssignment>>,
}

// =============================================================================
// RESPONSE TYPE
// =============================================================================

/// Response type for host endpoints.
/// Includes children (ip_addresses, ports, services, interfaces).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = crate::server::shared::types::examples::host_response)]
pub struct HostResponse {
    // Host identity
    /// Server-assigned unique identifier.
    pub id: Uuid,
    /// When this record was first created.
    pub created_at: DateTime<Utc>,
    /// When this record was last modified.
    pub updated_at: DateTime<Utc>,
    /// Last time discovery observed this host. User-facing (drives the "Last
    /// seen" column and the stale badge), which is why it is carried here while
    /// the rest of the SCD2/audit columns are not.
    pub last_seen_at: DateTime<Utc>,

    // Host fields
    /// Human-facing name for the host.
    pub name: String,
    /// What to call this host when `name` is empty: its hostname, sysName, chassis id or first
    /// address, whichever it has. `None` when nothing identifies it.
    ///
    /// Read-only and separate from `name` rather than folded into it. `name` is what a person
    /// typed and what the editor writes back, so filling it with a fallback would turn a chassis
    /// id into a name the next save persists. Computed from [`Host::display_name`], the same
    /// ladder topology titles a host container with, so a host cannot be called one thing in the
    /// list and another on the map.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(read_only)]
    pub display_name: Option<String>,
    /// Which rung of the ladder produced `display_name`. `None` exactly when `display_name` is.
    ///
    /// Resolved from the same [`Host::name_ladder`] call as `display_name`, so the UI can say
    /// where a host's title came from without walking the rungs itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(read_only)]
    pub display_name_rung: Option<crate::server::hosts::r#impl::name_ladder::HostNameRung>,
    /// Every rung of the display-name ladder for this host, highest first, with what each holds.
    #[serde(default)]
    #[schema(read_only)]
    pub name_ladder: Vec<crate::server::hosts::r#impl::name_ladder::HostNameLadderEntry>,
    /// What produced `name`. Read-only: it is decided by whoever supplied the name, not by the
    /// caller.
    #[serde(default)]
    #[schema(read_only)]
    pub name_source: AttributeSource,
    /// The network this entity belongs to.
    pub network_id: Uuid,
    /// Hostname as resolved or reported by the host.
    pub hostname: Option<String>,
    /// What produced `hostname`: a PTR lookup, the host's own OS, a controller, mDNS, or a person.
    /// Read-only: decided by whichever source read it.
    #[serde(default)]
    #[schema(read_only)]
    pub hostname_source: AttributeSource,
    /// Free-text notes about the host.
    pub description: Option<String>,
    /// How this host came to be known — discovered, imported, or created by hand.
    pub source: EntitySource,
    /// How the host is virtualized, when it is a VM or container guest.
    #[serde(
        default,
        deserialize_with = "crate::server::shared::types::api::deserialize_lenient_option"
    )]
    pub virtualization_metadata: Option<HostVirtualization>,
    /// The hypervisor service this VM runs on.
    #[serde(default)]
    pub virtualization_service_id: Option<Uuid>,
    /// Whether the host is hidden from topology views.
    pub hidden: bool,
    /// Tags assigned to this entity.
    pub tags: Vec<Uuid>,

    // SNMP System MIB fields
    /// SNMP sysDescr — the device's own description of itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sys_descr: Option<String>,
    /// What produced SNMP sysDescr. Read-only: decided by whichever source read it.
    #[serde(default)]
    #[schema(read_only)]
    pub sys_descr_source: AttributeSource,
    /// SNMP sysObjectID — the vendor's identifier for the device model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sys_object_id: Option<String>,
    /// What produced SNMP sysObjectID. Read-only: decided by whichever source read it.
    #[serde(default)]
    #[schema(read_only)]
    pub sys_object_id_source: AttributeSource,
    /// SNMP sysLocation — physical location as configured on the device.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sys_location: Option<String>,
    /// What produced SNMP sysLocation. Read-only: decided by whichever source read it.
    #[serde(default)]
    #[schema(read_only)]
    pub sys_location_source: AttributeSource,
    /// SNMP sysContact — administrative contact as configured on the device.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sys_contact: Option<String>,
    /// What produced SNMP sysContact. Read-only: decided by whichever source read it.
    #[serde(default)]
    #[schema(read_only)]
    pub sys_contact_source: AttributeSource,
    /// Link to the host's own management interface.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub management_url: Option<String>,
    /// What produced the management URL. Read-only: decided by whichever source read it.
    #[serde(default)]
    #[schema(read_only)]
    pub management_url_source: AttributeSource,
    /// LLDP chassis identifier, used to match the host to its neighbours.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chassis_id: Option<String>,
    /// What produced the LLDP chassis identifier. Read-only: decided by whichever source read it.
    #[serde(default)]
    #[schema(read_only)]
    pub chassis_id_source: AttributeSource,
    /// SNMP sysName.0 — the administratively-assigned hostname. Read-only: discovery collects it
    /// from the device, so neither create nor update accepts it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(read_only)]
    pub sys_name: Option<String>,
    /// What produced SNMP sysName. Read-only: decided by whichever source read it.
    #[serde(default)]
    #[schema(read_only)]
    pub sys_name_source: AttributeSource,
    /// ENTITY-MIB entPhysicalMfgName — hardware manufacturer. Read-only, as above.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(read_only)]
    pub manufacturer: Option<String>,
    /// What produced the manufacturer. Read-only: decided by whichever source read it.
    #[serde(default)]
    #[schema(read_only)]
    pub manufacturer_source: AttributeSource,
    /// ENTITY-MIB entPhysicalModelName — hardware model. Read-only, as above.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(read_only)]
    pub model: Option<String>,
    /// What produced the model. Read-only: decided by whichever source read it.
    #[serde(default)]
    #[schema(read_only)]
    pub model_source: AttributeSource,
    /// ENTITY-MIB entPhysicalSerialNum — hardware serial number. Read-only, as above.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(read_only)]
    pub serial_number: Option<String>,
    /// What produced the serial number. Read-only: decided by whichever source read it.
    #[serde(default)]
    #[schema(read_only)]
    pub serial_number_source: AttributeSource,
    /// ENTITY-MIB entPhysicalFirmwareRev — firmware revision of the device. Read-only, as above.
    #[schema(required, read_only)]
    pub firmware_revision: Option<String>,
    /// What produced the firmware revision. Read-only: decided by whichever source read it.
    #[serde(default)]
    #[schema(read_only)]
    pub firmware_revision_source: AttributeSource,
    /// ENTITY-MIB entPhysicalSoftwareRev — software revision of the device. Read-only, as above.
    #[schema(required, read_only)]
    pub software_revision: Option<String>,
    /// What produced the software revision. Read-only: decided by whichever source read it.
    #[serde(default)]
    #[schema(read_only)]
    pub software_revision_source: AttributeSource,
    /// Credentials assigned to scan this host.
    #[serde(default)]
    pub credential_assignments: Vec<CredentialAssignment>,

    // Children (fetched by service layer)
    /// IP addresses on this host.
    pub ip_addresses: Vec<IPAddress>,
    /// Open ports on this host.
    pub ports: Vec<Port>,
    /// Services running on this host.
    pub services: Vec<Service>,
    /// SNMP ifTable entries
    pub interfaces: Vec<Interface>,
}
