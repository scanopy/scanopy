use crate::server::hosts::r#impl::device_type::DeviceType;
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::DeviceTypeImplication;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;
use crate::server::shared::logos::Logo;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct HpPrinter;

impl ServiceDefinition for HpPrinter {
    fn name(&self) -> &'static str {
        "HP Printer"
    }
    fn description(&self) -> &'static str {
        "An HP Printer"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Printer
    }
    fn device_type(&self) -> DeviceTypeImplication {
        DeviceTypeImplication::Is(DeviceType::Printer)
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        // The embedded web server titles its page with the model on 80, 8080 and 443 alike. The
        // pages are requested at `/`, the path the other web definitions request, so the printer
        // answers one request per port rather than two. Requested at `""`, an M110w's 8080 page
        // fell through to the generic HTTP match in a full scan and not this definition.
        //
        // The certificate on 443 is issued by the printer itself under `O=HP Inc, OU=IPG`, the
        // imaging and printing group, so it names an HP printer where the page does not load.
        Pattern::AllOf(vec![
            Pattern::AnyOf(vec![
                Pattern::Endpoint(PortType::Http, "/", "LaserJet", None),
                Pattern::Endpoint(PortType::Http, "/", "DeskJet", None),
                Pattern::Endpoint(PortType::Http, "/", "OfficeJet", None),
                Pattern::Endpoint(PortType::Http8080, "/", "LaserJet", None),
                Pattern::Endpoint(PortType::Http8080, "/", "DeskJet", None),
                Pattern::Endpoint(PortType::Http8080, "/", "OfficeJet", None),
                Pattern::Endpoint(PortType::Https, "/", "LaserJet", None),
                Pattern::Endpoint(PortType::Https, "/", "DeskJet", None),
                Pattern::Endpoint(PortType::Https, "/", "OfficeJet", None),
                Pattern::CertificateIssuer("OU=IPG", &[443]),
            ]),
            // 515/udp is gone: no probe answers on it, so the network scan never reports it and a
            // container that declares it is not evidence of a printer. The remaining two qualify a
            // page that already named the model, so neither stands alone.
            Pattern::AnyOf(vec![
                Pattern::Port(PortType::Ipp),
                Pattern::Port(PortType::LdpTcp),
            ]),
            // HP's printer request and status ports, registered to HP with IANA as `prnrequest`
            // and `prnstatus`.
            Pattern::Optional(Box::new(Pattern::AnyOf(vec![
                Pattern::Port(PortType::new_tcp(3910)),
                Pattern::Port(PortType::new_tcp(3911)),
            ]))),
        ])
    }

    fn logo(&self) -> Option<Logo> {
        Some(Logo::Hp)
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<HpPrinter>));
