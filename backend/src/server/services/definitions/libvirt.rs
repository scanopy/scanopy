use crate::server::hosts::r#impl::device_type::DeviceType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::DeviceTypeImplication;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{ClientProbe, Pattern};
use crate::server::shared::logos::Logo;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Libvirt;

impl ServiceDefinition for Libvirt {
    fn name(&self) -> &'static str {
        "libvirt (KVM)"
    }
    fn description(&self) -> &'static str {
        "Linux KVM/QEMU virtualization managed through libvirt"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Hypervisor
    }
    fn device_type(&self) -> DeviceTypeImplication {
        DeviceTypeImplication::Is(DeviceType::Hypervisor)
    }

    /// `virsh` answering over SSH with the libvirt credential's connection, which is also what
    /// lets the libvirt integration run on the host (`execute_integrations` requires this service
    /// to be matched first). A person can still add it by hand to a host with no credential.
    ///
    /// libvirtd's TCP (16509) and TLS (16514) listeners are off by default, and a listener on
    /// either port alone is the bare-port evidence `proxmox.rs` dropped.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::ClientResponse(ClientProbe::Libvirt)
    }

    fn logo(&self) -> Option<Logo> {
        Some(Logo::Libvirt)
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Libvirt>));
