use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{InstantOnDeviceType, Pattern};

/// An Instant On switch — 1830, 1930 or 1960, standalone or stacked.
///
/// `STACK` matches here too: the portal presents a stack as one device with one management IP, so
/// it is one host wearing the switch label, not a separate type of thing.
#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct InstantOnSwitch;

impl ServiceDefinition for InstantOnSwitch {
    fn name(&self) -> &'static str {
        "Instant On Switch"
    }
    fn description(&self) -> &'static str {
        "HPE Networking Instant On switch"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::NetworkAccess
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AnyOf(vec![
            Pattern::ManagedDeviceType(InstantOnDeviceType::SWITCH),
            Pattern::ManagedDeviceType(InstantOnDeviceType::STACK),
        ])
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/hpe.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(
    create_service::<InstantOnSwitch>
));
