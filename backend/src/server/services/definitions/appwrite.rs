use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::DeviceTypeImplication;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;
use crate::server::shared::logos::Logo;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Appwrite;

impl ServiceDefinition for Appwrite {
    fn name(&self) -> &'static str {
        "Appwrite"
    }
    fn description(&self) -> &'static str {
        "Open-source backend-as-a-service platform"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Development
    }
    fn device_type(&self) -> DeviceTypeImplication {
        DeviceTypeImplication::Server
    }

    /// The console is served at `/console` and names itself `Appwrite Console` in its page shell.
    /// A link to appwrite.io is not evidence: any page can carry one.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AnyOf(vec![
            Pattern::Endpoint(PortType::Http, "/console", "Appwrite Console", None),
            Pattern::Endpoint(PortType::Https, "/console", "Appwrite Console", None),
        ])
    }

    fn logo(&self) -> Option<Logo> {
        Some(Logo::Appwrite)
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Appwrite>));
