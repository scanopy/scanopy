use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::server::{
    bindings::r#impl::base::{Binding, BindingBase, BindingType},
    services::r#impl::{
        base::{Service, ServiceBase},
        definitions::ServiceDefinition,
        virtualization::ServiceVirtualization,
    },
    shared::types::entities::EntitySource,
};

// =============================================================================
// CREATE BINDING INPUT
// =============================================================================

/// Input for creating a binding with a service.
/// `service_id` and `site_id` are assigned by the server after the service is created.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum CreateBindingInput {
    /// Bind to an interface (service listens on all ports on this ip_address)
    #[schema(title = "IPAddress")]
    IPAddress {
        /// The IP address the service is present at.
        ip_address_id: Uuid,
    },
    /// Bind to a port (optionally on a specific ip_address)
    #[schema(title = "Port")]
    Port {
        /// The port the service listens on.
        port_id: Uuid,
        /// The IP address this port binding applies to. `null` binds to every IP address on the host.
        #[serde(skip_serializing_if = "Option::is_none")]
        ip_address_id: Option<Uuid>,
    },
}

impl CreateBindingInput {
    /// Convert to a full Binding with the given service_id and site_id.
    pub fn into_binding(self, service_id: Uuid, site_id: Uuid) -> Binding {
        let binding_type = match self {
            CreateBindingInput::IPAddress { ip_address_id } => {
                BindingType::IPAddress { ip_address_id }
            }
            CreateBindingInput::Port {
                port_id,
                ip_address_id,
            } => BindingType::Port {
                port_id,
                ip_address_id,
            },
        };

        Binding::new(BindingBase::new(service_id, site_id, binding_type))
    }
}

// =============================================================================
// CREATE SERVICE REQUEST
// =============================================================================

/// Request type for creating a service.
/// Server assigns `id`, `created_at`, `updated_at`, and `source`.
/// Server also assigns `service_id` and `site_id` to all bindings.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateServiceRequest {
    /// The host this entity belongs to.
    pub host_id: Uuid,
    /// The site this entity belongs to.
    pub site_id: Uuid,
    #[schema(value_type = String)]
    // Refer to https://scanopy.net/services for options
    /// Which known software this service is, if identified.
    pub service_definition: Box<dyn ServiceDefinition>,
    /// Human-facing name for the service.
    pub name: String,
    /// Bindings to create with the service.
    /// `service_id` and `site_id` are assigned by the server.
    #[serde(default)]
    pub bindings: Vec<CreateBindingInput>,
    /// Container identity (name, id, compose project), when it is containerized.
    pub virtualization_metadata: Option<ServiceVirtualization>,
    /// The container runtime service hosting this container, if any.
    #[serde(default)]
    pub virtualization_service_id: Option<Uuid>,
    /// Tags assigned to this entity.
    #[serde(default)]
    #[schema(required)]
    pub tags: Vec<Uuid>,
}

impl CreateServiceRequest {
    /// Convert to a Service entity with the given source.
    /// Bindings are created with the service's ID and site_id.
    pub fn into_service(self, source: EntitySource) -> Service {
        let CreateServiceRequest {
            host_id,
            site_id,
            service_definition,
            name,
            bindings: binding_inputs,
            virtualization_metadata,
            virtualization_service_id,
            tags,
        } = self;

        // Create the service first to get an ID
        let service_id = Uuid::new_v4();
        let now = chrono::Utc::now();

        // Convert binding inputs to full bindings with the service's ID
        let bindings: Vec<Binding> = binding_inputs
            .into_iter()
            .map(|input| input.into_binding(service_id, site_id))
            .collect();

        Service {
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: service_id,
            created_at: now,
            updated_at: now,
            base: ServiceBase {
                host_id,
                site_id,
                service_definition,
                name,
                bindings,
                virtualization_metadata,
                virtualization_service_id,
                source,
                tags,
                position: 0, // Position assigned during creation based on existing services
            },
        }
    }

    /// Get site_id for access validation
    pub fn site_id(&self) -> Uuid {
        self.site_id
    }

    /// Get host_id for validation
    pub fn host_id(&self) -> Uuid {
        self.host_id
    }
}
