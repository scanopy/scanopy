use crate::server::bindings::r#impl::base::Binding;
use crate::server::discovery::r#impl::types::DiscoveryType;
use crate::server::interfaces::r#impl::base::Interface;
use crate::server::ports::r#impl::base::{Port, PortType};
use crate::server::services::definitions::ServiceDefinitionRegistry;
use crate::server::services::r#impl::definitions::ServiceDefinitionExt;
use crate::server::services::r#impl::definitions::{DefaultServiceDefinition, ServiceDefinition};
use crate::server::services::r#impl::endpoints::{Endpoint, EndpointResponse};
use crate::server::services::r#impl::patterns::{MatchConfidence, MatchReason};
use crate::server::services::r#impl::virtualization::{
    DockerVirtualization, ServiceVirtualization,
};
use crate::server::shared::entities::ChangeTriggersTopologyStaleness;
use crate::server::shared::position::Positioned;
use crate::server::shared::storage::traits::Storable;
use crate::server::shared::types::entities::{DiscoveryMetadata, EntitySource};
use crate::server::subnets::r#impl::base::Subnet;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::hash::Hash;
use std::net::IpAddr;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Validate, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct ServiceBase {
    pub host_id: Uuid,
    pub network_id: Uuid,
    #[schema(value_type = String)]
    pub service_definition: Box<dyn ServiceDefinition>,
    #[validate(length(min = 0, max = 100))]
    pub name: String,
    pub bindings: Vec<Binding>,
    pub virtualization: Option<ServiceVirtualization>,
    #[schema(read_only)]
    /// Will be automatically set to Manual for creation through API
    pub source: EntitySource,
    #[serde(default)]
    #[schema(required)]
    pub tags: Vec<Uuid>,
    /// Position of this service in the host's service list (for ordering)
    #[serde(default)]
    #[schema(required)]
    pub position: i32,
}

impl Default for ServiceBase {
    fn default() -> Self {
        Self {
            host_id: Uuid::nil(),
            network_id: Uuid::nil(),
            service_definition: Box::new(DefaultServiceDefinition),
            name: String::new(),
            bindings: Vec::new(),
            virtualization: None,
            source: EntitySource::Unknown,
            tags: Vec::new(),
            position: 0,
        }
    }
}

impl ChangeTriggersTopologyStaleness<Service> for Service {
    fn triggers_staleness(&self, other: Option<Service>) -> bool {
        if let Some(other_service) = other {
            self.base.bindings != other_service.base.bindings
                || self.base.host_id != other_service.base.host_id
                || self.base.virtualization != other_service.base.virtualization
        } else {
            true
        }
    }
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize, Eq, Default, ToSchema)]
#[schema(example = crate::server::shared::types::examples::service)]
pub struct Service {
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
    pub base: ServiceBase,
}

#[derive(Debug, Clone)]
pub struct DiscoverySessionServiceMatchParams<'a> {
    pub host_id: &'a Uuid,
    pub gateway_ips: &'a [IpAddr],
    pub daemon_id: &'a Uuid,
    pub network_id: &'a Uuid,
    pub discovery_type: &'a DiscoveryType,
    pub baseline_params: &'a ServiceMatchBaselineParams<'a>,
    pub service_params: ServiceMatchServiceParams<'a>,
}

#[derive(Debug, Clone)]
pub struct ServiceMatchBaselineParams<'a> {
    pub subnet: &'a Subnet,
    pub interface: &'a Interface,
    pub all_ports: &'a Vec<PortType>,
    pub endpoint_responses: &'a Vec<EndpointResponse>,
    pub virtualization: &'a Option<ServiceVirtualization>,
    pub client_responses: &'a std::collections::HashMap<
        crate::server::services::r#impl::patterns::ClientProbe,
        Vec<PortType>,
    >,
}

#[derive(Debug, Clone)]
pub struct ServiceMatchServiceParams<'a> {
    pub service_definition: Box<dyn ServiceDefinition>,
    pub matched_services: &'a Vec<Service>,
    pub unbound_ports: &'a Vec<PortType>,
}

