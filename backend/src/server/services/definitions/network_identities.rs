use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

/// The addresses and MACs a host presents from interfaces of its own beyond its configured NICs.
///
/// Like Unclaimed Open Ports it records data rather than a running service: each identity is a
/// host of its own linked to this service (`HostVirtualization::NetworkIdentity`). Never matched
/// from the network; the Proxmox integration attaches it when a guest's agent reports such an
/// interface. Not generic, so a host holds one.
#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct NetworkIdentities;

impl ServiceDefinition for NetworkIdentities {
    fn name(&self) -> &'static str {
        "Network Identities"
    }
    fn description(&self) -> &'static str {
        "Addresses and MACs this host presents on the network beyond its own NICs"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::NetworkIdentities
    }
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::None
    }
}

inventory::submit!(ServiceDefinitionFactory::new(
    create_service::<NetworkIdentities>
));
