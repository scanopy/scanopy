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
//! [`mapping`] the wire → entity translation.

pub mod client;
pub mod mapping;
pub mod types;

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Error, Result};
use async_trait::async_trait;
use uuid::Uuid;

use crate::server::credentials::r#impl::mapping::{
    CredentialQueryPayload, CredentialQueryPayloadDiscriminants,
};
use crate::server::hosts::r#impl::virtualization::{
    HostVirtualizationDiscriminants, ProxmoxGuestType,
};
use crate::server::interfaces::r#impl::base::InterfaceDataComplete;
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::base::ServiceMatchBaselineParams;
use crate::server::services::r#impl::definitions::{ServiceDefinitionExt, VirtualizationRole};
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::subnets::r#impl::base::Subnet;
use crate::server::subnets::r#impl::inference::placeable_subnet;

use super::{
    Checkpoint, CollectionShortfall, Completeness, DiscoveryIntegration, IntegrationContext,
    IntegrationFailure, InterfaceViewScope, ProbeContext, ProbeFailure, ProbeSuccess,
    merge_subnets,
};
use crate::daemon::discovery::service::ops::HostData;
use crate::daemon::discovery::service::warnings::AttemptOutcome;
use client::ProxmoxClient;
use mapping::{GuestAddress, GuestSummary, NodeSpec};
use types::{AgentInterfaces, ClusterResource, ClusterStatusEntry, GuestConfig, LxcInterface};

/// Connected client carried from `probe` to `execute`.
struct ProxmoxProbeHandle {
    client: ProxmoxClient,
    port: u16,
}

pub struct ProxmoxIntegration;

#[async_trait]
impl DiscoveryIntegration for ProxmoxIntegration {
    /// The node's own NICs come from SNMP or SSH; the API's view of them is the Linux bridge
    /// config, not an interface table.
    fn interface_view_scope(&self) -> InterfaceViewScope {
        InterfaceViewScope::NoInterfaces
    }

    fn credential_type(&self) -> CredentialQueryPayloadDiscriminants {
        CredentialQueryPayloadDiscriminants::Proxmox
    }

    fn estimated_seconds(&self) -> u32 {
        15
    }