impl PartialEq for Service {
    fn eq(&self, other: &Self) -> bool {
        // Quick path: if IDs match, they're the same service
        if self.id == other.id {
            return true;
        }

        // Must be on same host and network
        if self.base.host_id != other.base.host_id || self.base.network_id != other.base.network_id
        {
            return false;
        }

        // Must be same service definition
        if self.base.service_definition.id() != other.base.service_definition.id() {
            return false;
        }

        // For non-generic services: same host + definition = same service
        // Handles: Plex discovered on multiple interfaces (different port UUIDs)
        if !ServiceDefinitionExt::is_generic(&self.base.service_definition) {
            return true;
        }

        // Gateway and OpenPorts services are singletons per host - only one per host
        // Gateway typically only has interface bindings (no ports)
        // OpenPorts collects unclaimed ports and should merge rather than create duplicates
        // Handles: Gateway discovered on eth0, eth1, wlan0 -> should be same service
        // Handles: OpenPorts from multiple discovery runs -> should merge bindings
        if ServiceDefinitionExt::is_gateway(&self.base.service_definition)
            || ServiceDefinitionExt::is_open_ports(&self.base.service_definition)
        {
            return true;
        }

        // For non-gateway generic services, use port bindings and container info

        // === GENERIC SERVICE EQUALITY ===
        // All possible permutations of generic services on the same host:

        // Extract virtualization info
        let self_docker = self.base.virtualization.as_ref().map(|v| {
            let ServiceVirtualization::Docker(dv) = v;
            dv
        });

        let other_docker = other.base.virtualization.as_ref().map(|v| {
            let ServiceVirtualization::Docker(dv) = v;
            dv
        });

        // Extract port IDs from bindings
        let self_port_ids: std::collections::HashSet<_> = self
            .base
            .bindings
            .iter()
            .filter_map(|b| b.port_id())
            .collect();

        let other_port_ids: std::collections::HashSet<_> = other
            .base
            .bindings
            .iter()
            .filter_map(|b| b.port_id())
            .collect();

        let has_shared_ports = !self_port_ids.is_empty()
            && !other_port_ids.is_empty()
            && !self_port_ids.is_disjoint(&other_port_ids);

        match (self_docker, other_docker) {
            // ========================================
            // CASE 1: Both containerized
            // ========================================
            (Some(self_dv), Some(other_dv)) => {
                // CASE 1A: Both have container IDs
                // Match Method: Container ID equality
                // Example: PostgreSQL container discovered via docker scan vs network scan
                if let (Some(self_cid), Some(other_cid)) =
                    (&self_dv.container_id, &other_dv.container_id)
                {
                    return self_cid == other_cid;
                }

                // CASE 1B: Only one has container ID
                // Match Method: Different services
                // Example: Shouldn't happen in practice, but treat as different
                if self_dv.container_id.is_some() || other_dv.container_id.is_some() {
                    return false;
                }

                // CASE 1C: Neither has container ID, but both have container names
                // Match Method: Container name equality
                // Example: Edge case where container_id wasn't captured
                if let (Some(self_cname), Some(other_cname)) =
                    (&self_dv.container_name, &other_dv.container_name)
                {
                    return self_cname == other_cname;
                }

                // CASE 1D: Neither has container ID or name, check ports
                // Match Method: Port binding overlap
                // Example: Malformed container data, fall back to port matching
                has_shared_ports
            }

            // ========================================
            // CASE 2: One containerized, one not
            // ========================================
            (Some(_), None) | (None, Some(_)) => {
                // CASE 2A: Shared port bindings
                // Match Method: Port binding overlap
                // Example: Container discovered via docker (with container_id),
                //          then rediscovered via network scan (no container_id)
                if has_shared_ports {
                    return true;
                }

                // CASE 2B: No shared ports
                // Match Method: Different services
                // Example: Two different services, one containerized, one not
                false
            }

            // ========================================
            // CASE 3: Neither containerized
            // ========================================
            (None, None) => {
                // CASE 3A: Shared port bindings
                // Match Method: Port binding overlap
                // Example: This case doesn't happen - non-containerized generic services
                //          discovered from different subnets get different port UUIDs
                if has_shared_ports {
                    return true;
                }

                // CASE 3B: Different port bindings (or no ports)
                // Match Method: Different services
                // Example: Two separate PostgreSQL instances on bare metal
                //          OR: Generic service discovered from different subnets
                //          (creates duplicates - requires manual consolidation)
                false
            }
        }
    }
}

