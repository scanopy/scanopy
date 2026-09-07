//! The daemon host's own LLDP neighbours, read from a local lldpd.
//!
//! SNMP sees LLDP from the switches' side. A server running lldpd advertises itself and hears
//! its uplinks, but serves no SNMP agent, so its side of every server→switch edge went
//! unrecorded — the topology had the switch's port pointing at the server and nothing pointing
//! back. This module reads that view from lldpd, through `lldpcli -f json`, and hands it to
//! `run_daemon_host_interfaces_phase` to lay onto the per-NIC rows that phase already emits
//! (GH #689).
//!
//! What it does not do, deliberately. It creates no interface rows: a Linux host's `veth`, bond
//! and VLAN sub-interfaces are already listed by `own_nics_as_interfaces`, and only the rows
//! whose NIC actually heard a neighbour gain `lldp_*` columns. It does not report the host's own
//! chassis id either: since v0.17.13 every NIC is a row and `find_host_by_mac` matches any
//! interface MAC, so the switches' rem-table entries naming this host resolve without it.
//!
//! Gating is the socket, not configuration: `SCANOPY_LLDPD_SOCKET`, defaulting to lldpd's own
//! `/run/lldpd.socket`. No socket means silence — most daemon hosts run no lldpd, and that is
//! not a fault. A socket that refuses is worth a warning, because someone installed lldpd and
//! it is not answering. In a containerised daemon, bind-mount the host socket in and install
//! the `lldpd` package for the `lldpcli` binary. A daemon not running as root needs to be in
//! the group that owns the socket (`adm` on Debian and Ubuntu) — `lldpcli` is setuid to
//! lldpd's own user precisely so that group members can drive it.
//!
//! Removing lldpd later is not a way to clear the neighbours it reported: with the socket
//! gone this is a silent no-op and the stored `lldp_*` columns are preserved, exactly as they
//! are when an SNMP credential is removed. They age out through `neighbor_seen_at` instead.
//!
//! On shelling out: lldpd's control protocol is private and has no Rust binding, so `lldpcli`
//! is the only supported client. The invocation is kept narrow — a fixed argv with no shell,
//! a binary probe before spawning, a hard timeout, the child killed if the future is dropped,
//! and a cap on how much of its output is read.

use std::collections::BTreeMap;
use std::io::ErrorKind;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use serde_json::Value;
use thiserror::Error;
use tokio::io::AsyncReadExt;

use crate::daemon::discovery::service::warnings::AttemptOutcome;
use crate::server::interfaces::r#impl::base::Interface;
use crate::server::lldp::{LldpChassisId, LldpPortId, canonical_mac};

/// lldpd's compiled-in default control-socket path.
const DEFAULT_SOCKET: &str = "/run/lldpd.socket";

/// Environment variable overriding [`DEFAULT_SOCKET`].
const SOCKET_ENV: &str = "SCANOPY_LLDPD_SOCKET";

/// How long one `lldpcli` invocation may take, probe included. Local socket, local daemon: an
/// answer that takes longer than this is an answer that is not coming.
const LLDPCLI_TIMEOUT: Duration = Duration::from_secs(5);

/// Most stdout we will read from `lldpcli`. A neighbour entry is a few hundred bytes and a
/// host has a handful of uplinks, so this is two orders of magnitude of headroom; past it the
/// output is not a neighbour table.
const STDOUT_CAP: usize = 4 * 1024 * 1024;

/// Most stderr we keep from `lldpcli`. Its diagnostics are one line — "cannot connect to
/// /run/lldpd.socket" is the one that matters — and that line is carried into the warning.
const STDERR_CAP: usize = 4 * 1024;

