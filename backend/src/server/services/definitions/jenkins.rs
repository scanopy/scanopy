use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::DeviceTypeImplication;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;
use crate::server::shared::logos::Logo;

// Jenkins
#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Jenkins;

impl ServiceDefinition for Jenkins {
    fn name(&self) -> &'static str {
        "Jenkins"
    }
    fn description(&self) -> &'static str {
        "Automation server for CI/CD"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Development
    }
    fn device_type(&self) -> DeviceTypeImplication {
        DeviceTypeImplication::Server
    }
    fn discovery_pattern(&self) -> Pattern<'_> {
        // Jenkins sets `X-Jenkins` on every response, the 403 of a secured instance included. A
        // link to jenkins.io is not evidence: any page can carry one.
        Pattern::AnyOf(vec![
            Pattern::Header(Some(PortType::Http8080), "x-jenkins", "", Some(200..500)),
            Pattern::Header(Some(PortType::Https), "x-jenkins", "", Some(200..500)),
        ])
    }
    fn logo(&self) -> Option<Logo> {
        Some(Logo::Jenkins)
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Jenkins>));
