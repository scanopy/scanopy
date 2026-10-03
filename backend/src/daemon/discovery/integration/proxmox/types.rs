//! Raw Proxmox VE API JSON shapes.
//!
//! Free of Scanopy entity types: these mirror what `/api2/json` puts on the wire, and the
//! translation into hosts lives in [`super::mapping`]. Field names and meanings follow the API
//! viewer (pve.proxmox.com/pve-docs/api-viewer). Every field the mapping can live without is
//! optional, and nothing denies unknown fields, so a newer release adding keys changes nothing.
//!
//! Proxmox encodes booleans as `0`/`1`, which [`FlexBool`] reads alongside real booleans.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::daemon::discovery::integration::flex::{FlexBool, FlexInt};

/// Every response is `{"data": ...}`.
#[derive(Debug, Deserialize)]
pub struct PveEnvelope<T> {
    pub data: T,
}

/// `GET /version`.
#[derive(Debug, Clone, Deserialize)]
pub struct PveVersion {
    /// Full version, e.g. `8.4.21`.
    pub version: String,
    /// Major.minor, e.g. `8.4`.
    #[serde(default)]
    pub release: Option<String>,
}

/// One entry of `GET /cluster/status`: the cluster itself (only when the node is clustered) and
/// one per node. Needs `Sys.Audit` on `/`.
#[derive(Debug, Clone, Deserialize)]
pub struct ClusterStatusEntry {
    /// `cluster` or `node`.
    #[serde(rename = "type")]
    pub entry_type: String,
    pub name: String,
    /// The node's cluster (corosync) address. Present on `node` entries.
    #[serde(default)]
    pub ip: Option<String>,
    /// Whether this is the node that answered the request.
    #[serde(default)]
    pub local: Option<FlexBool>,
    #[serde(default)]
    pub online: Option<FlexBool>,
}

/// One entry of `GET /cluster/resources`. Every node, guest and storage the token may audit,
/// across the whole cluster.
#[derive(Debug, Clone, Deserialize)]
pub struct ClusterResource {
    /// `node`, `qemu`, `lxc`, `storage`, `sdn`, `pool`, ...
    #[serde(rename = "type")]
    pub resource_type: String,
    /// The node this resource lives on (the node's own name, for a `node` entry).
    #[serde(default)]
    pub node: Option<String>,
    #[serde(default)]
    pub vmid: Option<FlexInt>,
    /// Guest name.
    #[serde(default)]
    pub name: Option<String>,
    /// `running`, `stopped`, ... for guests; `online`, `offline` for nodes.
    #[serde(default)]
    pub status: Option<String>,
    /// Set on VM templates, which never run and have no addresses.
    #[serde(default)]
    pub template: Option<FlexBool>,
}

/// `GET /nodes/{node}/qemu/{vmid}/config` and `.../lxc/{vmid}/config`: a flat key→value map.
/// Only the `netN` keys are read; their values are comma-separated `key=value` strings.
pub type GuestConfig = BTreeMap<String, serde_json::Value>;

/// `GET /nodes/{node}/qemu/{vmid}/agent/network-get-interfaces`. The guest agent's answer is
/// wrapped once more in `result`.
#[derive(Debug, Clone, Deserialize)]
pub struct AgentInterfaces {
    #[serde(default)]
    pub result: Vec<AgentInterface>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentInterface {
    pub name: String,
    #[serde(rename = "hardware-address", default)]
    pub hardware_address: Option<String>,
    #[serde(rename = "ip-addresses", default)]
    pub ip_addresses: Vec<AgentIpAddress>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentIpAddress {
    #[serde(rename = "ip-address")]
    pub ip_address: String,
}

/// One entry of `GET /nodes/{node}/lxc/{vmid}/interfaces`, read from the running container.
#[derive(Debug, Clone, Deserialize)]
pub struct LxcInterface {
    pub name: String,
    #[serde(default)]
    pub hwaddr: Option<String>,
    /// IPv4 in CIDR form, e.g. `192.168.4.50/24`.
    #[serde(default)]
    pub inet: Option<String>,
    /// IPv6 in CIDR form.
    #[serde(default)]
    pub inet6: Option<String>,
}