/// Why a read from lldpd produced nothing.
///
/// Each variant is logged differently, which is the point of keeping them apart: a missing
/// binary is a debug line (lldpd is present but the CLI package is not — common in a container),
/// while a socket that refuses, a hung `lldpcli` or non-JSON output are warnings, since each
/// means someone set lldpd up and it is not working. [`Self::outcome`] maps them onto the same
/// [`AttemptOutcome`] vocabulary the credentialed integrations classify their failures with.
#[derive(Debug, Error)]
pub(super) enum LldpdError {
    #[error("no lldpcli binary on PATH")]
    CliMissing,
    #[error("lldpd socket {socket} refused the connection: {source}")]
    SocketRefused {
        socket: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("lldpcli could not be started: {0}")]
    Spawn(#[source] std::io::Error),
    #[error("lldpcli produced no answer within {:?}", LLDPCLI_TIMEOUT)]
    TimedOut,
    #[error("lldpcli exited with {status}: {stderr}")]
    Failed {
        status: std::process::ExitStatus,
        /// What `lldpcli` said on the way out, trimmed. Its one-line diagnostics name the
        /// cause — a socket it cannot connect to, a protocol version lldpd rejects — and
        /// dropping them leaves the operator with an exit code.
        stderr: String,
    },
    #[error("lldpcli output exceeded {STDOUT_CAP} bytes")]
    OutputTooLarge,
    #[error("lldpcli output was not JSON: {0}")]
    NotJson(#[source] serde_json::Error),
    #[error("lldpcli output was JSON of a shape this parser does not recognise: {0}")]
    UnrecognisedShape(&'static str),
}

impl LldpdError {
    pub(super) fn outcome(&self) -> AttemptOutcome {
        match self {
            Self::SocketRefused { .. } | Self::Spawn(_) => AttemptOutcome::Unreachable,
            Self::TimedOut => AttemptOutcome::TimedOut,
            // The socket is there and this host is set up wrong for it — the one outcome the
            // operator fixes on the daemon host rather than on the network.
            Self::CliMissing => AttemptOutcome::Malformed,
            // lldpd was reached and the read still failed: for `Failed` its stderr says
            // why; for `UnrecognisedShape` it answered parseable JSON whose shape has
            // drifted — it *is* the service, and the collection after the answer is what
            // broke.
            Self::Failed { .. } | Self::UnrecognisedShape(_) => AttemptOutcome::CollectionFailed,
            // Something on the socket path that is not an lldpd at all: bytes that are not
            // JSON, or megabytes where a neighbour table belongs.
            Self::OutputTooLarge | Self::NotJson(_) => AttemptOutcome::NotThisService,
        }
    }
}

/// One neighbour as lldpd sees it: the local port it was heard on, and the remote identity.
#[derive(Debug)]
pub(super) struct LldpdNeighbor {
    /// Local interface name the neighbour was received on (e.g. `eno1`).
    pub local_interface: String,
    pub chassis_id: Option<LldpChassisId>,
    pub port_id: Option<LldpPortId>,
    /// Remote system name — the key of lldpcli's `chassis` object.
    pub sys_name: Option<String>,
    /// Remote system description (`chassis.<name>.descr`).
    pub sys_desc: Option<String>,
    /// Remote port description, when non-blank.
    pub port_desc: Option<String>,
    /// First management address the neighbour advertised.
    pub mgmt_addr: Option<IpAddr>,
}

/// Map an lldpcli id `{type, value}` pair onto [`LldpChassisId`].
///
/// lldpcli renders the IEEE subtype as a short string (lldpd's `map_chassis_id` names);
/// values arrive already decoded, so MAC-typed ids go through the same canonicalisation the
/// SNMP path applies to raw octets. Unknown types fall back to
/// [`LldpChassisId::from_identifier_str`], the constructor that assumes least.
fn map_chassis_id(id_type: &str, value: &str) -> Option<LldpChassisId> {
    match id_type {
        "mac" => canonical_mac(value).map(LldpChassisId::MacAddress),
        "ifname" => Some(LldpChassisId::InterfaceName(value.to_string())),
        "ifalias" => Some(LldpChassisId::InterfaceAlias(value.to_string())),
        "ip" => value.parse().ok().map(LldpChassisId::NetworkAddress),
        "local" => Some(LldpChassisId::LocallyAssigned(value.to_string())),
        "chassis" => Some(LldpChassisId::ChassisComponent(value.to_string())),
        "port" => Some(LldpChassisId::PortComponent(value.to_string())),
        _ => Some(LldpChassisId::from_identifier_str(value)),
    }
}

/// Map an lldpcli id `{type, value}` pair onto [`LldpPortId`]. Same rationale as
/// [`map_chassis_id`].
fn map_port_id(id_type: &str, value: &str) -> Option<LldpPortId> {
    match id_type {
        "mac" => canonical_mac(value).map(LldpPortId::MacAddress),
        "ifname" => Some(LldpPortId::InterfaceName(value.to_string())),
        "ifalias" => Some(LldpPortId::InterfaceAlias(value.to_string())),
        "ip" => value.parse().ok().map(LldpPortId::NetworkAddress),
        "local" => Some(LldpPortId::LocallyAssigned(value.to_string())),
        "agentid" => Some(LldpPortId::AgentCircuitId(value.to_string())),
        _ => Some(LldpPortId::from_identifier_str(value)),
    }
}

/// lldpcli emits a JSON object for one entry and an array for several; every consumer of its
/// output re-learns this. Normalise to a list of objects.
fn as_entries(v: &Value) -> Vec<&serde_json::Map<String, Value>> {
    match v {
        Value::Array(items) => items.iter().filter_map(|i| i.as_object()).collect(),
        Value::Object(_) => v.as_object().into_iter().collect(),
        _ => Vec::new(),
    }
}

fn id_pair(obj: &Value) -> Option<(&str, &str)> {
    let id = obj.get("id")?;
    Some((id.get("type")?.as_str()?, id.get("value")?.as_str()?))
}

/// A string that carries information, or nothing. lldpd pads absent descriptions to `" "`.
fn non_blank(v: Option<&Value>) -> Option<String> {
    let s = v?.as_str()?.trim();
    (!s.is_empty()).then(|| s.to_string())
}

/// Parse `lldpcli -f json show neighbors details` output.
///
/// A shape this parser does not recognise is an error, never an empty table. The distinction
/// carries all the way to the database: an empty table is authoritative and clears the stored
/// neighbours, so an lldpd release that renamed or restructured a section — while still
/// emitting perfectly valid JSON — would otherwise erase good data on every scan. The two
/// genuine zero-neighbour shapes stay authoritative: an empty `lldp` object, and an `interface`
/// key with nothing under it.
pub(super) fn parse_neighbors(json: &Value) -> Result<Vec<LldpdNeighbor>, LldpdError> {
    let Some(lldp) = json.get("lldp").and_then(Value::as_object) else {
        return Err(LldpdError::UnrecognisedShape(
            "no `lldp` object at the top level",
        ));
    };
    let Some(interfaces) = lldp.get("interface") else {
        // `{"lldp": {}}` is what lldpcli emits for a host with no neighbours at all. Any
        // *other* key in its place means the section moved, not that it is empty.
        return if lldp.is_empty() {
            Ok(Vec::new())
        } else {
            Err(LldpdError::UnrecognisedShape(
                "`lldp` object carries no `interface` section",
            ))
        };
    };
    if !matches!(interfaces, Value::Array(_) | Value::Object(_)) {
        return Err(LldpdError::UnrecognisedShape(
            "`interface` is neither an object nor an array",
        ));
    }
    let mut out = Vec::new();
    for entry in as_entries(interfaces) {
        for (local_interface, body) in entry {
            // `chassis` is keyed by the remote system name.
            let remote = body
                .get("chassis")
                .and_then(|c| c.as_object())
                .and_then(|c| c.iter().next());
            let (sys_name, chassis_body) = match remote {
                Some((name, chassis_body)) => (Some(name.clone()), Some(chassis_body)),
                None => (None, None),
            };
            let chassis_id = chassis_body
                .and_then(id_pair)
                .and_then(|(t, v)| map_chassis_id(t, v));
            let sys_desc = chassis_body.and_then(|c| non_blank(c.get("descr")));
            // `mgmt-ip` is a string for one address and an array for several.
            let mgmt_addr = chassis_body
                .and_then(|c| c.get("mgmt-ip"))
                .and_then(|m| match m {
                    Value::String(s) => s.parse().ok(),
                    Value::Array(items) => items
                        .iter()
                        .filter_map(|i| i.as_str())
                        .find_map(|s| s.parse().ok()),
                    _ => None,
                });
            let port = body.get("port");
            let port_id = port.and_then(id_pair).and_then(|(t, v)| map_port_id(t, v));
            let port_desc = port.and_then(|p| non_blank(p.get("descr")));
            out.push(LldpdNeighbor {
                local_interface: local_interface.clone(),
                chassis_id,
                port_id,
                sys_name,
                sys_desc,
                port_desc,
                mgmt_addr,
            });
        }
    }
    Ok(out)
}

/// Lay lldpd's neighbours onto the daemon host's own NIC rows.
///
/// Rows are matched by interface name — lldpd reports the kernel name, which is exactly what
/// `nic_to_interface` puts in `if_name`. A neighbour heard on a NIC that is not in `interfaces`
/// (filtered out by `--interfaces`, or a container bridge) is dropped with a debug line; nothing
/// here may add a row. A row that heard no neighbour is left untouched.
///
/// A port can hear several neighbours (a hub, an unmanaged switch, a hypervisor bridge), and
/// lldpcli lists each as its own entry under the same interface name. A row holds one, so the
/// first wins — lldpd orders its table deterministically — and the rest are counted into a
/// debug line rather than silently overwriting each other.
pub(super) fn apply_neighbors(interfaces: &mut [Interface], neighbors: Vec<LldpdNeighbor>) {
    let mut extra: BTreeMap<String, usize> = BTreeMap::new();
    for n in neighbors {
        let Some(row) = interfaces
            .iter_mut()
            .find(|i| i.base.if_name.as_deref() == Some(n.local_interface.as_str()))
        else {
            tracing::debug!(
                interface = %n.local_interface,
                "lldpd neighbour on a NIC the daemon does not report; ignoring"
            );
            continue;
        };
        if row.base.lldp_chassis_id.is_some() || row.base.lldp_port_id.is_some() {
            *extra.entry(n.local_interface).or_default() += 1;
            continue;
        }
        row.base.lldp_chassis_id = n.chassis_id;
        row.base.lldp_port_id = n.port_id;
        row.base.lldp_sys_name = n.sys_name;
        row.base.lldp_port_desc = n.port_desc;
        row.base.lldp_mgmt_addr = n.mgmt_addr;
        row.base.lldp_sys_desc = n.sys_desc;
    }
    for (interface, dropped) in extra {
        tracing::debug!(
            interface = %interface,
            neighbours = dropped + 1,
            "several LLDP neighbours on one port; keeping the first"
        );
    }
}

/// The lldpd control socket to use, if one exists on this host.
///
/// Existence alone gates the whole read: `None` here is the everyday case and is logged only
/// at debug. Whether the socket *answers* is a separate question, settled by [`read_neighbors`].
pub(super) fn socket_path() -> Option<PathBuf> {
    let socket = std::env::var_os(SOCKET_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SOCKET));
    if socket.exists() {
        Some(socket)
    } else {
        tracing::debug!(socket = %socket.display(), "No lldpd socket; not reading LLDP neighbours");
        None
    }
}

/// `lldpcli` on `PATH`, resolved before spawning so a missing binary is its own error rather
/// than an opaque spawn failure — and so the spawn can never be handed a relative name a
/// `PATH` change could redirect.
fn find_lldpcli() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join("lldpcli"))
        .find(|candidate| candidate.is_file())
}

