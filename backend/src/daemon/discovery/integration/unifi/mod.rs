//! UniFi Network Application (controller) discovery integration.
//!
//! Reads `GET /stat/device` from a UniFi controller and turns the controller's inventory into
//! Scanopy hosts, interfaces, LLDP neighbors and bridge-FDB entries.
//!
//! # Why this exists
//!
//! UniFi switches (e.g. the USW-Pro-Max) do not expose LLDP-MIB over SNMP — a walk of
//! `1.0.8802.1.1.2.1.4.1` returns `No Such Object` — so SNMP-based L2 topology comes up empty
//! for them. The firmware collects LLDP internally and the controller API exposes it, so this
//! integration recovers the same neighbor data from a different source.
//!
//! # Shape
//!
//! Unlike SNMP or Docker, one credential describes an endpoint that reports on *many* devices.
//! So `execute` enriches the scanned host only when the controller is itself a UniFi device
//! (a Dream Machine or Cloud Key), and creates the rest via `ops.create_host`. Everything it
//! produces flows into the same ingestion and neighbor-resolution path SNMP feeds; there is no
//! UniFi-specific topology code anywhere on the server.
//!
//! Layering: [`types`] holds the raw wire structs, [`client`] the HTTP transport and auth, and
//! [`mapping`] the wire → entity translation. A field-shape correction is a one-file change.

pub mod client;
pub mod mapping;
pub mod types;

use crate::server::subnets::r#impl::inference::placeable_subnet;
use std::time::Duration;

use anyhow::{Error, Result};
use async_trait::async_trait;

use crate::server::credentials::r#impl::mapping::{
    CredentialQueryPayload, CredentialQueryPayloadDiscriminants,
};
use crate::server::interfaces::r#impl::base::InterfaceDataComplete;
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::base::ServiceMatchBaselineParams;
use crate::server::services::r#impl::patterns::{ClientProbe, ManagedDevice};

use super::controller;
use super::{
    Checkpoint, Completeness, DiscoveryIntegration, IntegrationContext, IntegrationFailure,
    InterfaceSource, InterfaceViewScope, ProbeContext, ProbeFailure, ProbeSuccess,
};
use crate::daemon::discovery::service::ops::HostData;
use crate::daemon::discovery::service::warnings::AttemptOutcome;
use client::UnifiClient;
use types::{UnifiDevice, UnifiStation};

/// Connected client carried from `probe` to `execute`.
struct UnifiProbeHandle {
    client: UnifiClient,
}

pub struct UnifiIntegration;

#[async_trait]
impl DiscoveryIntegration for UnifiIntegration {
    /// A `port_table` is every physical port and nothing else: the same switch under SNMP also
    /// reports the VLAN, loopback and CPU interfaces UniFi never mentions.
    fn interface_view_scope(&self) -> InterfaceViewScope {
        InterfaceViewScope::PhysicalPortsOnly
    }

    fn credential_type(&self) -> CredentialQueryPayloadDiscriminants {
        CredentialQueryPayloadDiscriminants::UnifiController
    }

    fn estimated_seconds(&self) -> u32 {
        10
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(180)
    }

    fn probe_gate_ports(&self, credential: &CredentialQueryPayload) -> Vec<PortType> {
        match credential {
            CredentialQueryPayload::UnifiController(c) => vec![PortType::new_tcp(c.port)],
            _ => vec![],
        }
    }

    async fn probe(&self, ctx: &ProbeContext<'_>) -> Result<ProbeSuccess, ProbeFailure> {
        let CredentialQueryPayload::UnifiController(credential) = ctx.credential else {
            return Err(ProbeFailure::malformed("Not a UniFi credential"));
        };

        let client =
            UnifiClient::connect(&ctx.ip.to_string(), credential, ctx.accept_invalid_certs)
                .await
                .map_err(|e| {
                    // `e` is already phrased for an operator and never contains the secret. The
                    // client distinguishes a refused login from an unknown site from a transport
                    // failure, so keep that distinction rather than flattening it — a wrong
                    // password and a self-signed certificate send an operator to opposite ends
                    // of the problem, and both used to read as "was rejected".
                    ProbeFailure::with_outcome(
                        UnifiClient::classify_connect_error(&e),
                        e.to_string(),
                    )
                })?;

        tracing::info!(
            ip = %ctx.ip,
            flavor = ?client.flavor(),
            site = %client.site(),
            "Authenticated to UniFi controller"
        );

        Ok(ProbeSuccess {
            client_probe: ClientProbe::UnifiController,
            ports: vec![PortType::new_tcp(credential.port)],
            handle: Some(Box::new(UnifiProbeHandle { client })),
        })
    }

