use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::DeviceTypeImplication;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;
use crate::server::shared::logos::Logo;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Grafana;

impl ServiceDefinition for Grafana {
    fn name(&self) -> &'static str {
        "Grafana"
    }
    fn description(&self) -> &'static str {
        "Analytics and monitoring visualization platform"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Monitoring
    }
    fn device_type(&self) -> DeviceTypeImplication {
        DeviceTypeImplication::Server
    }

    /// `app-grafana` is the body class Grafana renders into its page shell on every route. `/`
    /// redirects to `/login` unless anonymous access is on, and the daemon follows no redirects,
    /// so both paths are asked. A link to grafana.com is not evidence: any page can carry one.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AnyOf(vec![
            Pattern::Endpoint(PortType::Http, "/login", "app-grafana", None),
            Pattern::Endpoint(PortType::Http3000, "/login", "app-grafana", None),
            Pattern::Endpoint(PortType::Http, "/", "app-grafana", None),
            Pattern::Endpoint(PortType::Http3000, "/", "app-grafana", None),
        ])
    }

    fn logo(&self) -> Option<Logo> {
        Some(Logo::Grafana)
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Grafana>));