/// Is anything listening on `socket`?
///
/// Connecting from this process is what separates "lldpd is not running" from every other
/// way `lldpcli` can fail: the CLI's own error text for a dead socket is a free-form line on
/// stderr, and matching on it is exactly the fragility the typed error exists to avoid.
///
/// Permission denied is *not* a refusal. lldpd's socket is owned by its own user and group,
/// and `lldpcli` is setuid to that user so members of the group can drive it without being
/// able to open the socket themselves. A daemon in that position gets EACCES here and a
/// working `lldpcli` a moment later, so the probe stands aside and lets the CLI decide.
pub(super) async fn probe_socket(socket: &Path) -> Result<(), LldpdError> {
    match tokio::net::UnixStream::connect(socket).await {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == ErrorKind::PermissionDenied => Ok(()),
        Err(source) => Err(LldpdError::SocketRefused {
            socket: socket.to_path_buf(),
            source,
        }),
    }
}

/// Read up to `cap` bytes from `reader`, reporting whether there was more.
async fn read_capped<R: tokio::io::AsyncRead + Unpin>(
    reader: R,
    cap: usize,
) -> std::io::Result<(Vec<u8>, bool)> {
    let mut buf = Vec::new();
    // One byte past the cap tells over-size from exactly-at-cap.
    reader.take(cap as u64 + 1).read_to_end(&mut buf).await?;
    let overflowed = buf.len() > cap;
    buf.truncate(cap);
    Ok((buf, overflowed))
}

