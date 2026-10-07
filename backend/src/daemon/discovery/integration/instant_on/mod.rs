//! HPE Networking Instant On (cloud portal) discovery integration.
//!
//! Reads a site's `inventory` and `clientSummary` from the Instant On cloud API and turns them
//! into Scanopy hosts, interfaces, uplink neighbors and bridge-FDB entries.
//!
//! # Why this exists
//!
//! Instant On switches expose SNMP only in local management mode, which means giving up the cloud
//! portal they are bought for. An operator who will not make that trade has no on-device source of
//! port or neighbor data at all — a walk gets nothing because nothing is listening. The portal's
//! own API has the data, and Instant On derives device-to-device uplinks itself rather than from
//! LLDP, so it is authoritative even where LLDP is not running.
//!
//! # Shape
//!
//! Like UniFi, one credential describes an endpoint reporting on *many* devices. Unlike UniFi,
//! that endpoint is not on the operator's network at all — `ctx.ip` is not where the request goes.
//! It is the switch the credential is bound to, which is what makes it the *anchor*: the device
//! whose inventory entry matches it enriches the scanned host, and every other device in the site
//! is created via `ops.create_host`.
//!
//! Everything produced here flows into the same ingestion and neighbor-resolution path SNMP feeds;
//! there is no Instant On-specific topology code anywhere on the server.
//!
//! Layering matches `unifi/`: [`types`] holds the raw wire structs, [`client`] the HTTP transport
//! and the PKCE token exchange, and [`mapping`] the wire → entity translation. A field-shape
//! correction is a one-file change — which matters more here than usual, because HPE publishes no
//! contract for any of it.

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

use super::controller::{self, MappedClient};
use super::{
    Checkpoint, CollectionShortfall, Completeness, DiscoveryIntegration, IntegrationContext,
    IntegrationFailure, InterfaceSource, InterfaceViewScope, ProbeContext, ProbeFailure,
    ProbeSuccess,
};
use crate::daemon::discovery::service::ops::HostData;
use crate::daemon::discovery::service::warnings::AttemptOutcome;
use client::InstantOnClient;
use types::{InstantOnClient as InstantOnClientRecord, InstantOnDevice, InstantOnSite};

/// Authenticated portal session carried from `probe` to `execute`.
struct InstantOnProbeHandle {
    client: InstantOnClient,
}

pub struct InstantOnIntegration;

#[async_trait]
impl DiscoveryIntegration for InstantOnIntegration {
    /// The portal reports switch ports and AP uplinks, not a device's whole ifTable.
    fn interface_view_scope(&self) -> InterfaceViewScope {
        InterfaceViewScope::PhysicalPortsOnly
    }

    fn credential_type(&self) -> CredentialQueryPayloadDiscriminants {
        CredentialQueryPayloadDiscriminants::InstantOn
    }

    fn estimated_seconds(&self) -> u32 {
        15
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(180)
    }

    /// Empty, and not an oversight: there is no port on the target host to gate on, because the
    /// request goes to HPE's cloud rather than to the host at all.
    fn probe_gate_ports(&self, _credential: &CredentialQueryPayload) -> Vec<PortType> {
        vec![]
    }