impl Hash for Service {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.base.service_definition.hash(state);
        self.base.name.hash(state);
        self.base.host_id.hash(state);
    }
}

impl Display for Service {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {:?}", self.base.name, self.id)
    }
}

impl Service {
    pub fn get_binding(&self, id: Uuid) -> Option<&Binding> {
        self.base.bindings.iter().find(|b| b.id() == id)
    }

    pub fn to_bound_interface_ids(&self) -> Vec<Option<Uuid>> {
        self.base
            .bindings
            .iter()
            .map(|i| i.interface_id())
            .collect()
    }

    pub fn to_bound_port_ids(&self) -> Vec<Uuid> {
        self.base
            .bindings
            .iter()
            .filter_map(|i| i.port_id())
            .collect()
    }

    pub fn all_discovery_ports() -> Vec<PortType> {
        let mut ports: Vec<PortType> = ServiceDefinitionRegistry::all_service_definitions()
            .iter()
            .flat_map(|s| s.discovery_pattern().ports())
            .collect();

        ports.sort_by_key(|p| (p.number(), p.protocol()));
        ports.dedup();
        ports
    }

    pub fn all_discovery_endpoints() -> Vec<Endpoint> {
        let mut endpoints: Vec<Endpoint> = ServiceDefinitionRegistry::all_service_definitions()
            .iter()
            .flat_map(|s| s.discovery_pattern().endpoints())
            .collect();

        endpoints.sort_by_key(|e| (e.protocol.to_string(), e.port_type.number(), e.path.clone()));
        endpoints.dedup();
        endpoints
    }

    /// Get ports that appear ONLY in endpoint patterns, not in port scan patterns
    pub fn endpoint_only_ports() -> Vec<PortType> {
        let port_scan_ports = Self::all_discovery_ports();
        let endpoint_ports: Vec<PortType> = Self::all_discovery_endpoints()
            .iter()
            .map(|e| e.port_type)
            .collect();

        // Get ports that are in endpoints but NOT in port scans
        let mut endpoint_only: Vec<PortType> = endpoint_ports
            .into_iter()
            .filter(|ep| !port_scan_ports.contains(ep))
            .collect();

        endpoint_only.sort_by_key(|p| (p.number(), p.protocol()));
        endpoint_only.dedup();

        endpoint_only
    }