/// Run `lldpcli` against `socket` and return its neighbour table.
pub(super) async fn read_neighbors(socket: &Path) -> Result<Vec<LldpdNeighbor>, LldpdError> {
    let lldpcli = find_lldpcli().ok_or(LldpdError::CliMissing)?;

    let run = async {
        probe_socket(socket).await?;

        // The one process anything under `daemon/discovery/` spawns, and it must stay the
        // only one. It exists because lldpd's control protocol is private and unversioned —
        // there is no maintained Rust client, and `lldpcli -f json` is the interface the
        // lldpd project treats as stable — not because shelling out is an acceptable pattern
        // for reading anything that has a real client library.
        let mut child = tokio::process::Command::new(&lldpcli)
            .arg("-u")
            .arg(socket)
            .args(["-f", "json", "show", "neighbors", "details"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            // If the timeout fires, or discovery is cancelled and this future dropped, the
            // child goes with it rather than lingering on a wedged socket.
            .kill_on_drop(true)
            .spawn()
            .map_err(LldpdError::Spawn)?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| LldpdError::Spawn(ErrorKind::BrokenPipe.into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| LldpdError::Spawn(ErrorKind::BrokenPipe.into()))?;
        // Both pipes drained together: draining one and then the other lets the child block
        // on the full one before it has finished writing the other.
        let (stdout, stderr) = tokio::join!(
            read_capped(stdout, STDOUT_CAP),
            read_capped(stderr, STDERR_CAP)
        );
        let (stdout, overflowed) = stdout.map_err(LldpdError::Spawn)?;
        let (stderr, _) = stderr.map_err(LldpdError::Spawn)?;
        if overflowed {
            return Err(LldpdError::OutputTooLarge);
        }

        let status = child.wait().await.map_err(LldpdError::Spawn)?;
        if !status.success() {
            return Err(LldpdError::Failed {
                status,
                stderr: String::from_utf8_lossy(&stderr).trim().to_string(),
            });
        }

        serde_json::from_slice::<Value>(&stdout)
            .map_err(LldpdError::NotJson)
            .and_then(|json| parse_neighbors(&json))
    };

    tokio::time::timeout(LLDPCLI_TIMEOUT, run)
        .await
        .unwrap_or(Err(LldpdError::TimedOut))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::interfaces::r#impl::base::{IfAdminStatus, IfOperStatus, InterfaceBase};
    use uuid::Uuid;

    /// Trimmed from a real `lldpcli -f json show neighbors details`: captured 2026-08-25
    /// via lldpcli 1.0.18 (Ubuntu noble's package, in our pilot container) reading a host
    /// lldpd 1.0.19, and re-verified against the same pair 2026-08-26. Exercises an
    /// ifname-typed port, a local-typed port with a port description, and a multi-address
    /// chassis. The daemon image's Debian bookworm package is lldpd 1.0.16, and the
    /// scheduled `lldpd-live` workflow tracks that one — the version we actually ship.
    const NEIGHBORS: &str = r#"{
      "lldp": {
        "interface": [
          { "eno1": {
              "via": "LLDP",
              "chassis": { "netlab-leaf2": {
                "id": { "type": "mac", "value": "C0:C9:89:EF:20:D2" },
                "descr": "Arrcus Operating System (ArcOS)",
                "mgmt-ip": "10.22.64.105" } },
              "port": { "id": { "type": "ifname", "value": "swp1" }, "descr": " " } } },
          { "eno3": {
              "via": "LLDP",
              "chassis": { "netlab-mgmt0": {
                "id": { "type": "mac", "value": "dc:da:4d:86:f4:ea" },
                "descr": "SONiC Software Version: 4.0.0",
                "mgmt-ip": ["10.22.64.82", "fe80::1"] } },
              "port": { "id": { "type": "local", "value": "Ethernet2" },
                        "descr": "netlab-server : OS" } } }
        ]
      }
    }"#;

    fn nic_row(name: &str) -> Interface {
        Interface::new(InterfaceBase {
            host_id: Uuid::new_v4(),
            network_id: Uuid::new_v4(),
            if_index: 1,
            if_descr: name.to_string(),
            if_name: Some(name.to_string()),
            if_alias: None,
            if_type: 53,
            speed_bps: None,
            admin_status: IfAdminStatus::Up,
            oper_status: IfOperStatus::Up,
            mac_address: None,
            ip_address_id: None,
            neighbor: None,
            neighbor_seen_at: None,
            lldp_chassis_id: None,
            lldp_port_id: None,
            lldp_sys_name: None,
            lldp_port_desc: None,
            lldp_mgmt_addr: None,
            lldp_sys_desc: None,
            cdp_device_id: None,
            cdp_port_id: None,
            cdp_platform: None,
            cdp_address: None,
            fdb_macs: None,
            native_vlan_id: None,
            vlan_ids: None,
        })
    }

    #[test]
    fn parses_neighbors_with_both_port_id_shapes() {
        let neighbors = parse_neighbors(&serde_json::from_str(NEIGHBORS).unwrap()).unwrap();
        assert_eq!(neighbors.len(), 2);

        let leaf2 = &neighbors[0];
        assert_eq!(leaf2.local_interface, "eno1");
        assert_eq!(
            leaf2.chassis_id,
            // Uppercase input canonicalises to the same lowercase form the SNMP path stores.
            Some(LldpChassisId::MacAddress("c0:c9:89:ef:20:d2".into()))
        );
        assert_eq!(
            leaf2.port_id,
            Some(LldpPortId::InterfaceName("swp1".into()))
        );
        assert_eq!(leaf2.sys_name.as_deref(), Some("netlab-leaf2"));
        assert_eq!(leaf2.mgmt_addr, Some("10.22.64.105".parse().unwrap()));
        assert_eq!(leaf2.port_desc, None, "a blank description carries nothing");

        let mgmt0 = &neighbors[1];
        assert_eq!(
            mgmt0.port_id,
            Some(LldpPortId::LocallyAssigned("Ethernet2".into()))
        );
        assert_eq!(mgmt0.mgmt_addr, Some("10.22.64.82".parse().unwrap()));
        assert_eq!(mgmt0.port_desc.as_deref(), Some("netlab-server : OS"));
    }

    #[test]
    fn single_neighbor_object_parses() {
        // lldpcli emits an object (not a one-element array) for a single interface.
        let single = r#"{"lldp":{"interface":{"eth0":{
            "chassis":{"sw":{"id":{"type":"mac","value":"aa:bb:cc:dd:ee:ff"}}},
            "port":{"id":{"type":"ifname","value":"ge-0/0/1"}}}}}}"#;
        let neighbors = parse_neighbors(&serde_json::from_str(single).unwrap()).unwrap();
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].local_interface, "eth0");
    }

    /// The genuine zero-neighbour shapes are authoritative: they parse to an empty table,
    /// which clears stored neighbours. Only these shapes may do that.
    #[test]
    fn genuinely_empty_shapes_are_an_empty_table() {
        for empty in [
            r#"{"lldp":{}}"#,
            r#"{"lldp":{"interface":[]}}"#,
            r#"{"lldp":{"interface":{}}}"#,
        ] {
            let parsed = parse_neighbors(&serde_json::from_str::<Value>(empty).unwrap());
            assert!(matches!(parsed, Ok(ref v) if v.is_empty()), "{empty}");
        }
    }

    /// Valid JSON in a shape this parser does not recognise must be an error — the branch
    /// that preserves the server's stored neighbours — never an empty table, the branch
    /// that clears them. A future lldpd that renames a section emits exactly this.
    #[test]
    fn reshaped_but_valid_json_is_an_error_not_an_empty_table() {
        for reshaped in [
            // The top-level section renamed.
            r#"{"lldp-neighbors":{"interface":[]}}"#,
            // The `interface` key renamed, its content intact.
            r#"{"lldp":{"interfaces":{"eth0":{
                "chassis":{"sw":{"id":{"type":"mac","value":"aa:bb:cc:dd:ee:ff"}}}}}}}"#,
            // The section demoted to something that is not a collection.
            r#"{"lldp":{"interface":"eth0"}}"#,
            // `lldp` itself no longer an object.
            r#"{"lldp":[]}"#,
        ] {
            let parsed = parse_neighbors(&serde_json::from_str::<Value>(reshaped).unwrap());
            let err = parsed.expect_err(reshaped);
            assert!(matches!(err, LldpdError::UnrecognisedShape(_)), "{err}");
        }
    }

    /// The contract with `run_daemon_host_interfaces_phase`: neighbours land on the rows it
    /// already has, by name; a neighbour on an unlisted NIC adds nothing; an unneighboured row
    /// is not touched.
    #[test]
    fn neighbors_decorate_existing_rows_only() {
        let neighbors = parse_neighbors(&serde_json::from_str(NEIGHBORS).unwrap()).unwrap();
        let mut rows = vec![nic_row("lo"), nic_row("eno1"), nic_row("veth0")];
        apply_neighbors(&mut rows, neighbors);

        assert_eq!(rows.len(), 3, "no row is created for eno3");
        assert!(rows[0].base.lldp_chassis_id.is_none());
        assert!(rows[2].base.lldp_chassis_id.is_none());

        let eno1 = &rows[1].base;
        assert_eq!(
            eno1.lldp_chassis_id,
            Some(LldpChassisId::MacAddress("c0:c9:89:ef:20:d2".into()))
        );
        assert_eq!(
            eno1.lldp_port_id,
            Some(LldpPortId::InterfaceName("swp1".into()))
        );
        assert_eq!(eno1.lldp_sys_name.as_deref(), Some("netlab-leaf2"));
        assert_eq!(
            eno1.lldp_sys_desc.as_deref(),
            Some("Arrcus Operating System (ArcOS)")
        );
        assert_eq!(eno1.lldp_mgmt_addr, Some("10.22.64.105".parse().unwrap()));
    }

    /// Two neighbours heard on the same port: the first is kept, the second changes nothing.
    #[test]
    fn several_neighbours_on_one_port_keep_the_first() {
        let two = r#"{"lldp":{"interface":[
            {"eth0":{"chassis":{"a":{"id":{"type":"mac","value":"aa:aa:aa:aa:aa:aa"}}},
                     "port":{"id":{"type":"ifname","value":"1"}}}},
            {"eth0":{"chassis":{"b":{"id":{"type":"mac","value":"bb:bb:bb:bb:bb:bb"}}},
                     "port":{"id":{"type":"ifname","value":"2"}}}}]}}"#;
        let neighbors = parse_neighbors(&serde_json::from_str(two).unwrap()).unwrap();
        assert_eq!(neighbors.len(), 2);
        let mut rows = vec![nic_row("eth0")];
        apply_neighbors(&mut rows, neighbors);
        assert_eq!(rows[0].base.lldp_sys_name.as_deref(), Some("a"));
        assert_eq!(
            rows[0].base.lldp_port_id,
            Some(LldpPortId::InterfaceName("1".into()))
        );
    }

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let dir = std::env::temp_dir().join(format!("scanopy-lldpd-{}", Uuid::new_v4()));
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A socket path that exists but nothing listens on is the "installed but not running"
    /// case, and must come back as refused — not as a timeout or a CLI failure.
    #[tokio::test]
    async fn a_socket_nobody_listens_on_is_refused() {
        let dir = TempDir::new();
        let socket = dir.0.join("lldpd.socket");
        std::fs::write(&socket, b"").unwrap();

        let err = probe_socket(&socket).await.unwrap_err();
        assert!(matches!(err, LldpdError::SocketRefused { .. }), "{err}");
        assert_eq!(err.outcome(), AttemptOutcome::Unreachable);
    }

    /// The path vanishing between `socket_path` and the probe (lldpd stopped mid-scan) is the
    /// same verdict.
    #[tokio::test]
    async fn a_socket_that_has_gone_is_refused() {
        let dir = TempDir::new();
        let err = probe_socket(&dir.0.join("gone.socket")).await.unwrap_err();
        assert!(matches!(err, LldpdError::SocketRefused { .. }), "{err}");
    }

    #[tokio::test]
    async fn a_listening_socket_passes_the_probe() {
        let dir = TempDir::new();
        let socket = dir.0.join("lldpd.socket");
        let _listener = tokio::net::UnixListener::bind(&socket).unwrap();
        probe_socket(&socket).await.unwrap();
    }

    // ------------------------------------------------------------------------------------
    // Live-capture checks, driven by .github/workflows/lldpd-live.yml.
    //
    // The fixtures above prove the parser handles a capture; they cannot prove the capture
    // still matches what lldpcli emits. The scheduled workflow takes fresh captures from a
    // real lldpd (two instances exchanging LLDP over a veth pair inside the daemon image)
    // and feeds them here, so a shape change in a new Debian lldpd package fails on our
    // types rather than going unnoticed until it clears somebody's neighbours.
    // ------------------------------------------------------------------------------------

    fn capture(var: &str) -> Value {
        let path = std::env::var(var)
            .unwrap_or_else(|_| panic!("{var} must name a capture file; see lldpd-live.yml"));
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("reading capture {path}: {e}"));
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("capture {path} is not JSON: {e}"))
    }

    /// A populated table from a live lldpd parses, and its rows carry the three fields the
    /// server's L2 resolution and display actually consume.
    #[test]
    #[ignore = "needs captures from a live lldpd; run via .github/workflows/lldpd-live.yml"]
    fn live_capture_parses_to_a_usable_neighbour() {
        let neighbors = parse_neighbors(&capture("SCANOPY_LLDPD_NEIGHBORS_CAPTURE"))
            .unwrap_or_else(|e| panic!("live lldpcli output no longer parses: {e}"));
        assert!(!neighbors.is_empty(), "live capture held no neighbours");
        for n in &neighbors {
            assert!(
                n.chassis_id.is_some(),
                "{}: no chassis id",
                n.local_interface
            );
            assert!(n.port_id.is_some(), "{}: no port id", n.local_interface);
            assert!(
                n.sys_name.is_some(),
                "{}: no system name",
                n.local_interface
            );
        }
    }

    /// What a live lldpd emits *before* hearing anything must still read as the
    /// authoritative empty table — if this shape drifts, zero-neighbour hosts would start
    /// landing in the read-failure branch and stale neighbours would never clear.
    #[test]
    #[ignore = "needs captures from a live lldpd; run via .github/workflows/lldpd-live.yml"]
    fn live_empty_capture_is_an_authoritative_empty_table() {
        let parsed = parse_neighbors(&capture("SCANOPY_LLDPD_EMPTY_CAPTURE"));
        assert!(
            matches!(parsed, Ok(ref v) if v.is_empty()),
            "zero-neighbour shape no longer reads as empty: {parsed:?}"
        );
    }
}
