//! Pushes a generated estate into a running server as one daemon's discovery would, to measure
//! what the server needs at a given size (`docs/sizing-results.md`).
//!
//! It provisions a daemon on the site with a user API key, then runs each pass as a DaemonPoll
//! daemon runs a discovery session: subnets, VLANs, one host submission per host at the daemon's
//! own pacing, then the terminal update that triggers link resolution. Switch links travel as the
//! LLDP evidence each switch would report, so the server resolves them itself. A second pass
//! submits the same hosts again, which is what a scheduled rescan does.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use chrono::Utc;
use clap::{Parser, ValueEnum};
use scanopy::daemon::discovery::types::base::{DiscoveryPhase, DiscoveryTerminalReason};
use scanopy::server::daemons::r#impl::api::{
    DaemonDiscoveryRequest, DaemonWorkResponse, DiscoveryUpdatePayload, ScannedEntityIds,
};
use scanopy::server::hosts::r#impl::api::DiscoveryHostRequest;
use scanopy::server::interface_neighbors::r#impl::base::InterfaceNeighborEvidence;
use scanopy::server::interfaces::r#impl::base::InterfaceDataComplete;
use scanopy::server::lldp::{LldpChassisId, LldpPortId};
use scanopy::server::organizations::demo_data::large_estate::{
    EstateParams, LargeEstate, generate_large_estate, plant_site,
};
use scanopy::server::services::r#impl::patterns::ClientProbe;
use scanopy::server::shared::attribution::AttributeSource;
use serde_json::{Value, json};
use uuid::Uuid;

/// The daemon spaces host submissions at least this far apart.
const HOST_SUBMIT_INTERVAL: Duration = Duration::from_millis(25);
/// How often a running daemon polls for work, which also keeps its session alive.
const POLL_INTERVAL: Duration = Duration::from_secs(5);
/// Hosts between progress updates.
const PROGRESS_EVERY: usize = 250;

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Size {
    Small,
    Medium,
    Large,
}

#[derive(Parser, Debug)]
#[command(about = "Replay a generated estate into a running server through the daemon ingest path")]
struct Args {
    /// The server's base URL, e.g. http://localhost:60172.
    #[arg(long)]
    server_url: String,
    /// A user API key (scp_u_…) for the organization that owns the site.
    #[arg(long, required_unless_present = "session_id")]
    user_api_key: Option<String>,
    /// A signed-in user's `session_id` cookie, in place of an API key, for an account that
    /// can't create one (the demo user, whose email is unverified).
    #[arg(long, conflicts_with = "user_api_key")]
    session_id: Option<String>,
    /// The site to load the estate into.
    #[arg(long)]
    site_id: Uuid,
    #[arg(long, value_enum, default_value = "medium")]
    size: Size,
    /// Passes over the same estate. The first creates every host; later passes update them.
    #[arg(long, default_value_t = 2)]
    passes: u32,
}

struct Daemon {
    client: reqwest::Client,
    base: String,
    id: Uuid,
    key: String,
    site_id: Uuid,
    version: String,
}

impl Daemon {
    async fn post(&self, path: &str, body: &impl serde::Serialize) -> Result<Value> {
        let response = self
            .client
            .post(format!("{}{path}", self.base))
            .bearer_auth(&self.key)
            .header("X-Daemon-ID", self.id.to_string())
            .header("X-Daemon-Version", &self.version)
            .json(body)
            .send()
            .await
            .with_context(|| format!("POST {path}"))?;
        unwrap_response(path, response).await
    }

    fn status(&self, ready_for_work: bool) -> Value {
        json!({
            "name": "estate-replay",
            "mode": "daemon_poll",
            "version": self.version,
            "capabilities": { "interfaced_subnet_ids": [] },
            "ready_for_work": ready_for_work,
            "os": "linux",
        })
    }

    async fn poll(&self, ready_for_work: bool) -> Result<Option<DaemonDiscoveryRequest>> {
        let data = self
            .post(
                &format!("/api/daemons/{}/request-work", self.id),
                &self.status(ready_for_work),
            )
            .await?;
        let work: DaemonWorkResponse<DaemonDiscoveryRequest> = serde_json::from_value(data)?;
        Ok(work.next_session)
    }