    async fn execute(
        &self,
        ctx: &IntegrationContext<'_>,
        host_data: &mut HostData,
        _checkpoint: &Checkpoint<'_>,
    ) -> Result<Completeness, IntegrationFailure> {
        let handle = ctx
            .probe_handle
            .and_then(|h| h.downcast_ref::<UnifiProbeHandle>())
            .ok_or_else(|| Error::msg("UniFi execute ran without a probe handle"))?;

        ctx.ops.report_progress(10).await.ok();

        let devices: Vec<UnifiDevice> = handle.client.get_site("stat/device").await?.data;
        tracing::info!(ip = %ctx.ip, devices = devices.len(), "Fetched UniFi device inventory");
        ctx.ops.report_progress(40).await.ok();

        let network_id = host_data.host.base.network_id;
        let subnets = collect_subnets(ctx, host_data);
        let mapped = mapping::map_devices(&devices, network_id, &subnets);

        if mapped.len() < devices.len() {
            // Silent truncation would read as "the controller only manages these devices", and
            // in the total case as the integration doing nothing at all — which is how it
            // presented when a rescan scoped the subnet list too narrowly and dropped every
            // switch. An `info!` line was not enough: nobody reads the daemon log to find out
            // why something they configured produced no visible effect.
            let skipped = devices.len() - mapped.len();
            tracing::warn!(
                ip = %ctx.ip,
                skipped,
                total = devices.len(),
                "Skipped UniFi devices with no address in a subnet this network holds — service \
                 matching has no subnet to evaluate against for them"
            );
            ctx.ops
                .record_attempt_failure(
                    ctx.credential.into(),
                    ctx.ip,
                    AttemptOutcome::CollectionFailed,
                    format!(
                        "the controller reported {} device{}, {skipped} of which have no address \
                         on a known subnet and could not be recorded",
                        devices.len(),
                        if devices.len() == 1 { "" } else { "s" }
                    ),
                    true,
                    ctx.credential_id,
                )
                .await;
        }

        // The adopted devices' own addresses, so the client pass can skip records that describe
        // a device we already named from the inventory.
        let device_ips: Vec<std::net::IpAddr> = mapped.iter().map(|d| d.ip).collect();

        let mut created = 0usize;
        for device in mapped {
            if ctx.cancel.is_cancelled() {
                return Err(IntegrationFailure::cancelled());
            }

            // A UDM / Cloud Key is itself a UniFi device, so when the controller we scanned
            // appears in its own inventory it enriches the scanned host rather than becoming
            // a second one.
            if device.ip == ctx.ip {
                enrich_scanned_host(host_data, ctx.interface_source, &device);
                continue;
            }

            match create_device_host(ctx, &subnets, device).await {
                Ok(()) => created += 1,
                Err(e) => {
                    tracing::debug!(error = %e, "Failed to create UniFi-discovered host");
                }
            }
        }

        tracing::info!(created, "UniFi device sync complete");
        ctx.ops.report_progress(70).await.ok();

        // Clients: the devices the controller sees but has not adopted. Their names live nowhere
        // else — a phone or a server has no LLDP, and its DHCP hostname is frequently all there
        // is — so a client the sweep already found gets a better label, and one the sweep could
        // not reach becomes a host the controller is the only witness for.
        let created_clients = match handle.client.get_site::<UnifiStation>("stat/sta").await {
            Ok(envelope) => {
                let stations = envelope.data;
                let clients = mapping::map_clients(&stations, network_id, &device_ips);
                tracing::info!(
                    ip = %ctx.ip,
                    reported = stations.len(),
                    placed = clients.len(),
                    "Fetched UniFi client inventory"
                );
                controller::create_client_hosts(ctx, clients).await
            }
            Err(e) => {
                // Not fatal: the device sync above is the load-bearing half, and a controller
                // that refuses `stat/sta` should not lose the switches it did report.
                tracing::warn!(
                    ip = %ctx.ip,
                    error = %e,
                    "Could not read UniFi clients; continuing without client hosts"
                );
                0
            }
        };

        tracing::info!(created = created_clients, "UniFi client sync complete");
        ctx.ops.report_progress(90).await.ok();
        // No checkpoint: each device is committed server-side by `create_device_host` as it is
        // reached, so UniFi's progress never depended on `host_data` surviving a drop. The only
        // thing in the scratch buffer is the controller's own enrichment, which is all-or-nothing
        // by nature.
        //
        // Devices skipped for having no address on a known subnet are reported above as their own
        // issue — that is a scoping problem the operator fixes, not a collection that ran short.
        Ok(Completeness::Complete)
    }
}

