use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct TrueNAS;

impl ServiceDefinition for TrueNAS {
    fn name(&self) -> &'static str {
        "TrueNAS"
    }
    fn description(&self) -> &'static str {
        "Open-source network attached storage system"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Storage
    }

    // Every webui release from 12.0 (CORE) through SCALE carries `<title id="main-page-title">`
    // in index.html. SCALE's index.html has no "TrueNAS" text at all: the title is set by script.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AnyOf(vec![
            Pattern::Endpoint(PortType::Http, "/", "main-page-title", None),
            Pattern::Endpoint(PortType::Https, "/", "main-page-title", None),
        ])
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/truenas.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<TrueNAS>));
