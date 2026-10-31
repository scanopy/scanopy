//! Proxmox VE discovery integration.
//!
//! Reads a node's API and reports every node, QEMU VM and LXC container in its cluster, each
//! guest linked to the Proxmox VE service of the node it runs on.
//!
//! # Cluster scope
//!
//! Any node's API answers for the whole cluster (`/cluster/resources`), so a credential on one
//! node is enough, and a credential on several makes each of them report the same cluster. That
//! is idempotent: guests are submitted with the addresses and NIC MACs Proxmox reports (its MACs
//! are on Proxmox's registered `BC:24:11` OUI), and the server's address and MAC matching lands
//! every repeat on the host already on file.
//!
//! # Linking a guest to its node
//!
//! `hosts.virtualization_service_id` is a foreign key to the node's Proxmox VE service, a row on
//! another host. So each node is created first, through `create_host`, and its service's stored
//! id is read back from the response and sent with that node's guests. That includes the node
//! being scanned, whose own submission only happens after `execute` returns; it lands on the
//! same host by address.
//!
//! Layering: [`types`] holds the wire structs, [`client`] the HTTP transport and auth, and
//! [`mapping`] the wire → entity translation. What any hypervisor's guests share (address
//! selection, network identities, recording, reach) is [`super::hypervisor`].

pub mod client;
pub mod mapping;
pub mod reach;
pub mod types;

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use anyhow::{Error, Result};
use async_trait::async_trait;
use uuid::Uuid;

use crate::server::credentials::r#impl::mapping::{
    CredentialQueryPayload, CredentialQueryPayloadDiscriminants,
};
use crate::server::hosts::r#impl::attributes::{HostHostnameValue, HostOsValue};
use crate::server::hosts::r#impl::virtualization::{
    HostVirtualizationDiscriminants, ProxmoxGuestType,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::shared::attribution::Attributed;
use crate::server::shared::types::guest_state::GuestState;
use crate::server::subnets::r#impl::base::Subnet;
use crate::server::subnets::r#impl::inference::placeable_subnet;

use super::hypervisor::host::{HypervisorHost, create_hypervisor_host};
use super::hypervisor::identities;
use super::hypervisor::reach::{self as guest_reach, HypervisorBridge};
use super::hypervisor::sync::{self, GuestOutcome, GuestSync, sync_guests};
use super::{
    Checkpoint, CollectionShortfall, Completeness, DiscoveryIntegration, IntegrationContext,
    IntegrationFailure, ProbeContext, ProbeFailure, ProbeSuccess, merge_subnets,
};
use crate::daemon::discovery::service::ops::HostData;
use crate::daemon::discovery::service::warnings::{AttemptOutcome, UnreachedHypervisorGuest};
use client::ProxmoxClient;
use mapping::{GuestReading, GuestSummary, NodeReading, NodeSpec, REPORTED};
use types::{
    AgentHostName, AgentInterfaces, AgentOsInfo, ClusterResource, ClusterStatusEntry, GuestConfig,
    LxcInterface, NodeNetworkEntry, NodeStatus,
};

/// Connected client carried from `probe` to `execute`.
struct ProxmoxProbeHandle {
    client: ProxmoxClient,
    port: u16,
}

pub struct ProxmoxIntegration;

#[async_trait]
impl DiscoveryIntegration for ProxmoxIntegration {
    fn credential_type(&self) -> CredentialQueryPayloadDiscriminants {
        CredentialQueryPayloadDiscriminants::Proxmox
    }

    fn estimated_seconds(&self) -> u32 {
        15
    }

    /// Two reads per node, one config read per guest, and up to three runtime reads per running
    /// guest (the agent's interfaces, OS and hostname), each bounded at 30s.
    fn timeout(&self) -> Duration {
        Duration::from_secs(300)
    }

    fn probe_gate_ports(&self, credential: &CredentialQueryPayload) -> Vec<PortType> {
        match credential {
            CredentialQueryPayload::Proxmox(c) => vec![PortType::new_tcp(c.port)],
            _ => vec![],
        }
    }

