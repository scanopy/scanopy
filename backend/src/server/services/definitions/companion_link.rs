use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::{DeviceTypeImplication, ServiceDefinition};
use crate::server::services::r#impl::patterns::{DnsSdServiceType, Pattern};
use crate::server::shared::logos::Logo;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct CompanionLink;

impl ServiceDefinition for CompanionLink {
    fn name(&self) -> &'static str {
        "Apple Companion Link"
    }
    fn description(&self) -> &'static str {
        "Device-to-device channel Apple devices use for Continuity, Handoff and pairing"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Mobile
    }
    // Macs, iPhones, iPads, Apple TVs and HomePods all run it.
    fn device_type(&self) -> DeviceTypeImplication {
        DeviceTypeImplication::Nothing
    }
    fn logo(&self) -> Option<Logo> {
        Some(Logo::Apple)
    }

    // rapportd listens on a port the OS picks, so no port list can name it. The advertisement's
    // SRV record does, and `DnsSd` binds that port.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::DnsSd(DnsSdServiceType::COMPANION_LINK, None)
    }
}

inventory::submit!(ServiceDefinitionFactory::new(
    create_service::<CompanionLink>
));