    async fn probe(&self, ctx: &ProbeContext<'_>) -> Result<ProbeSuccess, ProbeFailure> {
        let CredentialQueryPayload::InstantOn(credential) = ctx.credential else {
            return Err(ProbeFailure::malformed("Not an Instant On credential"));
        };

        let client = InstantOnClient::connect(credential).await.map_err(|e| {
            // `e` is already phrased for an operator and never contains the password. The client
            // separates a refused sign-in from an MFA-blocked account from a transport failure,
            // so keep that distinction rather than flattening it — they send an operator to
            // completely different places.
            ProbeFailure::with_outcome(InstantOnClient::classify_connect_error(&e), e.to_string())
        })?;

        tracing::info!(
            ip = %ctx.ip,
            site = client.site_filter().unwrap_or("(all sites)"),
            "Authenticated to the Instant On portal"
        );

        Ok(ProbeSuccess {
            client_probe: Some(ClientProbe::InstantOn),
            // Nothing was proven about a port on this host — the authentication happened
            // elsewhere entirely.
            ports: vec![],
            handle: Some(Box::new(InstantOnProbeHandle { client })),
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
            .and_then(|h| h.downcast_ref::<InstantOnProbeHandle>())
            .ok_or_else(|| Error::msg("Instant On execute ran without a probe handle"))?;

        ctx.ops.report_progress(5).await.ok();

        let sites = self.sites_to_read(handle).await?;
        if sites.is_empty() {
            return Err(IntegrationFailure::from(Error::msg(
                "the Instant On account can see no sites",
            )));
        }

        let site_id = host_data.host.base.site_id;
        let subnets = collect_subnets(ctx, host_data);
        let site_count = sites.len();
        let mut read = 0usize;
        let mut created = 0usize;
        let mut created_clients = 0usize;
        let mut anchor_matched = false;

        for (index, site) in sites.iter().enumerate() {
            if ctx.cancel.is_cancelled() {
                return Err(IntegrationFailure::cancelled());
            }
            let Some(instant_on_site_id) = site.id.as_deref() else {
                continue;
            };

            match self
                .read_site(ctx, handle, instant_on_site_id, &subnets, site_id)
                .await
            {
                Ok((devices, clients)) => {
                    read += 1;
                    // Clients before devices: a client record that describes an adopted device
                    // is already excluded, and doing them first keeps the device loop's early
                    // `continue` paths from skipping them.
                    created_clients += controller::create_client_hosts(ctx, clients).await;
                    for device in devices {
                        if ctx.cancel.is_cancelled() {
                            return Err(IntegrationFailure::cancelled());
                        }
                        // The anchor: the switch this credential is bound to appears in its own
                        // site inventory, so it enriches the scanned host rather than becoming a
                        // second one.
                        if device.ip == ctx.ip {
                            anchor_matched = true;
                            enrich_scanned_host(host_data, ctx.interface_source, &device);
                            continue;
                        }
                        match create_device_host(ctx, &subnets, device).await {
                            Ok(()) => created += 1,
                            Err(e) => {
                                tracing::debug!(error = %e, "Failed to create Instant On host");
                            }
                        }
                    }
                }
                Err(e) => {
                    // One unreadable site must not discard the sites that did read. Report it and
                    // let `Completeness::Partial` carry the shortfall.
                    tracing::warn!(site = %instant_on_site_id, error = %e, "Could not read Instant On site");
                    ctx.ops
                        .record_attempt_failure(
                            ctx.credential.into(),
                            ctx.ip,
                            AttemptOutcome::CollectionFailed,
                            format!(
                                "could not read Instant On site {}: {e}",
                                site.name.as_deref().unwrap_or(instant_on_site_id)
                            ),
                            true,
                            ctx.credential_id,
                        )
                        .await;
                }
            }

            let percent = 10 + ((index + 1) * 80 / site_count) as u8;
            ctx.ops.report_progress(percent.min(90)).await.ok();
        }

        // Anchoring on a host the account does not manage is a configuration mistake that would
        // otherwise present as a clean scan that mysteriously conjured a pile of unrelated hosts.
        if !anchor_matched {
            ctx.ops
                .record_attempt_failure(
                    ctx.credential.into(),
                    ctx.ip,
                    AttemptOutcome::CollectionFailed,
                    "this host is not in any Instant On site this account manages — assign the \
                     credential to one of the Instant On devices it reports on"
                        .to_string(),
                    true,
                    ctx.credential_id,
                )
                .await;
        }

        tracing::info!(
            created,
            clients = created_clients,
            sites = read,
            "Instant On device sync complete"
        );
        // No checkpoint: each device is committed server-side by `create_device_host` as it is
        // reached, so progress never depended on `host_data` surviving a drop. The only thing in
        // the scratch buffer is the anchor's own enrichment, which is all-or-nothing by nature.
        if read == site_count {
            Ok(Completeness::Complete)
        } else {
            Ok(Completeness::Partial(CollectionShortfall {
                what: "sites",
                collected: read,
                expected: site_count,
            }))
        }
    }
}

impl InstantOnIntegration {
    /// The sites this run should read: the one the credential names, or every site the account
    /// can see.
    async fn sites_to_read(&self, handle: &InstantOnProbeHandle) -> Result<Vec<InstantOnSite>> {
        let sites = handle
            .client
            .get::<InstantOnSite>("api/sites")
            .await?
            .elements;

        let Some(wanted) = handle.client.site_filter() else {
            return Ok(sites);
        };

        // Match on name or id, because an operator reading the portal sees the name while the API
        // keys on the id, and both are reasonable things to have pasted into the field.
        let matched: Vec<InstantOnSite> = sites
            .iter()
            .filter(|s| s.name.as_deref() == Some(wanted) || s.id.as_deref() == Some(wanted))
            .cloned()
            .collect();

        if matched.is_empty() {
            let available: Vec<&str> = sites.iter().filter_map(|s| s.name.as_deref()).collect();
            return Err(Error::msg(format!(
                "Instant On site '{wanted}' was not found on this account. Available sites: {}",
                if available.is_empty() {
                    "none visible to this account".to_string()
                } else {
                    available.join(", ")
                }
            )));
        }
        Ok(matched)
    }

    /// Fetch one site's inventory and clients, and map them.
    ///
    /// A failure to read *clients* is not a failure of the site: the devices, their ports and
    /// their uplinks are the substance, and client MACs only enrich them. Losing the whole site's
    /// topology because one supplementary call failed would be a bad trade.
    async fn read_site(
        &self,
        ctx: &IntegrationContext<'_>,
        handle: &InstantOnProbeHandle,
        instant_on_site_id: &str,
        subnets: &[crate::server::subnets::r#impl::base::Subnet],
        site_id: uuid::Uuid,
    ) -> Result<(Vec<mapping::MappedDevice>, Vec<MappedClient>)> {
        let devices: Vec<InstantOnDevice> = handle
            .client
            .get_site::<InstantOnDevice>(instant_on_site_id, "inventory")
            .await?
            .elements;

        let clients: Vec<InstantOnClientRecord> = match handle
            .client
            .get_site::<InstantOnClientRecord>(instant_on_site_id, "clientSummary")
            .await
        {
            Ok(envelope) => envelope.elements,
            Err(e) => {
                tracing::warn!(
                    site = %instant_on_site_id,
                    error = %e,
                    "Could not read Instant On clients; continuing without port MAC attachment"
                );
                Vec::new()
            }
        };

        tracing::info!(
            site = %instant_on_site_id,
            devices = devices.len(),
            clients = clients.len(),
            "Fetched Instant On site inventory"
        );

        let mapped = mapping::map_devices(&devices, &clients, site_id, subnets);
        let device_ips: Vec<std::net::IpAddr> = mapped.iter().map(|d| d.ip).collect();
        let mapped_clients = mapping::map_clients(&clients, site_id, &device_ips, subnets);

        if mapped.len() < devices.len() {
            // Silent truncation would read as "the site only contains these devices", and in the
            // total case as the integration doing nothing at all. Same failure mode the UniFi
            // integration hit when a rescan scoped the subnet list too narrowly.
            let skipped = devices.len() - mapped.len();
            tracing::warn!(
                site = %instant_on_site_id,
                skipped,
                total = devices.len(),
                "Skipped Instant On devices with no IP in a known subnet — host identity is \
                 IP-based, so they cannot be deduplicated across scans"
            );
            ctx.ops
                .record_attempt_failure(
                    ctx.credential.into(),
                    ctx.ip,
                    AttemptOutcome::CollectionFailed,
                    format!(
                        "the site reported {} device{}, {skipped} of which have no address on a \
                         known subnet and could not be recorded",
                        devices.len(),
                        if devices.len() == 1 { "" } else { "s" }
                    ),
                    true,
                    ctx.credential_id,
                )
                .await;
        }

        Ok((mapped, mapped_clients))
    }
}

/// Subnets available to place each managed device's IP in.
///
/// `known_subnets` is the site's whole address space, not the scan's scope — which is what
/// makes a rescan of the anchor useful. The site reports every device it manages, and on a
/// segmented network almost none of them sit in the subnet the rescan is sweeping.
fn collect_subnets(
    ctx: &IntegrationContext<'_>,
    host_data: &HostData,
) -> Vec<crate::server::subnets::r#impl::base::Subnet> {
    super::merge_subnets(ctx.known_subnets, ctx.scanning_subnet, &host_data.subnets)
}

/// Fold the portal's own device record into the host being scanned.
///
/// `HostData` is shared across every integration for this IP, and these ports are merged with
/// whatever else collected the same host rather than replacing it — `PhysicalPortsOnly` says an
/// SNMP ifTable outranks this view port for port, and a port only the portal knows about is still
/// added. That matters less on an Instant On switch, where SNMP being unavailable is the reason
/// this integration exists, than on an anchor host that is some other reachable device.
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

/// Create a host for a device the site manages but we did not scan.
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

    let site_id = ctx.ops.site_id().await?;
    let host = identity.into_host(site_id);

    // Run the real service matcher rather than stamping a service on. The portal's reported device
    // class enters as `ManagedDevice` evidence and `Pattern::ManagedDeviceType` consumes it, so
    // these hosts get an ordinary `DiscoveryWithMatch` service with a real confidence — and are
    // identified as a switch or an access point, not merely as "Instant On".
    let managed_device = device_type.map(|device_type| ManagedDevice { device_type });
    // For the service matcher, which needs a subnet to evaluate its subnet patterns against. The
    // same rule placed `ip_address` in `map_device`. `placeable_subnet` rather than first-match:
    // the list carries the `0.0.0.0/0` organizational rows, which contain every IPv4 address, so
    // `find` returned `Internet` for every device.
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
        },
        &[],
        &daemon_id,
        &host.base.site_id,
    )?;

    ctx.ops
        .create_integration_host(
            ctx.integration,
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
/// The portal's port list is a complete list of *physical ports*, not of the device's ifTable —
/// the same switch under SNMP would also report VLAN, loopback and CPU interfaces the portal never
/// mentions. Claiming completeness would let `should_prune_interfaces` delete those rows and, with
/// them, their server-resolved `neighbor` links. Stale interfaces are the safe direction; deleted
/// ones are not.
fn interfaces_complete() -> bool {
    false
}

/// The portal reports uplink neighbors and per-port client MACs, but never CDP, and its per-port
/// VLAN is a single access VLAN rather than the membership list SNMP walks. Saying so keeps
/// `preserve_uncollected_data` from nulling columns an SNMP poll populated.
fn interface_data_complete() -> InterfaceDataComplete {
    InterfaceDataComplete {
        lldp: true,
        cdp: false,
        fdb: true,
        vlan_membership: false,
    }
}