    /// Matches scanned data and returns service, vec of matched ports
    pub fn from_discovery(
        params: DiscoverySessionServiceMatchParams,
    ) -> Option<(Self, Vec<Port>, Option<Endpoint>)> {
        let DiscoverySessionServiceMatchParams {
            host_id,
            network_id,
            baseline_params,
            service_params,
            daemon_id,
            discovery_type,
            ..
        } = params.clone();

        let ServiceMatchBaselineParams {
            interface,
            virtualization,
            ..
        } = baseline_params;

        let virtualization = *virtualization;

        let ServiceMatchServiceParams {
            service_definition, ..
        } = service_params;

        if let Ok(mut result) = service_definition.discovery_pattern().matches(&params) {
            tracing::info!(
                service = %service_definition.name(),
                host_ip = %interface.base.ip_address,
                network_id = %network_id,
                daemon_id = %daemon_id,
                discovery_type = ?discovery_type,
                matched_ports = ?result.ports.iter().map(|p| p.number()).collect::<Vec<_>>(),
                match_confidence = ?result.details.confidence,
                "Service discovered"
            );

            tracing::trace!(
                service_id = %service_definition.id(),
                match_reason = %result.details.reason,
                full_params = ?params,
                "Service match details"
            );

            let mut name = service_definition.name().to_string();

            if ServiceDefinitionExt::is_generic(&service_definition) {
                if let Some(ServiceVirtualization::Docker(DockerVirtualization {
                    container_name: Some(c_name),
                    ..
                })) = virtualization
                {
                    name = c_name.clone()
                }

                // Confidence not applicable for generic services
                result.details.confidence = MatchConfidence::NotApplicable;
                result.details.reason = MatchReason::Container(
                    "Generic service".to_string(),
                    vec![result.details.reason],
                )
            };

            let discovery_metadata = DiscoveryMetadata::new(discovery_type.clone(), *daemon_id);

            let ports: Vec<Port> = result
                .ports
                .iter()
                .map(|p| Port::new_hostless(*p))
                .collect();

            let bindings: Vec<Binding> = if !result.ports.is_empty() {
                ports
                    .iter()
                    .map(|p| Binding::new_port_serviceless(p.id, Some(interface.id)))
                    .collect()
            } else {
                vec![Binding::new_interface_serviceless(interface.id)]
            };

            let service = Service::new(ServiceBase {
                host_id: *host_id,
                network_id: *network_id,
                service_definition,
                name,
                virtualization: virtualization.clone(),
                tags: Vec::new(),
                bindings,
                source: EntitySource::DiscoveryWithMatch {
                    metadata: vec![discovery_metadata],
                    details: result.details.clone(),
                },
                position: 0, // Discovery services get position assigned during merge
            });

            Some((service, ports, result.endpoint))
        } else {
            tracing::trace!(
                service = %service_definition.name(),
                host_ip = %interface.base.ip_address,
                "Service pattern did not match"
            );
            None
        }
    }
}

impl Positioned for Service {
    fn position(&self) -> i32 {
        self.base.position
    }

    fn set_position(&mut self, position: i32) {
        self.base.position = position;
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn entity_name() -> &'static str {
        "service"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::bindings::r#impl::base::Binding;
    use crate::server::services::definitions::ServiceDefinitionRegistry;
    use crate::server::services::r#impl::virtualization::{
        DockerVirtualization, ServiceVirtualization,
    };
    use crate::server::shared::types::entities::EntitySource;
    use uuid::Uuid;

    fn make_service(
        host_id: Uuid,
        network_id: Uuid,
        definition_id: &str,
        virtualization: Option<ServiceVirtualization>,
        port_ids: Vec<Uuid>,
        interface_id: Option<Uuid>,
    ) -> Service {
        let service_def = ServiceDefinitionRegistry::find_by_id(definition_id)
            .unwrap_or_else(|| ServiceDefinitionRegistry::all_service_definitions()[0].clone());

        let bindings = port_ids
            .into_iter()
            .map(|pid| Binding::new_port_serviceless(pid, interface_id))
            .collect();

        Service::new(ServiceBase {
            name: service_def.name().to_string(),
            host_id,
            bindings,
            network_id,
            service_definition: service_def,
            virtualization,
            source: EntitySource::System,
            tags: Vec::new(),
            position: 0,
        })
    }

    fn docker_virt(container_name: &str) -> Option<ServiceVirtualization> {
        Some(ServiceVirtualization::Docker(DockerVirtualization {
            container_name: Some(container_name.to_string()),
            container_id: Some(Uuid::new_v4().to_string()),
            service_id: Uuid::new_v4(),
        }))
    }

    #[test]
    fn non_generic_same_definition_matches_regardless_of_virtualization() {
        // Network scan creates Scanopy Server without virtualization
        // Docker scan creates Scanopy Server WITH virtualization
        // They should be equal (same host + same non-generic definition)
        let host_id = Uuid::new_v4();
        let network_id = Uuid::new_v4();
        let port_id = Uuid::new_v4();
        let iface_id = Uuid::new_v4();

        let network_svc = make_service(
            host_id,
            network_id,
            "Scanopy Server",
            None,
            vec![port_id],
            Some(iface_id),
        );
        let docker_svc = make_service(
            host_id,
            network_id,
            "Scanopy Server",
            docker_virt("scanopy-server-1"),
            vec![port_id],
            Some(iface_id),
        );

        assert_eq!(
            network_svc, docker_svc,
            "Non-generic services with same definition on same host should match for upsert"
        );
    }

