use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, hash::Hash};
use strum_macros::{EnumDiscriminants, IntoStaticStr};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::shared::entities::ChangeTriggersTopologyStaleness;

/// The type of binding - either to an interface or to a port.
///
/// Bindings associate a service with network resources (ip_addresses/ports) on a host.
///
/// ## Validation Rules
///
/// - All bindings must reference ports/interfaces that belong to the same host as the service.
/// - Interface bindings conflict with port bindings on the same interface.
/// - A port binding on all ip_addresses (`ip_address_id: null`) conflicts with any interface binding.
/// - When a port binding with `ip_address_id: null` is created, it supersedes (removes) any
///   existing specific-interface bindings for the same port.
#[derive(
    Copy, Debug, Clone, Serialize, Deserialize, Eq, PartialEq, EnumDiscriminants, ToSchema,
)]
#[strum_discriminants(derive(IntoStaticStr))]
#[serde(tag = "type")]
pub enum BindingType {
    #[schema(title = "IPAddress")]
    #[serde(alias = "Interface")]
    /// IP address binding: Service is present at an IP address without a specific port.
    /// Used for non-port-bound services like gateways. Conflicts with port bindings on the same IP address.
    IPAddress {
        /// The IP address the service is present at.
        #[serde(alias = "interface_id")]
        ip_address_id: Uuid,
    },
    #[schema(title = "Port")]
    /// Port binding: Service listens on a specific port, optionally on a specific IP address.
    /// If `ip_address_id` is `null`, the service listens on this port across all IP addresses,
    /// which supersedes any specific-IP-address bindings for the same port.
    Port {
        /// The port the service listens on.
        port_id: Uuid,
        #[serde(skip_serializing_if = "Option::is_none", alias = "interface_id")]
        #[schema(required)]
        /// The IP address this port binding applies to. If `null`, the binding applies to all
        /// IP addresses on the host (and supersedes specific-IP-address bindings for this port).
        ip_address_id: Option<Uuid>,
    },
}

impl Default for BindingType {
    fn default() -> Self {
        BindingType::Port {
            port_id: Uuid::nil(),
            ip_address_id: Some(Uuid::nil()),
        }
    }
}

/// The base data for a Binding entity (everything except id, created_at, updated_at)
#[derive(Copy, Debug, Clone, Eq, Serialize, Deserialize, ToSchema, Validate)]
pub struct BindingBase {
    /// The service this entity refers to.
    pub service_id: Uuid,
    /// The site this entity belongs to.
    pub site_id: Uuid,
    /// What the service is bound to — a port, or an IP address on its own.
    #[serde(flatten)]
    pub binding_type: BindingType,
}

impl BindingBase {
    pub fn new(service_id: Uuid, site_id: Uuid, binding_type: BindingType) -> Self {
        Self {
            service_id,
            site_id,
            binding_type,
        }
    }

    /// Create a BindingBase without service/site (will be set by server)
    pub fn new_serviceless(binding_type: BindingType) -> Self {
        Self {
            service_id: Uuid::nil(),
            site_id: Uuid::nil(),
            binding_type,
        }
    }
}

impl Default for BindingBase {
    fn default() -> Self {
        Self::new_serviceless(BindingType::default())
    }
}

impl PartialEq for BindingBase {
    fn eq(&self, other: &Self) -> bool {
        self.binding_type == other.binding_type
    }
}

impl Hash for BindingBase {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.binding_type.hash(state);
    }
}

impl Hash for BindingType {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            BindingType::IPAddress { ip_address_id } => {
                "ip_address".hash(state);
                ip_address_id.hash(state);
            }
            BindingType::Port {
                port_id,
                ip_address_id,
            } => {
                "port".hash(state);
                port_id.hash(state);
                ip_address_id.hash(state);
            }
        }
    }
}

/// Association between a service and a port / interface that the service is listening on
#[derive(Copy, Debug, Clone, Eq, Serialize, Deserialize, ToSchema, Validate)]
#[schema(example = crate::server::shared::types::examples::binding)]
pub struct Binding {
    /// Server-assigned unique identifier.
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    /// When this record was first created.
    #[serde(default = "Utc::now")]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    /// When this record was last modified.
    #[serde(default = "Utc::now")]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    /// Start of the interval this revision was current for (SCD2 history).
    #[serde(default = "Utc::now")]
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
    #[serde(default = "Utc::now")]
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
    pub base: BindingBase,
}

