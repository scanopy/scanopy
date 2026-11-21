use crate::server::hosts::r#impl::device_type::DeviceType;
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::{DeviceTypeImplication, ServiceDefinition};
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct WpsUpnp;

impl ServiceDefinition for WpsUpnp {
    fn name(&self) -> &'static str {
        "WPS UPnP"
    }
    fn description(&self) -> &'static str {
        "Wi-Fi Protected Setup registrar over UPnP, on access points and routers"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::NetworkAccess
    }
    // Routers with built-in Wi-Fi run it too, and the Gateway definition outranks this there.
    fn device_type(&self) -> DeviceTypeImplication {
        DeviceTypeImplication::Suggests(DeviceType::AccessPoint)
    }

    // hostapd's WPS UPnP server serves its device description at `/wps_device.xml` on 49152, with
    // the Wi-Fi Alliance device type. The URN separates it from the other UPnP servers that take
    // 49152 (a router's IGD, an iPhone's listeners).
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::Endpoint(
            PortType::new_tcp(49152),
            "/wps_device.xml",
            "urn:schemas-wifialliance-org:device:WFADevice:1",
            None,
        )
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<WpsUpnp>));
