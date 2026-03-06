use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Grafana;

impl ServiceDefinition for Grafana {
    fn name(&self) -> &'static str {
        "Grafana"
    }
    fn description(&self) -> &'static str {
        "Analytics and monitoring visualization platform"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Monitoring
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::Endpoint(PortType::Http, "/login", "grafana", None)
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/grafana.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Grafana>));
