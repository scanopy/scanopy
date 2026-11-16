use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::DeviceTypeImplication;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;
use crate::server::shared::logos::Logo;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct WgDashboard;

impl ServiceDefinition for WgDashboard {
    fn name(&self) -> &'static str {
        "WGDashboard"
    }
    fn description(&self) -> &'static str {
        "Wireguard dashboard for visualizing and managing wireguard clients and server"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Dashboard
    }
    fn device_type(&self) -> DeviceTypeImplication {
        DeviceTypeImplication::Server
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::Endpoint(
            PortType::new_tcp(10086),
            "/",
            "<title>WGDashboard</title>",
            None,
        )
    }

    fn logo(&self) -> Option<Logo> {
        Some(Logo::Wireguard)
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<WgDashboard>));