    async fn probe(&self, ctx: &ProbeContext<'_>) -> Result<ProbeSuccess, ProbeFailure> {
        let CredentialQueryPayload::Proxmox(credential) = ctx.credential else {
            return Err(ProbeFailure::malformed("Not a Proxmox VE credential"));
        };

        let (client, version) = ProxmoxClient::connect(
            &ctx.ip.to_string(),
            credential,
            ctx.accept_invalid_certs,
            ctx.trusted_ca,
        )
        .await
        .map_err(|e| {
            ProbeFailure::with_outcome(ProxmoxClient::classify_connect_error(&e), e.to_string())
        })?;

        tracing::info!(ip = %ctx.ip, version = %version.version, "Authenticated to Proxmox VE");

        Ok(ProbeSuccess {
            client_probe: Some(ClientProbe::Proxmox),
            ports: vec![PortType::new_tcp(credential.port)],
            handle: Some(Box::new(ProxmoxProbeHandle {
                client,
                port: credential.port,
            })),
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
            .and_then(|h| h.downcast_ref::<ProxmoxProbeHandle>())
            .ok_or_else(|| Error::msg("Proxmox execute ran without a probe handle"))?;
        let client = &handle.client;

        ctx.ops.report_progress(5).await.ok();

        let resources: Vec<ClusterResource> =
            client.get("/cluster/resources").await?.ok_or_else(|| {
                IntegrationFailure::with_outcome(
                    AttemptOutcome::Rejected,
                    "the API token may not read /cluster/resources; grant it the PVEAuditor role \
                     on /",
                )
            })?;
        // Sys.Audit on `/`. Without it the nodes' addresses are unknown.
        let status: Option<Vec<ClusterStatusEntry>> = client.get("/cluster/status").await?;

        let nodes = mapping::nodes(&resources, status.as_deref(), ctx.ip);
        let guests = mapping::guests(&resources);
        tracing::info!(
            ip = %ctx.ip,
            nodes = nodes.len(),
            guests = guests.len(),
            "Fetched Proxmox VE cluster inventory"
        );
        ctx.ops.report_progress(15).await.ok();

        let subnets = merge_subnets(ctx.known_subnets, ctx.scanning_subnet, &host_data.subnets);

        // The node that answered is the host being scanned; its hostname is the node name.
        if let Some(local) = nodes.iter().find(|n| n.local) {
            host_data.with_hostname(local.name.clone(), REPORTED);
        }

        // Nodes first, so each guest can carry its node's stored service id.
        let mut owners: HashMap<String, Uuid> = HashMap::new();
        let mut node_hosts: HashMap<String, Uuid> = HashMap::new();
        let mut bridges: HashMap<String, Vec<HypervisorBridge>> = HashMap::new();
        for node in &nodes {
            if ctx.cancel.is_cancelled() {
                return Err(IntegrationFailure::cancelled());
            }
            // A node with no address is never recorded, so nothing it reports would be kept.
            let reading = match node.ip {
                Some(_) => read_node(client, node).await,
                None => NodeReading::default(),
            };
            if node.local
                && let Some(os) = &reading.os
            {
                host_data.offer_os(Attributed::new(HostOsValue(os.clone()), REPORTED));
            }
            bridges.insert(node.name.clone(), reading.bridges.clone());
            match create_node_host(ctx, &subnets, node, &reading, handle.port).await {
                Ok(Some((host_id, service_id))) => {
                    owners.insert(node.name.clone(), service_id);
                    node_hosts.insert(node.name.clone(), host_id);
                }
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!(node = %node.name, error = %e, "Failed to record Proxmox VE node")
                }
            }
        }
        ctx.ops.report_progress(30).await.ok();

        let site_id = ctx.ops.site_id().await?;
        let total = guests.len();
        let done = AtomicUsize::new(0);
        // Built before the first await: a closure over a borrowed guest held across one makes
        // this future fail the `Send` check `async_trait` puts on it.
        let syncs: Vec<_> = guests
            .iter()
            .map(|guest| {
                sync_and_report(
                    ctx, client, guest, &owners, &bridges, &subnets, site_id, &done, total,
                )
            })
            .collect();
        let GuestSync {
            created,
            unidentifiable,
            short_owners: short_nodes,
            identities_listed,
        } = sync_guests(syncs).await;
        if ctx.cancel.is_cancelled() {
            return Err(IntegrationFailure::cancelled());
        }

        tracing::info!(created, "Proxmox VE guest sync complete");
        if !unidentifiable.is_empty() {
            // No address and no NIC: nothing on the network to find, and nothing that would keep
            // it one host from scan to scan. A log line rather than a scan warning, since the
            // guest is configured that way on purpose.
            tracing::info!(
                guests = %unidentifiable.join(", "),
                "Proxmox VE guests with no network interface were not recorded"
            );
        }

        // `/cluster/resources` lists every guest, stopped ones included. A node whose guests were
        // all recorded has them listed in full, so a stored guest of it that this run didn't
        // report was removed. A guest with no NIC and no address was never stored, so leaving it
        // out holds nothing back.
        if let Some(credential_id) = ctx.credential_id {
            let integration = ctx.credential.into();
            for (name, host_id) in &node_hosts {
                if !short_nodes.contains(name) {
                    ctx.ops
                        .record_guests_listed(credential_id, integration, *host_id)
                        .await;
                }
            }
            for host_id in identities_listed {
                ctx.ops
                    .record_guests_listed(credential_id, integration, host_id)
                    .await;
            }
        }

        // A guest whose node has no address cannot be linked to it. That is a shortfall the
        // operator can fix (grant Sys.Audit), so it is reported as one.
        let unplaced_nodes = nodes
            .iter()
            .filter(|n| !owners.contains_key(&n.name))
            .count();
        if unplaced_nodes > 0 {
            return Ok(Completeness::Partial(CollectionShortfall {
                what: "Proxmox VE nodes",
                collected: nodes.len() - unplaced_nodes,
                expected: nodes.len(),
            }));
        }
        Ok(Completeness::Complete)
    }
}

/// Sync one guest unless the run was cancelled, then report progress over all `total`.
#[allow(clippy::too_many_arguments)]
async fn sync_and_report(
    ctx: &IntegrationContext<'_>,
    client: &ProxmoxClient,
    guest: &GuestSummary,
    owners: &HashMap<String, Uuid>,
    bridges: &HashMap<String, Vec<HypervisorBridge>>,
    subnets: &[Subnet],
    site_id: Uuid,
    done: &AtomicUsize,
    total: usize,
) -> GuestOutcome {
    let outcome = if ctx.cancel.is_cancelled() {
        GuestOutcome::Failed {
            owner: guest.node.clone(),
        }
    } else {
        sync_guest(ctx, client, guest, owners, bridges, subnets, site_id).await
    };
    let synced = done.fetch_add(1, Ordering::Relaxed) + 1;
    ctx.ops
        .report_progress(30 + (60 * synced / total) as u8)
        .await
        .ok();
    outcome
}

/// Read one guest and record it with its network identities. A running guest at addresses no subnet
/// this scan covers is recorded as unreached, with what would reach it.
async fn sync_guest(
    ctx: &IntegrationContext<'_>,
    client: &ProxmoxClient,
    guest: &GuestSummary,
    owners: &HashMap<String, Uuid>,
    bridges: &HashMap<String, Vec<HypervisorBridge>>,
    subnets: &[Subnet],
    site_id: Uuid,
) -> GuestOutcome {
    let reading = read_guest(client, guest).await;
    let Some(record) = mapping::guest_host(
        guest,
        &reading,
        owners.get(&guest.node).copied(),
        subnets,
        site_id,
    ) else {
        return GuestOutcome::Unidentifiable(guest_label(guest));
    };
    let identities =
        identities::network_identities(&reading.nics, &reading.reported, subnets, REPORTED);
    let label = guest_label(guest);
    match sync::record_guest(ctx, subnets, &label, record, &identities, site_id).await {
        Ok((host_id, all_identities_recorded)) => {
            if guest.state == GuestState::Running
                && let Some(unreached) = guest_reach::unreached(
                    &reading.nics,
                    &reading.addresses,
                    bridges.get(&guest.node).map_or(&[][..], Vec::as_slice),
                    ctx.scanned_subnets,
                    &guest.node,
                )
            {
                ctx.ops
                    .record_unreached_guest(UnreachedHypervisorGuest {
                        ip: unreached.address,
                        guest: guest.name.clone().unwrap_or_else(|| guest.vmid.to_string()),
                        integration: CredentialQueryPayloadDiscriminants::Proxmox,
                        reason: unreached.reason,
                    })
                    .await;
            }
            GuestOutcome::Recorded {
                host_id,
                identities_listed: reading.interfaces_read && all_identities_recorded,
            }
        }
        Err(e) => {
            tracing::warn!(guest = %label, error = %e, "Failed to record Proxmox VE guest");
            GuestOutcome::Failed {
                owner: guest.node.clone(),
            }
        }
    }
}

/// `pve/110 (gitlab)`, for log lines.
fn guest_label(guest: &GuestSummary) -> String {
    match &guest.name {
        Some(name) => format!("{}/{} ({name})", guest.node, guest.vmid),
        None => format!("{}/{}", guest.node, guest.vmid),
    }
}

/// What a guest's node reports about it.
///
/// Its addresses are read from inside it when it runs (the QEMU guest agent, or the container's
/// interfaces), kept only where they sit on one of its configured NICs; else the static addresses
/// its config sets (an LXC `ip=`, or a VM's cloud-init `ipconfigN`). See
/// [`mapping::select_addresses`]. The NICs the config declares come with them, and identify a
/// guest no address is reported for. A container's hostname is in its config; a VM's OS and
/// hostname come from its guest agent, asked only once the agent has answered the interface read.
async fn read_guest(client: &ProxmoxClient, guest: &GuestSummary) -> GuestReading {
    let path = guest.path();
    let config = client
        .get_best_effort::<GuestConfig>(&format!("{path}/config"))
        .await;
    let nics = config
        .as_ref()
        .map(mapping::config_nics)
        .unwrap_or_default();
    // An LXC container's config sets its hostname and names its distribution; a VM's says
    // neither, and its guest agent is asked below.
    let mut reading = match (guest.guest_type, config.as_ref()) {
        (ProxmoxGuestType::Lxc, Some(config)) => GuestReading {
            hostname: mapping::config_hostname(config)
                .map(|h| Attributed::new(HostHostnameValue(h), mapping::GUEST_CONFIG)),
            os: mapping::config_os(config)
                .map(|os| Attributed::new(HostOsValue(os), mapping::GUEST_CONFIG)),
            ..Default::default()
        },
        _ => GuestReading::default(),
    };

    let runtime: Option<Vec<mapping::GuestAddress>> = if guest.state == GuestState::Running {
        match guest.guest_type {
            ProxmoxGuestType::Qemu => {
                match client
                    .get_best_effort::<AgentInterfaces>(&format!(
                        "{path}/agent/network-get-interfaces"
                    ))
                    .await
                {
                    Some(agent) => {
                        let (os_path, hostname_path) = (
                            format!("{path}/agent/get-osinfo"),
                            format!("{path}/agent/get-host-name"),
                        );
                        let (os, hostname) = tokio::join!(
                            client.get_best_effort::<AgentOsInfo>(&os_path),
                            client.get_best_effort::<AgentHostName>(&hostname_path),
                        );
                        reading.os = os
                            .as_ref()
                            .and_then(mapping::guest_os)
                            .map(|os| Attributed::new(HostOsValue(os), mapping::GUEST_AGENT));
                        reading.hostname = hostname
                            .as_ref()
                            .and_then(mapping::agent_hostname)
                            .map(|h| Attributed::new(HostHostnameValue(h), mapping::GUEST_AGENT));
                        Some(mapping::agent_addresses(&agent))
                    }
                    None => None,
                }
            }
            ProxmoxGuestType::Lxc => client
                .get_best_effort::<Vec<LxcInterface>>(&format!("{path}/interfaces"))
                .await
                .map(|ifaces| mapping::lxc_addresses(&ifaces)),
        }
    } else {
        None
    };

    reading.interfaces_read = runtime.is_some();
    let runtime = runtime.unwrap_or_default();
    reading.addresses = mapping::select_addresses(&nics, runtime.clone());
    reading.reported = runtime;
    reading.nics = nics;
    reading
}

/// What a node's own API reports about it: its OS from `/status` and its addresses from
/// `/network`. Both need `Sys.Audit` on the node; a token without it, or a node that is offline,
/// leaves them empty.
async fn read_node(client: &ProxmoxClient, node: &NodeSpec) -> NodeReading {
    let (status_path, network_path) = (
        format!("/nodes/{}/status", node.name),
        format!("/nodes/{}/network", node.name),
    );
    let (status, network) = tokio::join!(
        client.get_best_effort::<NodeStatus>(&status_path),
        client.get_best_effort::<Vec<NodeNetworkEntry>>(&network_path),
    );
    NodeReading {
        os: status.as_ref().map(mapping::node_os),
        addresses: network
            .as_deref()
            .map(mapping::node_addresses)
            .unwrap_or_default(),
        bridges: network
            .as_deref()
            .map(reach::node_bridges)
            .unwrap_or_default(),
    }
}

/// Record a node's host with its Proxmox VE service ([`create_hypervisor_host`]), returning the
/// host's and that service's stored ids. `None` when the node has no known address.
async fn create_node_host(
    ctx: &IntegrationContext<'_>,
    subnets: &[Subnet],
    node: &NodeSpec,
    reading: &NodeReading,
    port: u16,
) -> Result<Option<(Uuid, Uuid)>, Error> {
    let Some(ip) = node.ip else {
        return Ok(None);
    };
    if placeable_subnet(subnets, ip).is_none() {
        tracing::warn!(node = %node.name, %ip, "Proxmox VE node address is in no known subnet");
        return Ok(None);
    }
    let site_id = ctx.ops.site_id().await?;
    // The address it is reached at comes first, which `create_hypervisor_host` matches on.
    let (host, ip_addresses) = mapping::node_host(node, ip, reading, subnets, site_id);
    create_hypervisor_host(
        ctx,
        subnets,
        HypervisorHost {
            host,
            ip_addresses,
            // The node answered the API, which is what the Proxmox VE definition's
            // `ClientResponse` arm matches.
            client_probe: ClientProbe::Proxmox,
            port,
            source: REPORTED,
            guests: HostVirtualizationDiscriminants::Proxmox,
        },
    )
    .await
}

#[cfg(test)]
mod lab_tests {
    use super::*;
    use crate::server::credentials::r#impl::mapping::{ProxmoxQueryCredential, ResolvableSecret};