impl Hash for Binding {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for Binding {
    fn eq(&self, other: &Self) -> bool {
        self.base == other.base
    }
}

impl Default for Binding {
    fn default() -> Self {
        Self::new_serviceless(BindingType::default())
    }
}

impl ChangeTriggersTopologyStaleness<Binding> for Binding {
    fn triggers_staleness(&self, other: Option<Binding>) -> bool {
        if let Some(other_binding) = other {
            self.base.binding_type != other_binding.base.binding_type
                || self.base.service_id != other_binding.base.service_id
        } else {
            true // New or deleted binding triggers staleness
        }
    }
}

impl Display for Binding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.base.binding_type {
            BindingType::IPAddress { ip_address_id } => {
                write!(f, "IP address binding {} -> {}", self.id, ip_address_id)
            }
            BindingType::Port {
                port_id,
                ip_address_id,
            } => {
                if let Some(addr_id) = ip_address_id {
                    write!(
                        f,
                        "Port binding {} -> {} (ip_address {})",
                        self.id, port_id, addr_id
                    )
                } else {
                    write!(
                        f,
                        "Port binding {} -> {} (all ip_addresses)",
                        self.id, port_id
                    )
                }
            }
        }
    }
}

impl Binding {
    pub fn new(base: BindingBase) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            base,
        }
    }

    /// Create a Binding with just a BindingType (service_id/site_id set to nil).
    /// Use this for bindings created during discovery before service assignment.
    pub fn new_serviceless(binding_type: BindingType) -> Self {
        Self::new(BindingBase::new_serviceless(binding_type))
    }

    // Convenience accessors
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn service_id(&self) -> Uuid {
        self.base.service_id
    }

    pub fn site_id(&self) -> Uuid {
        self.base.site_id
    }

    pub fn binding_type(&self) -> BindingType {
        self.base.binding_type
    }

    pub fn ip_address_id(&self) -> Option<Uuid> {
        match self.base.binding_type {
            BindingType::IPAddress { ip_address_id } => Some(ip_address_id),
            BindingType::Port { ip_address_id, .. } => ip_address_id,
        }
    }

    pub fn port_id(&self) -> Option<Uuid> {
        match self.base.binding_type {
            BindingType::IPAddress { .. } => None,
            BindingType::Port { port_id, .. } => Some(port_id),
        }
    }

    /// A copy of this binding anchored to a different IP address, with its own identity.
    ///
    /// For services reachable at several of their host's IP addresses: a container attached
    /// to more than one bridge subnet reports an endpoint on each, and the bindings resolved
    /// against one endpoint apply equally to the others. An all-IP-addresses port binding
    /// (`ip_address_id: None`) already covers every address, so rebinding narrows it to the
    /// given address rather than leaving it unanchored.
    pub fn rebound_to_ip_address(&self, ip_address_id: Uuid) -> Self {
        let binding_type = match self.base.binding_type {
            BindingType::IPAddress { .. } => BindingType::IPAddress { ip_address_id },
            BindingType::Port { port_id, .. } => BindingType::Port {
                port_id,
                ip_address_id: Some(ip_address_id),
            },
        };

        Self::new(BindingBase {
            binding_type,
            ..self.base
        })
    }

    /// Set the service_id and site_id (for serviceless bindings that get resolved later)
    pub fn with_service(mut self, service_id: Uuid, site_id: Uuid) -> Self {
        self.base.service_id = service_id;
        self.base.site_id = site_id;
        self
    }

    // Legacy convenience constructors (full versions)
    pub fn new_ip_address(service_id: Uuid, site_id: Uuid, ip_address_id: Uuid) -> Self {
        Self::new(BindingBase::new(
            service_id,
            site_id,
            BindingType::IPAddress { ip_address_id },
        ))
    }

    pub fn new_port(
        service_id: Uuid,
        site_id: Uuid,
        port_id: Uuid,
        ip_address_id: Option<Uuid>,
    ) -> Self {
        Self::new(BindingBase::new(
            service_id,
            site_id,
            BindingType::Port {
                port_id,
                ip_address_id,
            },
        ))
    }

    // Serviceless convenience constructors
    pub fn new_ip_address_serviceless(ip_address_id: Uuid) -> Self {
        Self::new_serviceless(BindingType::IPAddress { ip_address_id })
    }

    pub fn new_port_serviceless(port_id: Uuid, ip_address_id: Option<Uuid>) -> Self {
        Self::new_serviceless(BindingType::Port {
            port_id,
            ip_address_id,
        })
    }
}

// Keep BindingDiscriminants for external code that uses it
pub use BindingTypeDiscriminants as BindingDiscriminants;