/// Subnets available to place each managed device's IP in.
///
/// `known_subnets` is the network's whole address space, not the scan's scope — which is what
/// makes a rescan of the controller useful. The controller reports every switch it manages, and
/// on a segmented network almost none of them sit in the subnet the rescan is sweeping; scoping
/// this to the sweep dropped all of them.
fn collect_subnets(
    ctx: &IntegrationContext<'_>,
    host_data: &HostData,
) -> Vec<crate::server::subnets::r#impl::base::Subnet> {
    merge_subnets(ctx.known_subnets, ctx.scanning_subnet, &host_data.subnets)
}

// `merge_subnets` lives in `super` now: Instant On needs the identical union, and the rule is
// about where any controller-reported device's IP may live, not about UniFi.
use super::merge_subnets;

/// Fold the controller's own device record into the host being scanned.
///
/// `HostData` is shared across every integration for this IP, and the ports offered here are
/// merged with whatever else collected the same host rather than replacing it — `PhysicalPortsOnly`
/// says an SNMP ifTable outranks this view port for port, and a port only UniFi knows about is
/// still added. This used to be an `if !host_data.interfaces.is_empty()` bail, which dropped the
/// whole contribution and depended on SNMP happening to run first.
fn enrich_scanned_host(
    host_data: &mut HostData,
    source: InterfaceSource,
    device: &mapping::MappedDevice,
) {
    device.identity.enrich(host_data);

    host_data.contribute_interfaces(
        source,
        device.interfaces.clone(),
        interfaces_complete(),
        interface_data_complete(),
    );
}

