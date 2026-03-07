use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Homebox;

impl ServiceDefinition for Homebox {
    fn name(&self) -> &'static str {
        "Homebox"
    }

    fn description(&self) -> &'static str {
        "Self-hosted home inventory management"
    }

    fn category(&self) -> ServiceCategory {
        ServiceCategory::Office
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::Port(PortType::new_tcp(7745))
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/homebox.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Homebox>));
