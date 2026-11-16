use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::base::DiscoverySessionServiceMatchParams;
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::DeviceTypeImplication;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{MatchConfidence, Pattern};

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct WebService;

impl ServiceDefinition for WebService {
    fn name(&self) -> &'static str {
        "Web Service"
    }
    fn description(&self) -> &'static str {
        "Ports that answered HTTP but matched no service. Reassign to the correct service if known."
    }
    /// A placeholder like Unclaimed Open Ports: the ports it holds are waiting for an owner, and
    /// any identified service takes them back.
    fn category(&self) -> ServiceCategory {
        ServiceCategory::OpenPorts
    }
    fn device_type(&self) -> DeviceTypeImplication {
        DeviceTypeImplication::Nothing
    }

    /// The unbound ports that answered an HTTP request, whatever the status. Matching runs this
    /// after every other definition but "Unclaimed Open Ports", so a port reaches it only when no
    /// product claimed what it served, and it never takes a port another service holds.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::Custom(
            |p| !answered_http(p).is_empty(),
            answered_http,
            "An unbound port answered HTTP",
            "No unbound port answered HTTP",
            MatchConfidence::Low,
        )
    }
    fn is_generic(&self) -> bool {
        true
    }
}

fn answered_http(params: &DiscoverySessionServiceMatchParams) -> Vec<PortType> {
    params
        .service_params
        .unbound_ports
        .iter()
        .filter(|port| {
            params
                .baseline_params
                .endpoint_responses
                .iter()
                .any(|response| {
                    response.endpoint.port_type.number() == port.number()
                        && response.endpoint.port_type.protocol() == port.protocol()
                })
        })
        .copied()
        .collect()
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<WebService>));
