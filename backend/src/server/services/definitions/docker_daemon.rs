use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{ClientProbe, Pattern};

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Docker;

impl ServiceDefinition for Docker {
    fn name(&self) -> &'static str {
        "Docker"
    }
    fn description(&self) -> &'static str {
        "Docker"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Virtualization
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::ClientResponse(ClientProbe::Docker)
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/docker.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Docker>));
