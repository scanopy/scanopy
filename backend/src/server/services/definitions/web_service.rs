use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::DeviceTypeImplication;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct WebService;

impl ServiceDefinition for WebService {
    fn name(&self) -> &'static str {
        "Web Service"
    }
    fn description(&self) -> &'static str {
        "A web service that matches no product"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Unknown
    }
    fn device_type(&self) -> DeviceTypeImplication {
        DeviceTypeImplication::Nothing
    }

    /// Added by hand. A port that answered HTTP but matched no product goes to Unclaimed Open
    /// Ports with the rest.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::None
    }
    fn is_generic(&self) -> bool {
        true
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<WebService>));
