use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::DeviceTypeImplication;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;
use crate::server::shared::logos::Logo;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Portainer;

impl ServiceDefinition for Portainer {
    fn name(&self) -> &'static str {
        "Portainer"
    }
    fn description(&self) -> &'static str {
        "Orchestration platform for Docker, K8s, and Podman"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Orchestrator
    }
    fn device_type(&self) -> DeviceTypeImplication {
        DeviceTypeImplication::Server
    }

    /// `ng-app="portainer"` is the root element of Portainer's page shell. A link to portainer.io
    /// is not evidence: any page can carry one.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AnyOf(vec![
            Pattern::Endpoint(PortType::Https9443, "/", "ng-app=\"portainer\"", None),
            Pattern::Endpoint(PortType::Http9000, "/", "ng-app=\"portainer\"", None),
        ])
    }

    fn logo(&self) -> Option<Logo> {
        Some(Logo::Portainer)
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Portainer>));
