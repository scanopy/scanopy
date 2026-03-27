use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct SaltMaster;

impl ServiceDefinition for SaltMaster {
    fn name(&self) -> &'static str {
        "Salt Master"
    }
    fn description(&self) -> &'static str {
        "A Salt master server acts as a central control bus for the clients, which are called minions."
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::NetworkCore
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AllOf(vec![
            Pattern::Port(PortType::new_tcp(4505)),
            Pattern::Port(PortType::new_tcp(4506)),
            Pattern::Not(Box::new(Pattern::Port(PortType::new_tcp(8022)))),
        ])
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/salt-project.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<SaltMaster>));
