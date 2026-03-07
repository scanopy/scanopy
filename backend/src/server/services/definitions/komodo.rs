use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Komodo;

impl ServiceDefinition for Komodo {
    fn name(&self) -> &'static str {
        "Komodo"
    }

    fn description(&self) -> &'static str {
        "Container deployment and management platform"
    }

    fn category(&self) -> ServiceCategory {
        ServiceCategory::Virtualization
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::Port(PortType::new_tcp(9120))
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/komodo.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Komodo>));