    /// One config read per guest plus one runtime read per running guest, each bounded at 30s.
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
            host_data.with_hostname(
                local.name.clone(),
                crate::server::shared::attribution::AttributeSource::Probe(ClientProbe::Proxmox),
            );
        }

        // Nodes first, so each guest can carry its node's stored service id.
        let mut owners: HashMap<String, Uuid> = HashMap::new();
        for node in &nodes {
            if ctx.cancel.is_cancelled() {
                return Err(IntegrationFailure::cancelled());
            }
            match create_node_host(ctx, &subnets, node, handle.port).await {
                Ok(Some(service_id)) => {
                    owners.insert(node.name.clone(), service_id);
                }
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!(node = %node.name, error = %e, "Failed to record Proxmox VE node")
                }
            }
        }
        ctx.ops.report_progress(30).await.ok();

        let network_id = ctx.ops.network_id().await?;
        let total = guests.len();
        let mut created = 0usize;
        let mut without_address: Vec<String> = vec![];
        for (i, guest) in guests.iter().enumerate() {
            if ctx.cancel.is_cancelled() {
                return Err(IntegrationFailure::cancelled());
            }
            let addresses = guest_addresses(client, guest).await;
            if addresses.is_empty() {
                without_address.push(guest_label(guest));
                continue;
            }
            let (host, ips) = mapping::guest_host(
                guest,
                &addresses,
                owners.get(&guest.node).copied(),
                network_id,
            );
            match ctx
                .ops
                .create_host(
                    host,
                    ips,
                    vec![],
                    vec![],
                    vec![],
                    vec![],
                    // The API reports addresses, never the guest's interface table.
                    false,
                    InterfaceDataComplete::default(),
                    ctx.cancel,
                )
                .await
            {
                Ok(_) => created += 1,
                Err(e) => {
                    tracing::warn!(guest = %guest_label(guest), error = %e, "Failed to record Proxmox VE guest")
                }
            }
            if total > 0 {
                ctx.ops
                    .report_progress(30 + (60 * (i + 1) / total) as u8)
                    .await
                    .ok();
            }
        }

        tracing::info!(
            created,
            without_address = without_address.len(),
            "Proxmox VE guest sync complete"
        );
        if !without_address.is_empty() {
            // A stopped guest, or a VM without the guest agent, has no address the API can
            // report. Nothing is wrong, so this is a log line rather than a scan warning; the
            // guest appears once it runs with an address.
            tracing::info!(
                guests = %without_address.join(", "),
                "Proxmox VE guests with no reported address were not recorded"
            );
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

/// `pve/110 (gitlab)`, for log lines.
fn guest_label(guest: &GuestSummary) -> String {
    match &guest.name {
        Some(name) => format!("{}/{} ({name})", guest.node, guest.vmid),
        None => format!("{}/{}", guest.node, guest.vmid),
    }
}

/// The addresses a guest holds: read from inside it when it runs (the QEMU guest agent, or the
/// container's interfaces), kept only where they sit on one of its configured NICs; else the
/// static addresses its config sets (an LXC `ip=`, or a VM's cloud-init `ipconfigN`). See
/// [`mapping::select_addresses`].
async fn guest_addresses(client: &ProxmoxClient, guest: &GuestSummary) -> Vec<GuestAddress> {
    let path = guest.path();
    let nics = client
        .get_best_effort::<GuestConfig>(&format!("{path}/config"))
        .await
        .map(|config| mapping::config_nics(&config))
        .unwrap_or_default();

    let runtime = if guest.running {
        match guest.guest_type {
            ProxmoxGuestType::Qemu => client
                .get_best_effort::<AgentInterfaces>(&format!("{path}/agent/network-get-interfaces"))
                .await
                .map(|agent| mapping::agent_addresses(&agent))
                .unwrap_or_default(),
            ProxmoxGuestType::Lxc => client
                .get_best_effort::<Vec<LxcInterface>>(&format!("{path}/interfaces"))
                .await
                .map(|ifaces| mapping::lxc_addresses(&ifaces))
                .unwrap_or_default(),
        }
    } else {
        vec![]
    };

    mapping::select_addresses(&nics, runtime)
}

/// Record a node's host with its Proxmox VE service, returning that service's stored id.
///
/// `None` when the node has no known address, or the address sits in no subnet this network
/// holds (the matcher needs one to evaluate against).
async fn create_node_host(
    ctx: &IntegrationContext<'_>,
    subnets: &[Subnet],
    node: &NodeSpec,
    port: u16,
) -> Result<Option<Uuid>, Error> {
    let Some(ip) = node.ip else {
        return Ok(None);
    };
    let Some(subnet) = placeable_subnet(subnets, ip) else {
        tracing::warn!(node = %node.name, %ip, "Proxmox VE node address is in no known subnet");
        return Ok(None);
    };

    let network_id = ctx.ops.network_id().await?;
    let (host, ip_address) = mapping::node_host(node, ip, network_id);

    // The real matcher, fed the probe's answer: the node answered the API, which is what the
    // Proxmox VE definition's `ClientResponse` arm matches.
    let all_ports = vec![PortType::new_tcp(port)];
    let client_responses = HashMap::from([(ClientProbe::Proxmox, all_ports.clone())]);
    let daemon_id = ctx.ops.daemon_id().await?;
    let (services, ports) = ctx.ops.match_services(
        &host,
        &ServiceMatchBaselineParams {
            subnet,
            ip_address: &ip_address,
            all_ports: &all_ports,
            endpoint_responses: &vec![],
            virtualization_metadata: &None,
            virtualization_service_id: None,
            client_responses: &client_responses,
            managed_device: &None,
            dns_sd: &None,
        },
        &[],
        &daemon_id,
        &host.base.network_id,
    )?;

    let response = ctx
        .ops
        .create_host(
            host,
            vec![ip_address],
            ports,
            services,
            vec![],
            vec![],
            // The API never reports the node's interface table.
            false,
            InterfaceDataComplete::default(),
            ctx.cancel,
        )
        .await?;

    Ok(response
        .services
        .iter()
        .find(|s| is_proxmox_ve(s.base.service_definition.virtualization_role()))
        .map(|s| s.id))
}

fn is_proxmox_ve(role: Option<VirtualizationRole>) -> bool {
    role == Some(VirtualizationRole::Vms(
        HostVirtualizationDiscriminants::Proxmox,
    ))
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
        for node in &nodes {
            println!("node {node:?}");
        }
        for guest in &guests {
            let addresses = guest_addresses(&client, guest).await;
            println!(
                "guest {} {:?} {addresses:?}",
                guest_label(guest),
                guest.guest_type
            );
        }
        assert!(!nodes.is_empty(), "the lab has a node");
        assert!(!guests.is_empty(), "the lab has a guest");
    }
}
