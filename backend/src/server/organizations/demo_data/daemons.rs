use super::*;

// ============================================================================
// Daemons
// ============================================================================

pub(super) fn generate_daemons(
    networks: &[Network],
    hosts: &[&Host],
    now: DateTime<Utc>,
    user_id: Uuid,
) -> Vec<Daemon> {
    let find_network = |name: &str| {
        networks
            .iter()
            .find(|n| n.base.name.contains(name))
            .unwrap()
    };
    let find_host = |name: &str| {
        hosts
            .iter()
            .find(|h| h.base.name.value().as_str() == name)
            .copied()
    };

    let mut daemons = Vec::new();

    // HQ Daemon on docker-prod01. Interfaced subnets live in the `daemon_interfaced_subnets`
    // junction, not on the daemon row; see `generate_daemon_interfaced_subnets`.
    if let Some(host) = find_host("docker-prod01") {
        let network = find_network("Headquarters");
        daemons.push(Daemon {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: DaemonBase {
                host_id: host.id,
                network_id: network.id,
                url: "https://docker-prod01.acme.local:8443".to_string(),
                last_seen: Some(now),
                mode: DaemonMode::DaemonPoll,
                name: "HQ Daemon".to_string(),
                tags: vec![],
                version: Version::parse(env!("CARGO_PKG_VERSION"))
                    .map(Some)
                    .unwrap_or_default(),
                user_id,
                api_key_id: None,
                is_unreachable: false,
                standby: false,
                standby_cleared_at: None,
                os: Some(DaemonOs::Linux),
            },
        });
    }

    // DC Daemon on dc-docker01
    if let Some(host) = find_host("dc-docker01") {
        let network = find_network("Data Center");
        daemons.push(Daemon {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: DaemonBase {
                host_id: host.id,
                network_id: network.id,
                url: "https://docker01.dc.acme.io:8443".to_string(),
                last_seen: Some(now),
                mode: DaemonMode::DaemonPoll,
                standby_cleared_at: None,
                os: Some(DaemonOs::Linux),
                name: "DC Daemon".to_string(),
                tags: vec![],
                version: Version::parse(env!("CARGO_PKG_VERSION"))
                    .map(Some)
                    .unwrap_or_default(),
                user_id,
                api_key_id: None,
                is_unreachable: false,
                standby: false,
            },
        });
    }

    daemons
}

/// The subnets each daemon reports in its heartbeat: one per address on its host. Derived from
/// IP addresses rather than `Interface` rows because the daemon hosts carry a single eth0
/// interface, and their docker0 bridge address exists only as an IP address. A real daemon
/// reports both.
pub(super) fn generate_daemon_interfaced_subnets(
    daemons: &[Daemon],
    ip_addresses: &[&IPAddress],
) -> Vec<(Uuid, Vec<Uuid>)> {
    daemons
        .iter()
        .map(|daemon| {
            let mut subnet_ids: Vec<Uuid> = Vec::new();
            for ip in ip_addresses
                .iter()
                .filter(|ip| ip.base.host_id == daemon.base.host_id)
            {
                if !subnet_ids.contains(&ip.base.subnet_id) {
                    subnet_ids.push(ip.base.subnet_id);
                }
            }
            (daemon.id, subnet_ids)
        })
        .collect()
}
