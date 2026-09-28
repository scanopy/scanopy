use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Cockpit;

impl ServiceDefinition for Cockpit {
    fn name(&self) -> &'static str {
        "Cockpit"
    }
    fn description(&self) -> &'static str {
        "Web-based Linux server administration console"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::RemoteAccess
    }

    // cockpit-ws clears its session cookie on the login page (`cockpit=deleted`). 9090 is also
    // Prometheus's default port, so the port alone identifies nothing.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::Header(
            Some(PortType::new_tcp(9090)),
            "set-cookie",
            "cockpit=",
            None,
        )
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/cockpit.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Cockpit>));