    #[test]
    fn non_generic_same_definition_matches_with_different_ports() {
        // Network scan finds Scanopy Server on host port 60072
        // Docker scan finds same service but bound to Docker bridge port (different UUID)
        // Should still match — non-generic services match on definition alone
        let host_id = Uuid::new_v4();
        let network_id = Uuid::new_v4();

        let network_svc = make_service(
            host_id,
            network_id,
            "Scanopy Server",
            None,
            vec![Uuid::new_v4()],
            Some(Uuid::new_v4()),
        );
        let docker_svc = make_service(
            host_id,
            network_id,
            "Scanopy Server",
            docker_virt("scanopy-server-1"),
            vec![Uuid::new_v4()],
            Some(Uuid::new_v4()),
        );

        assert_eq!(
            network_svc, docker_svc,
            "Non-generic services should match even with different port/interface UUIDs"
        );
    }

    #[test]
    fn different_definitions_do_not_match() {
        let host_id = Uuid::new_v4();
        let network_id = Uuid::new_v4();
        let port_id = Uuid::new_v4();

        let svc_a = make_service(
            host_id,
            network_id,
            "Scanopy Server",
            None,
            vec![port_id],
            None,
        );
        let svc_b = make_service(host_id, network_id, "Portainer", None, vec![port_id], None);

        assert_ne!(
            svc_a, svc_b,
            "Different service definitions should not match even with shared ports"
        );
    }

    #[test]
    fn generic_docker_containers_match_by_container_id() {
        // Two Docker Container services with the same container_id should match
        let host_id = Uuid::new_v4();
        let network_id = Uuid::new_v4();
        let container_id = Uuid::new_v4().to_string();

        let svc_a = make_service(
            host_id,
            network_id,
            "Docker Container",
            Some(ServiceVirtualization::Docker(DockerVirtualization {
                container_name: Some("my-container".to_string()),
                container_id: Some(container_id.clone()),
                service_id: Uuid::new_v4(),
            })),
            vec![],
            None,
        );
        let svc_b = make_service(
            host_id,
            network_id,
            "Docker Container",
            Some(ServiceVirtualization::Docker(DockerVirtualization {
                container_name: Some("my-container".to_string()),
                container_id: Some(container_id),
                service_id: Uuid::new_v4(),
            })),
            vec![],
            None,
        );

        assert_eq!(
            svc_a, svc_b,
            "Generic Docker Container services with same container_id should match"
        );
    }

    #[test]
    fn generic_containerized_vs_non_containerized_matches_on_shared_ports() {
        // Docker Container (with virtualization) vs bare service (no virtualization)
        // Should match if they share port bindings (Case 2A)
        let host_id = Uuid::new_v4();
        let network_id = Uuid::new_v4();
        let shared_port = Uuid::new_v4();

        let bare_svc = make_service(
            host_id,
            network_id,
            "Docker Container",
            None,
            vec![shared_port],
            None,
        );
        let docker_svc = make_service(
            host_id,
            network_id,
            "Docker Container",
            docker_virt("my-container"),
            vec![shared_port],
            None,
        );

        assert_eq!(
            bare_svc, docker_svc,
            "Generic services with one containerized should match on shared ports"
        );
    }

    #[test]
    fn generic_containerized_vs_non_containerized_no_shared_ports_no_match() {
        // Docker Container (with virtualization) vs bare service (no virtualization)
        // Different ports → should NOT match (Case 2B)
        let host_id = Uuid::new_v4();
        let network_id = Uuid::new_v4();

        let bare_svc = make_service(
            host_id,
            network_id,
            "Docker Container",
            None,
            vec![Uuid::new_v4()],
            None,
        );
        let docker_svc = make_service(
            host_id,
            network_id,
            "Docker Container",
            docker_virt("my-container"),
            vec![Uuid::new_v4()],
            None,
        );

        assert_ne!(
            bare_svc, docker_svc,
            "Generic services without shared ports should not match"
        );
    }
}
