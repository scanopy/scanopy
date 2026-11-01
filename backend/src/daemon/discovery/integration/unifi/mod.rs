//! UniFi Network Application (controller) discovery integration.
//!
//! Reads `GET /stat/device` from a UniFi controller and turns the controller's inventory into
//! Scanopy hosts, interfaces, LLDP neighbors and bridge-FDB entries. Reads `rest/networkconf` and
//! `rest/portconf` first, so the controller's networks become the site's VLANs and subnets before
//! any device is placed, and each switch port carries the VLANs it is configured for.
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
pub mod networks;
pub mod ports;
pub mod types;

use crate::server::lldp::canonical_mac;
use crate::server::shared::attribution::AttributeSource;
use crate::server::subnets::r#impl::base::Subnet;
use crate::server::subnets::r#impl::inference::placeable_subnet;
use crate::server::vlans::handlers::VlanDiscoveryItem;
use std::time::Duration;

use anyhow::{Error, Result};
use async_trait::async_trait;
use futures::future::join_all;
use futures::stream::{self, StreamExt};

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
    InterfaceSource, ProbeContext, ProbeFailure, ProbeSuccess, ReportedRange, held_subnets,
    place_reported_range,
};
use crate::daemon::discovery::service::ops::HostData;
use crate::daemon::discovery::service::warnings::AttemptOutcome;
use client::UnifiClient;
use networks::RecordedNetwork;
use types::{UnifiDevice, UnifiNetwork, UnifiPortConf, UnifiStation};

/// Connected client carried from `probe` to `execute`.
struct UnifiProbeHandle {
    client: UnifiClient,
}

pub struct UnifiIntegration;

#[async_trait]
impl DiscoveryIntegration for UnifiIntegration {
    fn credential_type(&self) -> CredentialQueryPayloadDiscriminants {
        CredentialQueryPayloadDiscriminants::UnifiController
    }

    fn estimated_seconds(&self) -> u32 {
        10
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(180)
    }

