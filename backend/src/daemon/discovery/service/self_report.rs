//! Self-report phase: daemon reports itself as a host on the network.
//!
//! Runs on first discovery only. Creates the daemon host with its ip_addresses, NIC rows,
//! Scanopy service, and bindings on bound subnets. Later scans re-report only the NIC rows,
//! through `run_daemon_host_interfaces_phase`. Both decorate the NIC rows with whatever LLDP
//! neighbours a local lldpd can name for them.

use std::net::{IpAddr, Ipv4Addr};

use anyhow::{Error, Result};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::daemon::discovery::service::base::DiscoveryRunner;
use crate::daemon::discovery::service::lldpd;
use crate::daemon::discovery::service::ops::DiscoveryOps;
use crate::daemon::utils::base::DaemonUtils;
use crate::server::bindings::r#impl::base::Binding;
use crate::server::hosts::r#impl::base::{Host, HostBase};
use crate::server::hosts::r#impl::name::{HostName, HostNameSources};
use crate::server::interfaces::r#impl::base::{Interface, InterfaceDataComplete};
use crate::server::ip_addresses::r#impl::base::{ALL_IP_ADDRESSES_IP, IPAddress};
use crate::server::ports::r#impl::base::Port;
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::scanopy_daemon::ScanopyDaemon;
use crate::server::services::r#impl::base::{Service, ServiceBase};
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::MatchDetails;
use crate::server::shared::storage::traits::Storable;
use crate::server::shared::types::entities::EntitySource;
use crate::server::subnets::r#impl::base::Subnet;

impl DiscoveryRunner {
    /// The daemon host's own addresses, and one `Interface` row per NIC that bears them.
    ///
    /// Addresses are narrowed to the subnets this discovery created, because an address outside
    /// them has nowhere to live. The NIC rows deliberately are not: a multi-NIC server's LLDP
    /// chassis id is whichever MAC lldpd elected, and that NIC need not carry an address on any
    /// subnet Scanopy scans — which is exactly the case that leaves a switch's neighbour record
    /// for this host unresolvable. Every NIC has to be present for that MAC to be findable.
    async fn own_addresses_and_interfaces(
        &self,
        network_id: Uuid,
        created_subnets: &[Subnet],
    ) -> Result<(Vec<IPAddress>, Vec<Interface>), Error> {
        let utils = &self.service.utils;
        let interface_filter = self.service.config_store.get_interfaces().await?;
        let (ip_addresses, _, _) = utils
            .get_own_interfaces(network_id, &interface_filter)
            .await?;

        let ip_addresses: Vec<IPAddress> = ip_addresses
            .into_iter()
            .filter_map(|mut i| {
                if let Some(subnet) = created_subnets
                    .iter()
                    .find(|s| s.base.cidr.contains(&i.base.ip_address))
                {
                    i.base.subnet_id = subnet.id;
                    return Some(i);
                }
                None
            })
            .collect();

        let interfaces = utils.own_nics_as_interfaces(network_id, self.host_id, &interface_filter);

        Ok((ip_addresses, interfaces))
    }

    /// Re-report the daemon host's own NICs, every scan after the first.
    ///
    /// Self-report runs once per install and the localhost phase only runs when a localhost
    /// credential exists, so without this the daemon's interface rows would be frozen at whatever
    /// the machine looked like the first time it ever scanned. A NIC added, renamed or re-addressed
    /// later would never appear, and the neighbour records naming it would stay unresolved.
    ///
    /// Ports and services are left to self-report: neither is pruned on upsert, so omitting them
    /// changes nothing, and re-sending them every scan would be noise.
    pub(super) async fn run_daemon_host_interfaces_phase(
        &self,
        ops: &DiscoveryOps,
        created_subnets: &[Subnet],
        cancel: &CancellationToken,
    ) -> Result<(), Error> {
        if cancel.is_cancelled() {
            return Err(anyhow::anyhow!("Discovery cancelled"));
        }

        let network_id = self
            .service
            .config_store
            .get_network_id()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Network ID not set"))?;

        let (ip_addresses, mut interfaces) = self
            .own_addresses_and_interfaces(network_id, created_subnets)
            .await?;

        if interfaces.is_empty() {
            tracing::debug!("No local NICs to report for the daemon host");
            return Ok(());
        }

        let lldp_collected = self.apply_lldpd_neighbours(&mut interfaces).await;

        let mut host = Host::new(self.own_host_base(network_id));
        host.id = self.host_id;

        ops.create_host(
            host,
            ip_addresses,
            vec![],
            vec![],
            interfaces,
            vec![],
            // pnet enumerates NICs, not an ifTable, and skips container bridges — so this is
            // never authority to delete an interface some other collector recorded.
            false,
            // The LLDP group is authoritative only on a scan where lldpd actually answered: a
            // neighbour it no longer lists has genuinely gone and must be cleared. Where it did
            // not answer, nothing here read neighbour data, so every group must be preserved.
            InterfaceDataComplete {
                lldp: lldp_collected,
                ..InterfaceDataComplete::none()
            },
            cancel,
        )
        .await?;

        Ok(())
    }