    /// Reads the lab's API live, through the same client and mapping `execute` uses, and prints
    /// what a scan would record. Needs `~/.config/scanopy-lab/proxmox.env` (see `tools/wol/`);
    /// prefers the read-only `PROXMOX_DISCOVERY_TOKEN_*` and falls back to the lab token.
    ///
    /// `cargo test --lib -- --ignored proxmox::lab_tests --nocapture`
    #[tokio::test]
    #[ignore = "reads a live Proxmox VE lab"]
    async fn reads_the_lab_cluster() {
        let path = std::env::var("PROXMOX_ENV_FILE").unwrap_or_else(|_| {
            format!(
                "{}/.config/scanopy-lab/proxmox.env",
                std::env::var("HOME").unwrap()
            )
        });
        let env: HashMap<String, String> = std::fs::read_to_string(&path)
            .expect("lab env file")
            .lines()
            .filter_map(|l| l.split_once('='))
            .map(|(k, v)| {
                (
                    k.trim().to_string(),
                    v.trim().trim_matches(['"', '\'']).to_string(),
                )
            })
            .collect();
        let var = |names: &[&str]| {
            names
                .iter()
                .find_map(|n| env.get(*n).filter(|v| !v.is_empty()).cloned())
                .unwrap_or_else(|| panic!("{names:?} not set in {path}"))
        };
        let host = var(&["PROXMOX_HOST"]);
        let credential = ProxmoxQueryCredential {
            port: crate::server::credentials::r#impl::types::default_proxmox_port(),
            token_id: var(&["PROXMOX_DISCOVERY_TOKEN_ID", "PROXMOX_TOKEN_ID"]),
            token_secret: ResolvableSecret::Value {
                value: var(&["PROXMOX_DISCOVERY_TOKEN_SECRET", "PROXMOX_TOKEN_SECRET"]),
            },
        };

        let (client, version) = ProxmoxClient::connect(&host, &credential, true, None)
            .await
            .expect("token accepted");
        let resources: Vec<ClusterResource> = client
            .get("/cluster/resources")
            .await
            .unwrap()
            .expect("token may read /cluster/resources");
        let status: Option<Vec<ClusterStatusEntry>> = client.get("/cluster/status").await.unwrap();
        let scanned: std::net::IpAddr = host.parse().expect("PROXMOX_HOST is an address");

        let nodes = mapping::nodes(&resources, status.as_deref(), scanned);
        let guests = mapping::guests(&resources);
        println!(
            "PVE {}; /cluster/status {}",
            version.version,
            if status.is_some() {
                "readable"
            } else {
                "denied"
            }
        );
        // A /22 around the scanned address stands in for the subnet a scan would have found it
        // in, so the printed subnet ids show which addresses a live subnet would place.
        let subnets = vec![lab_subnet(scanned)];
        println!(
            "lab subnet {} = {}",
            subnets[0].base.cidr.value().0,
            subnets[0].id
        );
        let placed = |rows: &[crate::server::ip_addresses::r#impl::base::IPAddress]| {
            rows.iter()
                .map(|r| {
                    format!(
                        "{} ({}) subnet {}",
                        r.base.ip_address,
                        r.base.name.as_deref().unwrap_or("-"),
                        r.base.subnet_id
                    )
                })
                .collect::<Vec<_>>()
        };
        for node in &nodes {
            let reading = read_node(&client, node).await;
            println!("node {node:?}");
            println!(
                "  os {:?}",
                reading
                    .os
                    .as_ref()
                    .map(|os| (os.to_string(), &os.kernel_version))
            );
            if let Some(ip) = node.ip {
                let (_, rows) = mapping::node_host(node, ip, &reading, &subnets, uuid::Uuid::nil());
                println!("  addresses {:?}", placed(&rows));
            }
        }
        for guest in &guests {
            let reading = read_guest(&client, guest).await;
            let record = mapping::guest_host(guest, &reading, None, &subnets, uuid::Uuid::nil());
            println!("guest {} {:?}", guest_label(guest), guest.guest_type);
            println!(
                "  os {:?} hostname {:?}",
                reading.os.as_ref().map(|os| (
                    os.value().0.family,
                    os.value().0.to_string(),
                    &os.value().0.codename,
                    &os.value().0.kernel_version,
                    os.source(),
                )),
                reading
                    .hostname
                    .as_ref()
                    .map(|h| (&h.value().0, h.source()))
            );
            println!(
                "  addresses {:?} interfaces {:?}",
                record.as_ref().map(|r| placed(&r.ip_addresses)),
                record.as_ref().map(|r| {
                    r.interfaces
                        .iter()
                        .map(|i| i.base.mac_address.as_ref().map(|m| m.value().0.to_string()))
                        .collect::<Vec<_>>()
                })
            );
            let identities = identities::network_identities(
                &reading.nics,
                &reading.reported,
                &subnets,
                REPORTED,
            );
            if !identities.is_empty() {
                println!("  network identities {identities:?}");
            }
        }
        assert!(!nodes.is_empty(), "the lab has a node");
        assert!(!guests.is_empty(), "the lab has a guest");
    }

    fn lab_subnet(scanned: std::net::IpAddr) -> Subnet {
        use crate::server::shared::storage::traits::Storable;
        use crate::server::subnets::r#impl::base::{SubnetBase, SubnetCidr, SubnetCidrValue};
        let cidr = cidr::IpInet::new(scanned, 22).unwrap().network();
        Subnet::new(SubnetBase {
            cidr: SubnetCidr::new(
                SubnetCidrValue(cidr),
                crate::server::shared::attribution::AttributeSource::Manual,
            ),
            ..Default::default()
        })
    }
}