    async fn probe(&self, ctx: &ProbeContext<'_>) -> Result<ProbeSuccess, ProbeFailure> {
        let CredentialQueryPayload::UnifiController(credential) = ctx.credential else {
            return Err(ProbeFailure::malformed("Not a UniFi credential"));
        };

        let client = UnifiClient::connect(
            &ctx.ip.to_string(),
            credential,
            ctx.accept_invalid_certs,
            ctx.trusted_ca,
        )
        .await
        .map_err(|e| {
            // `e` is already phrased for an operator and never contains the secret. The
            // client distinguishes a refused login from an unknown site from a transport
            // failure, so keep that distinction rather than flattening it — a wrong
            // password and a self-signed certificate send an operator to opposite ends
            // of the problem, and both used to read as "was rejected".
            ProbeFailure::with_outcome(UnifiClient::classify_connect_error(&e), e.to_string())
        })?;

        tracing::info!(
            ip = %ctx.ip,
            flavor = ?client.flavor(),
            site = %client.site(),
            "Authenticated to UniFi controller"
        );

        Ok(ProbeSuccess {
            client_probe: Some(ClientProbe::UnifiController),
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
        ctx.ops.report_progress(30).await.ok();

        // The controller's networks become the site's VLANs and subnets before any device is
        // placed, so a device on a VLAN the daemon has no interface on still lands on a subnet.
        let site_id = host_data.host.base.site_id;
        let config = fetch_network_config(ctx, &handle.client).await;
        let recorded = match &config {
            Some(config) => record_networks(ctx, site_id, &config.networks).await,
            None => Vec::new(),
        };
        ctx.ops.report_progress(40).await.ok();

        let subnets = collect_subnets(ctx, host_data, &recorded);
        let mut mapped = mapping::map_devices(&devices, site_id, &subnets);
        if let Some(config) = &config {
            apply_network_config(ctx, host_data, &devices, &mut mapped, config, &recorded);
        }

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
                "Skipped UniFi devices with no address in a subnet this site holds — service \
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

        // A UDM / Cloud Key is itself a UniFi device, so when the controller we scanned appears in
        // its own inventory it enriches the scanned host rather than becoming a second one.
        let mut remote = Vec::with_capacity(mapped.len());
        for device in mapped {
            if device.ip == ctx.ip {
                enrich_scanned_host(host_data, ctx.interface_source, &device);
            } else {
                remote.push(device);
            }
        }
        if ctx.cancel.is_cancelled() {
            return Err(IntegrationFailure::cancelled());
        }

        // Concurrently, for the reason `controller::CONCURRENT_HOST_SUBMISSIONS` gives.
        let created = stream::iter(remote)
            .map(|device| create_device_host(ctx, &subnets, device))
            .buffer_unordered(controller::CONCURRENT_HOST_SUBMISSIONS)
            .filter(|result| {
                if let Err(e) = result {
                    tracing::debug!(error = %e, "Failed to create UniFi-discovered host");
                }
                std::future::ready(result.is_ok())
            })
            .count()
            .await;

        tracing::info!(created, "UniFi device sync complete");
        ctx.ops.report_progress(70).await.ok();

        // Clients: the devices the controller sees but has not adopted. Their names live nowhere
        // else — a phone or a server has no LLDP, and its DHCP hostname is frequently all there
        // is — so a client the sweep already found gets a better label, and one the sweep could
        // not reach becomes a host the controller is the only witness for.
        let created_clients = match handle.client.get_site::<UnifiStation>("stat/sta").await {
            Ok(envelope) => {
                let stations = envelope.data;
                let clients = mapping::map_clients(&stations, site_id, &device_ips, &subnets);
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
/// `known_subnets` is the site's whole address space, not the scan's scope — which is what
/// makes a rescan of the controller useful. The controller reports every switch it manages, and
/// on a segmented network almost none of them sit in the subnet the rescan is sweeping; scoping
/// this to the sweep dropped all of them. The controller's own networks are added on top: a
/// network created this run is in no list the daemon was handed.
fn collect_subnets(
    ctx: &IntegrationContext<'_>,
    host_data: &HostData,
    recorded: &[RecordedNetwork],
) -> Vec<Subnet> {
    let from_host: Vec<Subnet> = host_data
        .subnets
        .iter()
        .cloned()
        .chain(recorded.iter().filter_map(|r| r.subnet.clone()))
        .collect();
    merge_subnets(ctx.known_subnets, ctx.scanning_subnet, &from_host)
}

/// What the controller says about its networks and port profiles.
struct NetworkConfig {
    networks: Vec<UnifiNetwork>,
    profiles: Vec<UnifiPortConf>,
}

/// Read `rest/networkconf` and `rest/portconf`. `None` when the networks cannot be read: the
/// device sync then runs as it did before networks were read, against the subnets the site
/// already holds, and the shortfall is reported rather than left to show up as skipped devices.
async fn fetch_network_config(
    ctx: &IntegrationContext<'_>,
    client: &UnifiClient,
) -> Option<NetworkConfig> {
    let networks = match client.get_site::<UnifiNetwork>("rest/networkconf").await {
        Ok(envelope) => envelope.data,
        Err(e) => {
            tracing::warn!(ip = %ctx.ip, error = %e, "Could not read UniFi networks");
            ctx.ops
                .record_attempt_failure(
                    ctx.credential.into(),
                    ctx.ip,
                    AttemptOutcome::CollectionFailed,
                    format!(
                        "could not read the controller's networks, so its VLANs and subnets were \
                         not recorded: {e}"
                    ),
                    true,
                    ctx.credential_id,
                )
                .await;
            return None;
        }
    };
    // Without profiles a port that names one cannot be resolved, which leaves that device's VLAN
    // membership unclaimed (`ports::VlanConfig::device_ports`) rather than wrong.
    let profiles = match client.get_site::<UnifiPortConf>("rest/portconf").await {
        Ok(envelope) => envelope.data,
        Err(e) => {
            tracing::warn!(ip = %ctx.ip, error = %e, "Could not read UniFi port profiles");
            Vec::new()
        }
    };
    tracing::info!(
        ip = %ctx.ip,
        networks = networks.len(),
        profiles = profiles.len(),
        "Fetched UniFi network configuration"
    );
    Some(NetworkConfig { networks, profiles })
}

/// Upsert a VLAN for each of the controller's site networks and make sure the site holds a subnet
/// for each routed one.
///
/// A range is matched to a subnet the site already holds by the rule SNMP uses for a router's
/// interfaces ([`place_reported_range`]): a range overlapping nothing is created, and one overlapping
/// a held subnet without equalling it is not. A range the site already holds exactly is still
/// submitted, so the server's CIDR match can merge the controller's name and type into the held row
/// by source; a name a person typed keeps its rank and stays.
async fn record_networks(
    ctx: &IntegrationContext<'_>,
    site_id: uuid::Uuid,
    networks: &[UnifiNetwork],
) -> Vec<RecordedNetwork> {
    let site_networks = networks::site_networks(networks);

    let items: Vec<VlanDiscoveryItem> = site_networks
        .iter()
        .map(|n| VlanDiscoveryItem {
            vlan_number: n.vlan_number,
            name: n.name.clone(),
            // An administrator named the network in the controller.
            name_source: AttributeSource::Authored(ClientProbe::UnifiController),
        })
        .collect();
    let vlan_ids = if items.is_empty() {
        std::collections::HashMap::new()
    } else {
        match ctx.ops.upsert_vlans(items, site_id, ctx.cancel).await {
            Ok(ids) => ids,
            Err(e) => {
                tracing::warn!(ip = %ctx.ip, error = %e, "Failed to record UniFi VLANs");
                ctx.ops.record_vlan_recording_failure(ctx.ip).await;
                std::collections::HashMap::new()
            }
        }
    };

    // Decide every network's subnet first, in order, so each is checked against the ranges planned
    // before it; then submit the creates together. Under ServerPoll each submission waits for the
    // server's next poll, and awaiting them one at a time cost a poll round per network, which ran
    // the integration out of time before it reached the devices.
    let mut held = held_subnets(ctx.known_subnets, ctx.scanning_subnet);
    let plans: Vec<SubnetPlan> = site_networks
        .iter()
        .map(|network| {
            let Some(range) = &network.range else {
                return SubnetPlan::Skip;
            };
            match place_reported_range(&range.cidr, range.gateway, &held) {
                // The site holds this exact range. Submit the network anyway: the server matches it
                // by CIDR and merges the controller's name and type into the held row by source,
                // which is how a subnet the daemon named after its CIDR takes the network's name.
                ReportedRange::Held(subnet) if *subnet.base.cidr == range.cidr => {
                    match network.to_subnet(site_id) {
                        Some(reported) => SubnetPlan::Submit {
                            reported: Box::new(reported),
                            fallback: Some(subnet.clone()),
                        },
                        None => SubnetPlan::Reuse(subnet.clone()),
                    }
                }
                // A wider held subnet contains the network: place on it, submit nothing.
                ReportedRange::Held(subnet) => SubnetPlan::Reuse(subnet.clone()),
                ReportedRange::Unplaced => {
                    tracing::debug!(
                        network = %network.name,
                        cidr = %range.cidr,
                        "UniFi network overlaps a held subnet that does not contain its gateway; \
                         recording no subnet for it"
                    );
                    SubnetPlan::Skip
                }
                ReportedRange::New => match network.to_subnet(site_id) {
                    Some(reported) => {
                        // Held from here on, so a later network overlapping it is not created too.
                        held.push(reported.clone());
                        SubnetPlan::Submit {
                            reported: Box::new(reported),
                            fallback: None,
                        }
                    }
                    None => SubnetPlan::Skip,
                },
            }
        })
        .collect();

    let subnets = join_all(
        site_networks
            .iter()
            .zip(plans)
            .map(|(network, plan)| async move {
                match plan {
                    SubnetPlan::Skip => None,
                    SubnetPlan::Reuse(subnet) => Some(subnet),
                    SubnetPlan::Submit { reported, fallback } => {
                        match ctx.ops.create_subnet(&reported, ctx.cancel).await {
                            Ok(created) => {
                                tracing::info!(
                                    network = %network.name,
                                    cidr = %reported.base.cidr,
                                    "Recorded UniFi network as a subnet"
                                );
                                Some(created)
                            }
                            Err(e) => {
                                tracing::warn!(
                                    network = %network.name,
                                    cidr = %reported.base.cidr,
                                    error = %e,
                                    "Failed to submit subnet for UniFi network"
                                );
                                fallback
                            }
                        }
                    }
                }
            }),
    )
    .await;

    site_networks
        .into_iter()
        .zip(subnets)
        .map(|(network, subnet)| RecordedNetwork {
            vlan_id: vlan_ids.get(&network.vlan_number).copied(),
            subnet,
            network,
        })
        .collect()
}

/// What `record_networks` does with one network's range.
enum SubnetPlan {
    /// No subnet: the network is not routed, or its range collides with one the site holds.
    Skip,
    /// Place on a subnet the site already holds.
    Reuse(Subnet),
    /// Submit the controller's subnet. `fallback` is the held row to use if the submission fails.
    Submit {
        reported: Box<Subnet>,
        fallback: Option<Subnet>,
    },
}

/// Fill in what the controller's configuration adds to the mapped devices: each switch's port
/// VLAN membership, and the gateway's addresses and interfaces on the networks it routes.
fn apply_network_config(
    ctx: &IntegrationContext<'_>,
    host_data: &HostData,
    devices: &[UnifiDevice],
    mapped: &mut [mapping::MappedDevice],
    config: &NetworkConfig,
    recorded: &[RecordedNetwork],
) {
    let vlan_config = ports::VlanConfig::new(&config.networks, recorded, &config.profiles);
    let by_mac: std::collections::HashMap<String, &UnifiDevice> = devices
        .iter()
        .filter_map(|d| Some((canonical_mac(d.mac.as_deref()?)?, d)))
        .collect();
    for device in mapped.iter_mut() {
        let Some(raw) = device
            .identity
            .chassis_id
            .as_deref()
            .and_then(|mac| by_mac.get(mac))
        else {
            continue;
        };
        if let Some(port_vlans) = vlan_config.device_ports(raw) {
            ports::apply(&mut device.interfaces, &port_vlans);
            device.vlan_membership_complete = true;
        }
    }

    let mut gateways = mapped
        .iter_mut()
        .filter(|d| networks::is_gateway(d.device_type.as_deref()));
    let (Some(gateway), None) = (gateways.next(), gateways.next()) else {
        // None: a third-party router. Several: which one routes a network is not in the payload.
        // Either way subnet↔VLAN links come only from switch forwarding tables.
        tracing::debug!(
            ip = %ctx.ip,
            "No single UniFi gateway in the inventory; not recording gateway network interfaces"
        );
        return;
    };

    // When the gateway is the scanned host its address row is the scan's, not the mapped one.
    let management_id = if gateway.ip == ctx.ip {
        host_data
            .ip_addresses
            .iter()
            .find(|a| a.base.ip_address == ctx.ip)
            .map(|a| a.id)
            .unwrap_or(gateway.ip_address.id)
    } else {
        gateway.ip_address.id
    };
    let (addresses, interfaces) = networks::gateway_interfaces(
        recorded,
        gateway.ip_address.base.site_id,
        (gateway.ip, management_id),
    );
    gateway.gateway_addresses = addresses;
    gateway.interfaces.extend(interfaces);
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
    for address in &device.gateway_addresses {
        host_data.add_ip_address(address.clone());
    }

    host_data.contribute_interfaces(
        source,
        device.interfaces.clone(),
        interfaces_complete(),
        interface_data_complete(device.vlan_membership_complete),
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
        gateway_addresses,
        vlan_membership_complete,
    } = device;

    let site_id = ctx.ops.site_id().await?;
    let host = identity.into_host(site_id);

    // Run the real service matcher rather than stamping a service on. The controller's
    // reported device class enters as `ManagedDevice` evidence and `Pattern::ManagedDeviceType`
    // consumes it, exactly as the container scanner feeds `ServiceVirtualization` — so these
    // hosts get an ordinary `DiscoveryWithMatch` service with a real confidence.
    let managed_device = device_type.map(|device_type| ManagedDevice { device_type });
    // For the service matcher, which needs a subnet to evaluate its subnet patterns against. The
    // same rule placed `ip_address` in `map_device`, so both see the same subnet.
    let subnet = placeable_subnet(subnets, ip)
        .ok_or_else(|| Error::msg("device IP is in no subnet this site holds"))?;

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
            certificates: &[],
            fingerprints: &[],
        },
        &[],
        &daemon_id,
        &host.base.site_id,
        AttributeSource::Probe(ClientProbe::UnifiController),
    )?;

    let mut ip_addresses = vec![ip_address];
    ip_addresses.extend(gateway_addresses);

    ctx.ops
        .create_integration_host(
            ctx.integration,
            host,
            ip_addresses,
            ports,
            services,
            interfaces,
            vec![],
            interfaces_complete(),
            interface_data_complete(vlan_membership_complete),
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

/// UniFi reports LLDP and the bridge FDB, never CDP, and VLAN membership only for a device whose
/// every port's configuration it could read (`ports::VlanConfig::device_ports`). Saying so keeps
/// `preserve_uncollected_data` from nulling CDP/VLAN columns an SNMP poll populated.
fn interface_data_complete(vlan_membership: bool) -> InterfaceDataComplete {
    InterfaceDataComplete {
        lldp: true,
        cdp: false,
        fdb: true,
        vlan_membership,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::shared::attribution::AttributeSource;
    use crate::server::shared::types::entities::EntitySource;
    use crate::server::subnets::r#impl::base::{Subnet, SubnetBase};
    use crate::server::subnets::r#impl::base::{SubnetCidr, SubnetCidrValue};
    use crate::server::subnets::r#impl::base::{SubnetName, SubnetTypeAttributed};
    use crate::server::subnets::r#impl::types::SubnetType;
    use uuid::Uuid;

    fn subnet(cidr: &str) -> Subnet {
        Subnet {
            base: SubnetBase {
                name: SubnetName::unattributed(cidr.to_string()),
                site_id: Uuid::nil(),
                cidr: SubnetCidr::new(
                    SubnetCidrValue(cidr.parse().expect("valid CIDR")),
                    AttributeSource::DaemonSelfReport,
                ),
                subnet_type: SubnetTypeAttributed::unattributed(SubnetType::Lan),
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
            // The site's whole address space, which is what `known_subnets` now carries.
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

    fn device_at(ip: &str, mac: &str) -> UnifiDevice {
        UnifiDevice {
            ip: Some(ip.to_string()),
            mac: Some(mac.to_string()),
            device_type: Some("usw".to_string()),
            ..Default::default()
        }
    }

    /// The failure this reads networks for: a switch on a VLAN the daemon has no interface on.
    /// With the controller's network among the placement subnets it is kept; a device on no
    /// network the site or the controller knows is still skipped, and so still reported.
    #[test]
    fn a_device_on_a_controller_network_is_placed_and_one_on_none_is_not() {
        let daemon_lan = subnet("192.168.1.0/24");
        let servers = networks::site_networks(&[UnifiNetwork {
            name: Some("Servers".to_string()),
            purpose: Some("corporate".to_string()),
            vlan_enabled: Some(types::FlexBool(true)),
            vlan: Some(types::FlexInt(20)),
            ip_subnet: Some("10.20.0.1/24".to_string()),
            ..Default::default()
        }])[0]
            .to_subnet(Uuid::nil())
            .expect("a routed network has a subnet");

        let placeable = merge_subnets(std::slice::from_ref(&daemon_lan), None, &[servers]);
        let mapped = mapping::map_devices(
            &[
                device_at("10.20.0.2", "aa:bb:cc:00:00:01"),
                device_at("172.31.0.5", "aa:bb:cc:00:00:02"),
            ],
            Uuid::nil(),
            &placeable,
        );

        let ips: Vec<String> = mapped.iter().map(|d| d.ip.to_string()).collect();
        assert_eq!(ips, vec!["10.20.0.2"]);
    }
}

#[cfg(test)]
mod lab_tests {
    use super::*;
    use crate::server::credentials::r#impl::mapping::ResolvableSecret;
    use crate::server::credentials::r#impl::types::{UnifiAuth, UnifiQueryCredential};
    use uuid::Uuid;

    /// Reads a live controller's networks and port profiles through the client `execute` uses, and
    /// resolves every profile against the networks. Uses the variables `tools/unifi/unifi-test-env.sh`
    /// reads (`UNIFI_HOST`, `UNIFI_PORT`, `UNIFI_SITE`, and `UNIFI_API_KEY` or
    /// `UNIFI_USERNAME`/`UNIFI_PASSWORD`).
    ///
    /// `cargo test --lib -- --ignored unifi::lab_tests --nocapture`
    #[tokio::test]
    #[ignore = "reads a live UniFi controller"]
    async fn reads_the_controllers_networks() {
        let var = |name: &str| std::env::var(name).ok().filter(|v| !v.is_empty());
        let host = var("UNIFI_HOST").expect("UNIFI_HOST");
        let auth = match var("UNIFI_API_KEY") {
            Some(api_key) => UnifiAuth::ApiKey {
                api_key: ResolvableSecret::Value { value: api_key },
            },
            None => UnifiAuth::LocalAdmin {
                username: var("UNIFI_USERNAME").expect("UNIFI_USERNAME or UNIFI_API_KEY"),
                password: ResolvableSecret::Value {
                    value: var("UNIFI_PASSWORD").expect("UNIFI_PASSWORD"),
                },
            },
        };
        let credential = UnifiQueryCredential {
            port: var("UNIFI_PORT").map_or(11443, |p| p.parse().expect("UNIFI_PORT")),
            site: var("UNIFI_SITE").unwrap_or_else(|| "default".to_string()),
            auth,
        };

        let client = UnifiClient::connect(&host, &credential, true, None)
            .await
            .expect("controller accepts the credential");
        let networks: Vec<UnifiNetwork> = client
            .get_site("rest/networkconf")
            .await
            .expect("rest/networkconf")
            .data;
        let profiles: Vec<UnifiPortConf> = client
            .get_site("rest/portconf")
            .await
            .expect("rest/portconf")
            .data;

        let site_networks = networks::site_networks(&networks);
        println!(
            "{:?}: {} networks, {} site VLANs, {} profiles",
            client.flavor(),
            networks.len(),
            site_networks.len(),
            profiles.len()
        );
        assert!(
            !site_networks.is_empty(),
            "every controller has at least the default LAN"
        );
        for network in &site_networks {
            println!(
                "  VLAN {:>4} {:<20} {}",
                network.vlan_number,
                network.name,
                network
                    .range
                    .as_ref()
                    .map_or("-".to_string(), |r| format!("{} via {}", r.cidr, r.gateway))
            );
            if let Some(range) = &network.range {
                assert!(
                    range.cidr.contains(&range.gateway),
                    "{}: gateway outside its range",
                    network.name
                );
            }
        }

        // A switch with one port on each profile: every profile the controller returns must read.
        let recorded: Vec<RecordedNetwork> = site_networks
            .into_iter()
            .map(|network| RecordedNetwork {
                network,
                vlan_id: Some(Uuid::new_v4()),
                subnet: None,
            })
            .collect();
        let switch = UnifiDevice {
            port_table: (1..)
                .zip(&profiles)
                .map(|(idx, profile)| types::UnifiPort {
                    port_idx: Some(types::FlexInt(idx)),
                    portconf_id: profile.id.clone(),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        };
        if !switch.port_table.is_empty() {
            let ports = ports::VlanConfig::new(&networks, &recorded, &profiles)
                .device_ports(&switch)
                .expect("every profile resolves against the controller's networks");
            for (profile, idx) in profiles.iter().zip(1..) {
                println!(
                    "  profile {:<20} native {:?}, {} tagged",
                    profile.name.as_deref().unwrap_or("?"),
                    ports[&idx].native,
                    ports[&idx].tagged.len()
                );
            }
        }
    }
}
