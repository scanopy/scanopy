use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Arcane;

impl ServiceDefinition for Arcane {
    fn name(&self) -> &'static str {
        "Arcane"
    }

    fn description(&self) -> &'static str {
        "Container orchestration service"
    }

    fn category(&self) -> ServiceCategory {
        ServiceCategory::Virtualization
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AnyOf(vec![
            Pattern::Port(PortType::new_tcp(3552)),
            Pattern::Port(PortType::new_tcp(3553)),
        ])
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/arcane.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Arcane>));
