use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::{DeviceTypeImplication, ServiceDefinition};
use crate::server::services::r#impl::patterns::{DnsSdServiceType, Pattern};
use crate::server::shared::logos::Logo;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct GoogleCast;

impl ServiceDefinition for GoogleCast {
    fn name(&self) -> &'static str {
        "Google Cast"
    }
    fn description(&self) -> &'static str {
        "Receives Google Cast streams on a TV or speaker with Cast built in"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Media
    }
    // TVs and speakers of many brands run it. Chromecast, Google Home and Bose have their own
    // definitions, which run first and hold 8009, so this one covers the rest.
    fn is_generic(&self) -> bool {
        true
    }
    fn device_type(&self) -> DeviceTypeImplication {
        DeviceTypeImplication::Nothing
    }

    // The Cast channel on 8009 is TLS under a short-lived self-signed certificate that names
    // nothing, so the advertisement is the evidence; `DnsSd` binds the port its SRV record names.
    // The receiver's setup API on 8008 is part of it, and its `eureka_info` names the Cast build.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AllOf(vec![
            Pattern::DnsSd(DnsSdServiceType::GOOGLE_CAST, None),
            Pattern::Optional(Box::new(Pattern::Endpoint(
                PortType::new_tcp(8008),
                "/setup/eureka_info",
                "cast_build_revision",
                None,
            ))),
        ])
    }

    fn logo(&self) -> Option<Logo> {
        Some(Logo::GoogleCast)
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<GoogleCast>));
