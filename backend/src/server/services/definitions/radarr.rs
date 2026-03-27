use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Radarr;

impl ServiceDefinition for Radarr {
    fn name(&self) -> &'static str {
        "Radarr"
    }
    fn description(&self) -> &'static str {
        "A movie collection manager for Usenet and BitTorrent users."
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Media
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AnyOf(vec![
            Pattern::Endpoint(
                PortType::new_tcp(7878),
                "/Content/manifest.json",
                "Radarr",
                None,
            ),
            Pattern::Endpoint(
                PortType::new_tcp(9898),
                "/Content/manifest.json",
                "Radarr",
                None,
            ),
        ])
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/radarr.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Radarr>));