/// Create a host for a device the controller manages but we did not scan.
async fn create_device_host(
    ctx: &IntegrationContext<'_>,
    subnets: &[crate::server::subnets::r#impl::base::Subnet],
    device: mapping::MappedDevice,
) -> Result<(), Error> {
    let mapping::MappedDevice {
        identity,
        ip_address,
        interfaces,
        device_type,
        ip,
    } = device;

    let network_id = ctx.ops.network_id().await?;
    let host = identity.into_host(network_id);

    // Run the real service matcher rather than stamping a service on. The controller's
    // reported device class enters as `ManagedDevice` evidence and `Pattern::ManagedDeviceType`
    // consumes it, exactly as the container scanner feeds `ServiceVirtualization` — so these
    // hosts get an ordinary `DiscoveryWithMatch` service with a real confidence.
    let managed_device = device_type.map(|device_type| ManagedDevice { device_type });
    // For the service matcher, which needs a subnet to evaluate its subnet patterns against —
    // not for placement, which the server redoes for every address it stores.
    let subnet = placeable_subnet(subnets, ip)
        .ok_or_else(|| Error::msg("device IP is in no subnet this network holds"))?;

    let all_ports: Vec<PortType> = vec![];
    let endpoint_responses = vec![];
    let client_responses = std::collections::HashMap::new();
    let daemon_id = ctx.ops.daemon_id().await?;
    let (services, ports) = ctx.ops.match_services(
        &host,
        &ServiceMatchBaselineParams {
            subnet,
            ip_address: &ip_address,
            all_ports: &all_ports,
            endpoint_responses: &endpoint_responses,
            virtualization_metadata: &None,
            virtualization_service_id: None,
            client_responses: &client_responses,
            managed_device: &managed_device,
            // Reported by a controller, not swept over multicast.
            dns_sd: &None,
        },
        &[],
        &daemon_id,
        &host.base.network_id,
    )?;

    ctx.ops
        .create_host(
            host,
            vec![ip_address],
            ports,
            services,
            interfaces,
            vec![],
            interfaces_complete(),
            interface_data_complete(),
            ctx.cancel,
        )
        .await?;
    Ok(())
}

/// Always `false`, deliberately.
///
/// A UniFi `port_table` is a complete list of *physical ports*, not of the device's ifTable —
/// the same switch under SNMP also reports VLAN, loopback and CPU interfaces UniFi never
/// mentions. Claiming completeness would let `should_prune_interfaces` delete those rows and,
/// with them, their server-resolved `neighbor` links (the GH #649 failure mode). Stale
/// interfaces are the safe direction; deleted ones are not.
fn interfaces_complete() -> bool {
    false
}

/// UniFi reports LLDP and the bridge FDB, but never CDP or VLAN membership. Saying so keeps
/// `preserve_uncollected_data` from nulling CDP/VLAN columns an SNMP poll populated.
fn interface_data_complete() -> InterfaceDataComplete {
    InterfaceDataComplete {
        lldp: true,
        cdp: false,
        fdb: true,
        vlan_membership: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::shared::attribution::AttributeSource;
    use crate::server::shared::types::entities::EntitySource;
    use crate::server::subnets::r#impl::base::{Subnet, SubnetBase};
    use crate::server::subnets::r#impl::base::{SubnetCidr, SubnetCidrValue};
    use crate::server::subnets::r#impl::types::SubnetType;
    use uuid::Uuid;

    fn subnet(cidr: &str) -> Subnet {
        Subnet {
            base: SubnetBase {
                name: cidr.to_string(),
                network_id: Uuid::nil(),
                cidr: SubnetCidr::new(
                    SubnetCidrValue(cidr.parse().expect("valid CIDR")),
                    AttributeSource::DaemonSelfReport,
                ),
                subnet_type: SubnetType::Lan,
                source: EntitySource::System,
                ..Default::default()
            },
            id: Uuid::new_v4(),
            ..Default::default()
        }
    }

    /// The failure Motala hit: a rescan of the controller sweeps only the controller's own
    /// subnet, but the controller manages switches on a management VLAN it never touches. When
    /// the subnet list came from the sweep, `map_devices` could place none of them and dropped
    /// every switch — so the rescan appeared to do nothing at all.
    #[test]
    fn a_managed_device_outside_the_swept_subnet_is_still_placeable() {
        let management = subnet("192.168.210.0/24");
        let controller = subnet("172.16.8.0/24");

        let available = merge_subnets(
            // The network's whole address space, which is what `known_subnets` now carries.
            &[controller.clone(), management.clone()],
            // A rescan of the controller sweeps only this one.
            Some(&controller),
            &[],
        );

        let switch: std::net::IpAddr = "192.168.210.207".parse().expect("valid IP");
        assert!(
            available.iter().any(|s| s.base.cidr.contains(&switch)),
            "a switch on the management VLAN must be placeable from a rescan of the controller"
        );
    }

    /// The sweep's subnet and anything the host's own collection found still get folded in, and
    /// nothing appears twice — `map_devices` picks the first containing subnet, so a duplicate
    /// would be a silent coin-flip over which id an address is stamped with.
    #[test]
    fn merging_is_a_union_by_id() {
        let a = subnet("10.0.0.0/24");
        let b = subnet("10.0.1.0/24");

        let merged = merge_subnets(std::slice::from_ref(&a), Some(&a), &[b.clone(), a.clone()]);

        let ids: Vec<Uuid> = merged.iter().map(|s| s.id).collect();
        assert_eq!(ids, vec![a.id, b.id]);
    }
}