    /// Lay the local lldpd's neighbour table onto the daemon host's NIC rows, if there is one.
    /// Shared by self-report and the every-scan daemon-host phase.
    ///
    /// Returns whether lldpd answered, which is what decides if the LLDP group is authoritative
    /// for this submission. Absence of a socket is the everyday case and stays quiet; a socket
    /// that exists and does not serve is logged at the level its classification warrants, and
    /// the rows go up undecorated — exactly as they did before this read existed (GH #689).
    async fn apply_lldpd_neighbours(&self, interfaces: &mut [Interface]) -> bool {
        let Some(socket) = lldpd::socket_path() else {
            return false;
        };
        match lldpd::read_neighbors(&socket).await {
            Ok(neighbors) => {
                tracing::debug!(
                    socket = %socket.display(),
                    neighbours = neighbors.len(),
                    "Read the daemon host's LLDP neighbours from lldpd"
                );
                lldpd::apply_neighbors(interfaces, neighbors);
                true
            }
            // lldpd is running but the CLI package is not installed: nothing to warn about,
            // the operator has not asked for this.
            Err(e @ lldpd::LldpdError::CliMissing) => {
                tracing::debug!(error = %e, "Not reading LLDP neighbours");
                false
            }
            Err(e) => {
                tracing::warn!(
                    socket = %socket.display(),
                    outcome = ?e.outcome(),
                    error = %e,
                    "lldpd socket exists but its neighbour table could not be read"
                );
                false
            }
        }
    }

    /// The daemon's own `HostBase`, named the way self-report names it.
    fn own_host_base(&self, network_id: Uuid) -> HostBase {
        let utils = &self.service.utils;
        let hostname = utils.get_own_hostname();

        let mut host_base = HostBase {
            name: HostName::unnamed(),
            hostname: hostname.clone(),
            network_id,
            description: Some("Scanopy daemon".to_string()),
            tags: Vec::new(),
            source: EntitySource::Discovery,
            hidden: false,
            virtualization_metadata: None,
            virtualization_service_id: None,
            sys_descr: None,
            sys_object_id: None,
            sys_location: None,
            sys_contact: None,
            management_url: None,
            chassis_id: None,
            sys_name: None,
            manufacturer: None,
            model: None,
            serial_number: None,
            firmware_revision: None,
            software_revision: None,
            credential_assignments: vec![],
        };

        // The daemon's own host: its hostname if the OS reports one, otherwise its address.
        host_base.apply_name(hostname.map(HostName::from_hostname).unwrap_or_else(|| {
            match utils.get_own_ip_address() {
                Ok(ip) => HostName::from_ip(ip),
                Err(_) => HostName::unnamed(),
            }
        }));

        host_base
    }

    /// Self-report phase: detect ip_addresses, create daemon host with Scanopy service.
    /// Only runs on first discovery (is_first_run check in caller).
    pub(super) async fn run_self_report_phase(
        &self,
        ops: &DiscoveryOps,
        created_subnets: &[Subnet],
        cancel: &CancellationToken,
    ) -> Result<(), Error> {
        if cancel.is_cancelled() {
            return Err(anyhow::anyhow!("Discovery cancelled"));
        }

        let network_id = self
            .service
            .config_store
            .get_network_id()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Network ID not set"))?;

        let host_id = self.host_id;

        let binding_address = self.service.config_store.get_bind_address().await?;
        let binding_ip = IpAddr::V4(binding_address.parse::<Ipv4Addr>()?);

        let (ip_addresses, mut interfaces) = self
            .own_addresses_and_interfaces(network_id, created_subnets)
            .await?;

        if cancel.is_cancelled() {
            return Err(anyhow::anyhow!("Discovery cancelled"));
        }

        // Same decoration as every later scan, so the first scan after install already draws
        // this host's side of its uplinks rather than waiting a cycle for it.
        let lldp_collected = self.apply_lldpd_neighbours(&mut interfaces).await;

        let daemon_bound_subnet_ids: Vec<Uuid> =
            if binding_address == ALL_IP_ADDRESSES_IP.to_string() {
                created_subnets.iter().map(|s| s.id).collect()
            } else {
                created_subnets
                    .iter()
                    .filter(|s| s.base.cidr.contains(&binding_ip))
                    .map(|s| s.id)
                    .collect()
            };

        let own_port = Port::new_hostless(PortType::new_tcp(
            self.service.config_store.get_port().await?,
        ));
        let own_port_id = own_port.id;

        let mut host = Host::new(self.own_host_base(network_id));
        host.id = host_id;

        let daemon_service_definition = ScanopyDaemon;
        let daemon_service_bound_interfaces: Vec<&IPAddress> = ip_addresses
            .iter()
            .filter(|i| daemon_bound_subnet_ids.contains(&i.base.subnet_id))
            .collect();

        let daemon_service = Service::new(ServiceBase {
            name: ServiceDefinition::name(&daemon_service_definition).to_string(),
            service_definition: Box::new(daemon_service_definition),
            tags: Vec::new(),
            network_id,
            bindings: daemon_service_bound_interfaces
                .iter()
                .map(|i| Binding::new_port_serviceless(own_port_id, Some(i.id)))
                .collect(),
            host_id: host.id,
            virtualization_metadata: None,
            virtualization_service_id: None,
            source: EntitySource::DiscoveryWithMatch {
                details: MatchDetails::new_certain("Scanopy Daemon self-report"),
            },
            position: 0,
        });

        if cancel.is_cancelled() {
            return Err(anyhow::anyhow!("Discovery cancelled"));
        }

        ops.create_host(
            host,
            ip_addresses.clone(),
            vec![own_port],
            vec![daemon_service],
            interfaces,
            vec![],
            // pnet enumerates NICs, not an ifTable, and skips container bridges — so this is
            // never authority to delete an interface some other collector recorded.
            false,
            // LLDP is authoritative only when lldpd answered; see the daemon-host phase.
            InterfaceDataComplete {
                lldp: lldp_collected,
                ..InterfaceDataComplete::none()
            },
            cancel,
        )
        .await?;

        Ok(())
    }
}