    async fn wait_for_session(&self) -> Result<DaemonDiscoveryRequest> {
        for _ in 0..120 {
            if let Some(session) = self.poll(true).await? {
                return Ok(session);
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        bail!("no discovery session reached the daemon within two minutes")
    }

    async fn update(
        &self,
        session: &DaemonDiscoveryRequest,
        phase: DiscoveryPhase,
        progress: u8,
        started_at: chrono::DateTime<Utc>,
        scanned: Option<ScannedEntityIds>,
    ) -> Result<()> {
        let terminal = phase == DiscoveryPhase::Complete;
        let now = Utc::now();
        let payload = DiscoveryUpdatePayload {
            session_id: session.session_id,
            daemon_id: self.id,
            site_id: self.site_id,
            phase,
            discovery_type: session.discovery_type.clone(),
            progress,
            error: None,
            warnings: Vec::new(),
            credential_results: Vec::new(),
            started_at: Some(started_at),
            finished_at: terminal.then_some(now),
            hosts_discovered: None,
            estimated_remaining_secs: None,
            discovery_id: None,
            scanned,
            reason: terminal.then_some(DiscoveryTerminalReason::Completed),
            last_update_at: Some(now),
            daemon_version: Some(self.version.clone()),
            last_activity_at: Some(now),
        };
        if terminal {
            println!(
                "terminal update {} bytes",
                serde_json::to_vec(&payload)?.len()
            );
        }
        self.post(
            &format!("/api/v1/discoveries/{}/update", session.session_id),
            &payload,
        )
        .await?;
        Ok(())
    }
}

async fn unwrap_response(path: &str, response: reqwest::Response) -> Result<Value> {
    let status = response.status();
    let body: Value = response.json().await.unwrap_or(Value::Null);
    if !status.is_success() || body["success"] != json!(true) {
        bail!("{path} returned {status}: {body}");
    }
    Ok(body["data"].clone())
}

fn params(size: Size) -> EstateParams {
    match size {
        Size::Small => EstateParams::small(),
        Size::Medium => EstateParams::medium(),
        Size::Large => EstateParams::large(),
    }
}

/// Puts each switch's links on its ports as the LLDP records the switch would report: the far
/// switch's chassis MAC and port name, or only its chassis when the port could not be named.
fn attach_lldp_evidence(estate: &mut LargeEstate) {
    let mut chassis: HashMap<String, (String, Option<std::net::IpAddr>)> = HashMap::new();
    let mut port_names: HashMap<(String, i32), String> = HashMap::new();
    let host_names: HashMap<Uuid, String> = estate
        .hosts_with_services
        .iter()
        .map(|h| (h.host.id, h.host.base.name.to_string()))
        .collect();
    for hws in &estate.hosts_with_services {
        if let Some(chassis_id) = &hws.host.base.chassis_id {
            chassis.insert(
                hws.host.base.name.to_string(),
                (
                    chassis_id.value().0.clone(),
                    hws.ip_addresses.first().map(|ip| ip.base.ip_address),
                ),
            );
        }
    }
    for iface in &estate.interfaces {
        if let (Some(if_index), Some(descr)) = (iface.base.if_index, &iface.base.if_descr) {
            port_names.insert(
                (host_names[&iface.base.host_id].clone(), if_index),
                descr.clone(),
            );
        }
    }

    let mut evidence: HashMap<(String, i32), Vec<InterfaceNeighborEvidence>> = HashMap::new();
    let record = |target: &str, port: Option<String>| {
        let (chassis_mac, mgmt) = chassis[target].clone();
        InterfaceNeighborEvidence {
            lldp_chassis_id: Some(LldpChassisId::MacAddress(chassis_mac)),
            lldp_port_id: Some(match port {
                Some(name) => LldpPortId::InterfaceName(name),
                None => LldpPortId::LocallyAssigned("unknown".to_string()),
            }),
            lldp_sys_name: Some(target.to_string()),
            lldp_mgmt_addr: mgmt,
            evidence_source: AttributeSource::Probe(ClientProbe::Snmp),
            ..Default::default()
        }
    };
    for link in &estate.neighbor_updates {
        let port = link
            .target_if_index
            .and_then(|i| port_names.get(&(link.target_host_name.clone(), i)).cloned());
        evidence
            .entry((link.source_host_name.clone(), link.source_if_index))
            .or_default()
            .push(record(&link.target_host_name, port));
    }
    for link in &estate.host_neighbor_updates {
        evidence
            .entry((link.source_host_name.clone(), link.source_if_index))
            .or_default()
            .push(record(&link.target_host_name, None));
    }
    for iface in &mut estate.interfaces {
        if let Some(if_index) = iface.base.if_index
            && let Some(records) =
                evidence.remove(&(host_names[&iface.base.host_id].clone(), if_index))
        {
            iface.base.neighbor_candidates = records;
        }
    }
}

/// The ids the server assigned in one host submission's response.
fn collect_ids(host: &Value, scanned: &mut ScannedEntityIds) {
    let ids = |key: &str| -> Vec<Uuid> {
        host[key]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|row| row["id"].as_str()?.parse().ok())
            .collect()
    };
    if let Some(id) = host["id"].as_str().and_then(|s| s.parse().ok()) {
        scanned.host_ids.push(id);
    }
    scanned.ip_address_ids.extend(ids("ip_addresses"));
    scanned.port_ids.extend(ids("ports"));
    scanned.interface_ids.extend(ids("interfaces"));
    scanned.service_ids.extend(ids("services"));
    for service in host["services"].as_array().into_iter().flatten() {
        scanned.binding_ids.extend(
            service["bindings"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|b| b["id"].as_str()?.parse::<Uuid>().ok()),
        );
    }
}

async fn run_pass(daemon: &Daemon, estate: &LargeEstate, pass: u32) -> Result<Uuid> {
    let session = daemon.wait_for_session().await?;
    let started_at = Utc::now();
    let clock = Instant::now();
    println!("pass {pass} start {}", started_at.to_rfc3339());
    daemon
        .update(&session, DiscoveryPhase::Started, 0, started_at, None)
        .await?;

    let mut scanned = ScannedEntityIds::default();

    // Subnets, as the daemon reports what it swept. The server answers with its own ids.
    let mut subnet_ids: HashMap<Uuid, Uuid> = HashMap::new();
    for subnet in &estate.subnets {
        let created = daemon.post("/api/v1/subnets", subnet).await?;
        let id: Uuid = serde_json::from_value(created["id"].clone())?;
        subnet_ids.insert(subnet.id, id);
        scanned.subnet_ids.push(id);
    }

    // VLANs by number, as the switches' VLAN tables report them.
    let vlans: Vec<Value> = estate
        .vlans
        .iter()
        .map(|v| json!({ "vlan_number": v.base.vlan_number, "name": v.base.name.value().0 }))
        .collect();
    let created = daemon
        .post(
            "/api/v1/vlans/discovery",
            &json!({ "site_id": daemon.site_id, "vlans": vlans }),
        )
        .await?;
    let by_number: HashMap<u64, Uuid> = created["vlans"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| Some((v["vlan_number"].as_u64()?, v["id"].as_str()?.parse().ok()?)))
        .collect();
    let vlan_ids: HashMap<Uuid, Uuid> = estate
        .vlans
        .iter()
        .filter_map(|v| Some((v.id, *by_number.get(&u64::from(v.base.vlan_number))?)))
        .collect();
    scanned.vlan_ids.extend(vlan_ids.values());

    let mut interfaces_by_host: HashMap<Uuid, Vec<_>> = HashMap::new();
    for iface in &estate.interfaces {
        let mut iface = iface.clone();
        iface.base.native_vlan_id = iface
            .base
            .native_vlan_id
            .and_then(|v| vlan_ids.get(&v).copied());
        iface.base.vlan_ids = iface.base.vlan_ids.map(|ids| {
            ids.iter()
                .filter_map(|v| vlan_ids.get(v).copied())
                .collect()
        });
        interfaces_by_host
            .entry(iface.base.host_id)
            .or_default()
            .push(iface);
    }

    let total = estate.hosts_with_services.len();
    let mut latencies = Vec::with_capacity(total);
    let mut last_poll = Instant::now();
    for (i, hws) in estate.hosts_with_services.iter().enumerate() {
        let submit_started = Instant::now();
        let host_id = hws.host.id;
        let site_id = hws.host.base.site_id;
        let mut ip_addresses = hws.ip_addresses.clone();
        for ip in &mut ip_addresses {
            ip.base.subnet_id = subnet_ids[&ip.base.subnet_id];
        }
        let request = DiscoveryHostRequest {
            host: hws.host.clone(),
            ip_addresses,
            ports: hws
                .ports
                .iter()
                .cloned()
                .map(|p| p.with_host(host_id, site_id))
                .collect(),
            services: hws.services.clone(),
            interfaces: interfaces_by_host.remove(&host_id).unwrap_or_default(),
            subnets: Vec::new(),
            interfaces_complete: true,
            interface_data_complete: InterfaceDataComplete::default(),
            port_coverage: Default::default(),
            superseded_wire_shape: false,
        };
        let response = daemon.post("/api/v1/hosts/discovery", &request).await?;
        collect_ids(&response, &mut scanned);
        latencies.push(submit_started.elapsed());

        if (i + 1) % PROGRESS_EVERY == 0 {
            let progress = ((i + 1) * 99 / total) as u8;
            daemon
                .update(
                    &session,
                    DiscoveryPhase::Scanning,
                    progress,
                    started_at,
                    None,
                )
                .await?;
            println!(
                "pass {pass} hosts {}/{total} at {:.0}s",
                i + 1,
                clock.elapsed().as_secs_f64()
            );
        }
        if last_poll.elapsed() >= POLL_INTERVAL {
            daemon.poll(false).await?;
            last_poll = Instant::now();
        }
        if let Some(rest) = HOST_SUBMIT_INTERVAL.checked_sub(submit_started.elapsed()) {
            tokio::time::sleep(rest).await;
        }
    }
    let hosts_done = clock.elapsed();

    let completing = Instant::now();
    daemon
        .update(
            &session,
            DiscoveryPhase::Complete,
            100,
            started_at,
            Some(scanned),
        )
        .await?;
    let completion = completing.elapsed();

    latencies.sort();
    let pct = |p: usize| latencies[(latencies.len() * p / 100).min(latencies.len() - 1)];
    println!(
        "pass {pass} end {} hosts={total} host_phase={:.0}s complete_request={:.1}s submit_ms p50={} p95={} max={}",
        Utc::now().to_rfc3339(),
        hosts_done.as_secs_f64(),
        completion.as_secs_f64(),
        pct(50).as_millis(),
        pct(95).as_millis(),
        latencies.last().map(|d| d.as_millis()).unwrap_or(0),
    );

    Ok(session.discovery_id)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()?;
    let base = args.server_url.trim_end_matches('/').to_string();
    let version = env!("CARGO_PKG_VERSION").to_string();

    let provisioned = unwrap_response(
        "/api/v1/daemons/provision",
        client
            .post(format!("{base}/api/v1/daemons/provision"))
            .user_auth(&args)
            .json(&json!({
                "name": format!("estate-replay-{:?}", args.size).to_lowercase(),
                "site_id": args.site_id,
                "mode": "daemon_poll",
                "os": "linux",
            }))
            .send()
            .await?,
    )
    .await?;
    let daemon = Daemon {
        client: client.clone(),
        base: base.clone(),
        id: serde_json::from_value(provisioned["daemon"]["id"].clone())?,
        key: provisioned["daemon_api_key"]
            .as_str()
            .context("provisioning returned no key")?
            .to_string(),
        site_id: args.site_id,
        version: version.clone(),
    };
    daemon
        .post(
            &format!("/api/daemons/{}/startup", daemon.id),
            &json!({ "daemon_version": version, "os": "linux" }),
        )
        .await?;
    println!("daemon {} provisioned", daemon.id);

    let params = params(args.size);
    let now = Utc::now();
    let mut site = plant_site(Uuid::nil(), now);
    site.id = args.site_id;
    let mut estate = generate_large_estate(&params, &site, Uuid::nil(), now);
    attach_lldp_evidence(&mut estate);
    println!(
        "estate {:?}: {} switches, {} hosts, {} interfaces",
        args.size,
        params.switch_count(),
        estate.hosts_with_services.len(),
        estate.interfaces.len()
    );

    let mut discovery_id = None;
    for pass in 1..=args.passes {
        if let Some(id) = discovery_id {
            // The provisioned discovery ran once already; start it again for this pass.
            unwrap_response(
                "/api/v1/discoveries/start-session",
                client
                    .post(format!("{base}/api/v1/discoveries/start-session"))
                    .user_auth(&args)
                    .json(&json!(id))
                    .send()
                    .await?,
            )
            .await?;
        }
        let session_discovery = run_pass(&daemon, &estate, pass).await?;
        discovery_id = discovery_id.or(Some(session_discovery));
    }
    Ok(())
}

/// Signs a request as the user who owns the site: their API key or their session.
trait UserAuth {
    fn user_auth(self, args: &Args) -> Self;
}

impl UserAuth for reqwest::RequestBuilder {
    fn user_auth(self, args: &Args) -> Self {
        match (&args.user_api_key, &args.session_id) {
            (Some(key), _) => self.bearer_auth(key),
            (None, Some(session)) => self.header("Cookie", format!("session_id={session}")),
            (None, None) => unreachable!("clap requires one of the two"),
        }
    }
}
