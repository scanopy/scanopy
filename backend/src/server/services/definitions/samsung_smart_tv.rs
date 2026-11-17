use crate::server::hosts::r#impl::device_type::DeviceType;
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::{DeviceTypeImplication, ServiceDefinition};
use crate::server::services::r#impl::patterns::Pattern;
use crate::server::shared::logos::Logo;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct SamsungSmartTv;

impl ServiceDefinition for SamsungSmartTv {
    fn name(&self) -> &'static str {
        "Samsung Smart TV"
    }
    fn description(&self) -> &'static str {
        "Remote control and app launch API of a Samsung Tizen TV"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::IoT
    }
    fn device_type(&self) -> DeviceTypeImplication {
        DeviceTypeImplication::Is(DeviceType::SmartTv)
    }
    fn logo(&self) -> Option<Logo> {
        Some(Logo::Samsung)
    }

    // A Tizen TV answers `/api/v2/` without authentication with a JSON description of itself,
    // `"OS":"Tizen"` among it: on 8001, and on 8002 over TLS. The other ports it opens are the
    // TV's too: the DIAL server on 8080 (Netflix comes preinstalled, so its DIAL application
    // document is always there), and the AllShare UPnP servers on 7678 and 9110, which answer a
    // plain GET with a 400 SOAP fault and `Server: … Samsung AllShare Server`.
    fn discovery_pattern(&self) -> Pattern<'_> {
        let tizen_api = [PortType::new_tcp(8001), PortType::new_tcp(8002)]
            .into_iter()
            .map(|port| Pattern::Endpoint(port, "/api/v2/", "\"OS\":\"Tizen\"", None));
        let allshare = [PortType::new_tcp(7678), PortType::new_tcp(9110)]
            .into_iter()
            .map(|port| {
                Pattern::Optional(Box::new(Pattern::Header(
                    Some(port),
                    "server",
                    "Samsung AllShare Server",
                    Some(200..500),
                )))
            });
        Pattern::AllOf(
            [
                Pattern::AnyOf(tizen_api.collect()),
                Pattern::Optional(Box::new(Pattern::Endpoint(
                    PortType::Http8080,
                    "/ws/apps/Netflix",
                    "urn:dial-multiscreen-org:schemas:dial",
                    None,
                ))),
            ]
            .into_iter()
            .chain(allshare)
            .collect(),
        )
    }
}

inventory::submit!(ServiceDefinitionFactory::new(
    create_service::<SamsungSmartTv>
));
