use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::{DeviceTypeImplication, ServiceDefinition};
use crate::server::services::r#impl::patterns::Pattern;
use crate::server::shared::logos::Logo;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct NetflixNrdp;

impl ServiceDefinition for NetflixNrdp {
    fn name(&self) -> &'static str {
        "Netflix (NRDP)"
    }
    fn description(&self) -> &'static str {
        "Netflix Ready Device Platform, the Netflix app's second-screen control endpoint"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Media
    }
    // TVs, set-top boxes and consoles of every brand ship it.
    fn is_generic(&self) -> bool {
        true
    }
    // So it names no device type: a TV's or console's own definition says what the host is.
    fn device_type(&self) -> DeviceTypeImplication {
        DeviceTypeImplication::Nothing
    }
    fn logo(&self) -> Option<Logo> {
        Some(Logo::Netflix)
    }

    // The Netflix app answers on 9080 with `Server: NRDP/<version>`.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::Header(
            Some(PortType::new_tcp(9080)),
            "server",
            "NRDP/",
            Some(200..500),
        )
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<NetflixNrdp>));
