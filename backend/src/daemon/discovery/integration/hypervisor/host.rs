//! Recording a hypervisor's own host, with the service its guests link to.

use std::collections::HashMap;

use anyhow::Error;
use uuid::Uuid;

use crate::daemon::discovery::integration::IntegrationContext;
use crate::server::hosts::r#impl::base::Host;
use crate::server::hosts::r#impl::virtualization::HostVirtualizationDiscriminants;
use crate::server::interfaces::r#impl::base::InterfaceDataComplete;
use crate::server::ip_addresses::r#impl::base::IPAddress;
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::base::ServiceMatchBaselineParams;
use crate::server::services::r#impl::definitions::{ServiceDefinitionExt, VirtualizationRole};
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::shared::attribution::AttributeSource;
use crate::server::subnets::r#impl::base::Subnet;
use crate::server::subnets::r#impl::inference::placeable_subnet;

/// A hypervisor host as its integration reads it, before its service is matched.
pub struct HypervisorHost {
    pub host: Host,
    /// Its addresses, the one it is reached at first.
    pub ip_addresses: Vec<IPAddress>,
    /// The probe that answered, which is what its hypervisor service definition's
    /// `ClientResponse` arm matches.
    pub client_probe: ClientProbe,
    /// The source of what the probe found.
    pub source: AttributeSource,
    /// The guests it runs: its service's role names them.
    pub guests: HostVirtualizationDiscriminants,
}

/// Record a hypervisor's host with its hypervisor service, returning the host's and that
/// service's stored ids, so its guests can link to the service.
///
/// `None` when the host's address sits in no subnet this site holds (the matcher needs one to
/// evaluate against), or the matcher found no hypervisor service for its guests. Stricter than
/// the submission rule ([`IPAddress::discovered`]) for that reason; its other addresses follow the
/// rule.
pub async fn create_hypervisor_host(
    ctx: &IntegrationContext<'_>,
    subnets: &[Subnet],
    hypervisor: HypervisorHost,
) -> Result<Option<(Uuid, Uuid)>, Error> {
    let HypervisorHost {
        host,
        ip_addresses,
        client_probe,
        source,
        guests,
    } = hypervisor;
    let Some(ip_address) = ip_addresses.first() else {
        return Ok(None);
    };
    let Some(subnet) = placeable_subnet(subnets, ip_address.base.ip_address) else {
        tracing::warn!(
            ip = %ip_address.base.ip_address,
            "Hypervisor address is in no known subnet; its guests are recorded without it"
        );
        return Ok(None);
    };

    // The real matcher, fed the probe's answer on the ports the hypervisor's own service owns:
    // Proxmox VE's API port. None for libvirt, which signs in through an SSH login and answers
    // on a local socket, so its service is never in the way of the SSH service on that port.
    let all_ports = ctx.credential.reach().own_ports();
    let client_responses = HashMap::from([(client_probe, all_ports.clone())]);
    let daemon_id = ctx.ops.daemon_id().await?;
    let (services, ports) = ctx.ops.match_services(
        &host,
        &ServiceMatchBaselineParams {
            subnet,
            ip_address,
            all_ports: &all_ports,
            endpoint_responses: &vec![],
            virtualization_metadata: &None,
            virtualization_service_id: None,
            client_responses: &client_responses,
            managed_device: &None,
            dns_sd: &None,
            certificates: &[],
            fingerprints: &[],
        },
        &[],
        &daemon_id,
        &host.base.site_id,
        source,
    )?;

    let response = ctx
        .ops
        .create_integration_host(
            ctx.integration,
            host,
            ip_addresses,
            ports,
            services,
            vec![],
            vec![],
            // A hypervisor's inventory never carries its host's interface table.
            false,
            InterfaceDataComplete::none(),
            ctx.cancel,
        )
        .await?;

    let role = Some(VirtualizationRole::Hypervisor { hosts: guests });
    Ok(response
        .services
        .iter()
        .find(|s| s.base.service_definition.virtualization_role() == role)
        .map(|s| (response.id, s.id)))
}
