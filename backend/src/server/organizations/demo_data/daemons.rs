use super::*;

// ============================================================================
// Daemons
// ============================================================================

pub(super) fn generate_daemons(
    networks: &[Network],
    hosts: &[&Host],
    _subnets: &[Subnet],
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

    // HQ Daemon on docker-prod01. (Interfaced subnets are populated at runtime from
    // daemon heartbeats into the `daemon_interfaced_subnets` junction; demo seed data
    // leaves them empty.)
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
