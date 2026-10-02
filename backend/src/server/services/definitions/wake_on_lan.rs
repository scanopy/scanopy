use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

/// The integration behind the Wake-on-LAN credential: a host the daemon powers on before a scan.
///
/// Never matched on a host: a NIC listening for magic packets has nothing to fingerprint. Its
/// `Pattern::None` is what lets the Wake-on-LAN integration execute without a match. It exists so the credential has a
/// name, category and logo like every other integration.
#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct WakeOnLan;

impl ServiceDefinition for WakeOnLan {
    fn name(&self) -> &'static str {
        "Wake-on-LAN"
    }
    fn description(&self) -> &'static str {
        "Powered on by a magic packet before a scan"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::RemoteAccess
    }
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::None
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<WakeOnLan>));
