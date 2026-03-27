use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Snmp;

impl ServiceDefinition for Snmp {
    fn name(&self) -> &'static str {
        "SNMP"
    }
    fn description(&self) -> &'static str {
        "Simple Network Management Protocol"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::SNMP
    }
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AnyOf(vec![
            Pattern::Port(PortType::Snmp),
            Pattern::Port(PortType::SnmpAlt),
        ])
    }
    fn is_generic(&self) -> bool {
        true
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Snmp>));
