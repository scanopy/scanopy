use super::*;
use crate::daemon::discovery::types::base::DiscoveryTerminalReason;
use crate::daemon::discovery::types::warnings::{ClaimSource, DiscoveryWarning, SnmpWalkGroup};
use crate::server::credentials::r#impl::mapping::CredentialQueryPayloadDiscriminants;
use crate::server::credentials::r#impl::run_results::{
    CredentialRunOutcome, CredentialRunResult, WakeOnLanResult,
};
use crate::server::credentials::r#impl::types::ssh_script::{SshScriptOutcome, SshScriptRun};

// ============================================================================
// Discoveries
// ============================================================================

pub(super) fn generate_discoveries(
    sites: &[Site],
    subnets: &[Subnet],
    daemons: &[Daemon],
    _hosts: &[&Host],
    credentials: &[Credential],
    now: DateTime<Utc>,
) -> Vec<Discovery> {
    let find_site = |name: &str| sites.iter().find(|n| n.base.name.contains(name)).unwrap();
    let find_daemon = |name: &str| daemons.iter().find(|d| d.base.name.contains(name));
    let find_subnets_for_site = |site_id: Uuid| -> Vec<Uuid> {
        subnets
            .iter()
            .filter(|s| s.base.site_id == site_id)
            .map(|s| s.id)
            .collect()
    };

    // Credential IDs for integration targets.
    let default_snmp_cred_id = credentials
        .iter()
        .find(|c| c.base.name == "Default SNMPv2c")
        .unwrap()
        .id;
    let linux_inventory_cred_id = credentials
        .iter()
        .find(|c| c.base.name == "Linux Inventory")
        .unwrap()
        .id;
    let backup_wake_cred_id = credentials
        .iter()
        .find(|c| c.base.name == "Backup NAS Wake")
        .unwrap()
        .id;
    let network_devices_cred_id = credentials
        .iter()
        .find(|c| c.base.name == "Network Devices")
        .unwrap()
        .id;
    let docker_socket_cred_id = credentials
        .iter()
        .find(|c| c.base.name == "Local Docker Socket")
        .unwrap()
        .id;

    // Both SNMP creds are broadcast to every site (see
    // `generate_site_credential_assignments`), so every discovery targets both.
    let snmp_network_targets = || {
        vec![
            IntegrationTarget::Site {
                credential_id: default_snmp_cred_id,
            },
            IntegrationTarget::Site {
                credential_id: network_devices_cred_id,
            },
        ]
    };
    // Ad-hoc discoveries additionally reach the daemon-local Docker socket.
    let snmp_and_docker_targets = || {
        let mut targets = snmp_network_targets();
        targets.push(IntegrationTarget::DaemonHost {
            credential_id: docker_socket_cred_id,
        });
        targets
    };

    let unified = |daemon: &Daemon, subnet_ids: Option<Vec<Uuid>>| DiscoveryType::Unified {
        host_id: daemon.base.host_id,
        subnet_ids,
        host_naming_fallback: HostNamingFallback::BestService,
        scan_settings: ScanSettings::default(),
    };

    let mut discoveries = Vec::new();

    // ===== HQ Unified discovery =====
    let hq = find_site("Headquarters");
    if let Some(daemon) = find_daemon("HQ") {
        let hq_subnet_ids = find_subnets_for_site(hq.id);
        discoveries.push(Discovery {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: DiscoveryBase {
                discovery_type: unified(daemon, Some(hq_subnet_ids.clone())),
                run_type: RunType::AdHoc {
                    last_run: Some(now - Duration::days(2)),
                },
                name: "Discovery".to_string(),
                daemon_id: daemon.id,
                site_id: hq.id,
                tags: vec![],
            },
            scan_count: 0,
            force_full_scan: false,
            integration_targets: snmp_and_docker_targets(),
        });

        // Historical — completed 3 weeks ago
        let three_weeks_ago = now - Duration::weeks(3);
        let hq_unified = unified(daemon, Some(hq_subnet_ids.clone()));
        discoveries.push(Discovery {
            id: Uuid::new_v4(),
            created_at: three_weeks_ago,
            updated_at: three_weeks_ago,
            base: DiscoveryBase {
                discovery_type: hq_unified.clone(),
                run_type: RunType::Historical {
                    results: Box::new(DiscoveryUpdatePayload {
                        session_id: Uuid::new_v4(),
                        daemon_id: daemon.id,
                        site_id: hq.id,
                        phase: DiscoveryPhase::Complete,
                        discovery_type: hq_unified.clone(),
                        progress: 100,
                        error: None,
                        warnings: Vec::new(),
                        credential_results: Vec::new(),
                        started_at: Some(three_weeks_ago),
                        finished_at: Some(three_weeks_ago + Duration::minutes(12)),
                        hosts_discovered: None,
                        estimated_remaining_secs: None,
                        discovery_id: None,
                        scanned: None,
                        reason: Some(DiscoveryTerminalReason::Completed),
                        last_update_at: Some(three_weeks_ago + Duration::minutes(12)),
                        daemon_version: daemon.base.version.as_ref().map(|v| v.to_string()),
                    }),
                },
                name: "Discovery".to_string(),
                daemon_id: daemon.id,
                site_id: hq.id,
                tags: vec![],
            },
            scan_count: 0,
            force_full_scan: false,
            integration_targets: snmp_network_targets(),
        });

        // Historical — completed 1 week ago
        let one_week_ago = now - Duration::weeks(1);
        discoveries.push(Discovery {
            id: Uuid::new_v4(),
            created_at: one_week_ago,
            updated_at: one_week_ago,
            base: DiscoveryBase {
                discovery_type: hq_unified.clone(),
                run_type: RunType::Historical {
                    results: Box::new(DiscoveryUpdatePayload {
                        session_id: Uuid::new_v4(),
                        daemon_id: daemon.id,
                        site_id: hq.id,
                        phase: DiscoveryPhase::Complete,
                        discovery_type: hq_unified,
                        progress: 100,
                        error: None,
                        // One warning per WarningRemedy group, so the report's four sections
                        // (and more than one Severity icon) all have something to show.
                        warnings: vec![
                            // FixInScanopy: nothing answered at this address for the SNMP
                            // credential during the scan.
                            DiscoveryWarning::CredentialTargetNotResponding {
                                address: IpAddr::V4(Ipv4Addr::new(10, 0, 1, 15)),
                                integration: CredentialQueryPayloadDiscriminants::Snmp,
                                credential_id: Some(network_devices_cred_id),
                            },
                            // CheckTheDevice: the switch claims LLDP (it answered
                            // lldpLocChassisId) but its neighbour table came back empty.
                            DiscoveryWarning::ClaimedCapabilityEmpty {
                                address: IpAddr::V4(Ipv4Addr::new(10, 0, 1, 3)),
                                group: SnmpWalkGroup::Lldp,
                                source: ClaimSource::LldpLocalIdentity,
                            },
                            // ClearsOnTheNextScan: a partial ARP-table read was discarded
                            // rather than recorded.
                            DiscoveryWarning::SnmpWalkPartialDiscarded {
                                address: IpAddr::V4(Ipv4Addr::new(10, 0, 40, 20)),
                                group: SnmpWalkGroup::ArpTable,
                            },
                            // NothingToDo: a firewall has no bridge ports to number, and no
                            // later scan changes that.
                            DiscoveryWarning::SnmpWalkUnsupported {
                                address: IpAddr::V4(Ipv4Addr::new(10, 0, 1, 1)),
                                group: SnmpWalkGroup::BridgePortNumbering,
                            },
                        ],
                        // What each credential produced: SNMP collected from the network devices,
                        // the inventory script ran on jenkins-ci, and the backup NAS woke for the scan.
                        credential_results: vec![
                            CredentialRunResult {
                                credential_id: network_devices_cred_id,
                                outcome: CredentialRunOutcome::Collected { hosts: 9 },
                            },
                            CredentialRunResult {
                                credential_id: linux_inventory_cred_id,
                                outcome: CredentialRunOutcome::SshScript {
                                    runs: vec![SshScriptRun {
                                        ip: IpAddr::V4(Ipv4Addr::new(10, 0, 20, 30)),
                                        outcome: SshScriptOutcome::Applied,
                                        exit_code: Some(0),
                                        applied_keys: [
                                            "hostname",
                                            "sys_descr",
                                            "manufacturer",
                                            "model",
                                            "serial_number",
                                            "firmware_revision",
                                            "software_revision",
                                        ]
                                        .map(String::from)
                                        .to_vec(),
                                        rejected_keys: Vec::new(),
                                        detail: None,
                                        duration_ms: 412,
                                    }],
                                },
                            },
                            CredentialRunResult {
                                credential_id: backup_wake_cred_id,
                                outcome: CredentialRunOutcome::WakeOnLan {
                                    hosts: vec![WakeOnLanResult {
                                        ip: IpAddr::V4(Ipv4Addr::new(10, 0, 40, 21)),
                                        woke: true,
                                    }],
                                },
                            },
                        ],
                        started_at: Some(one_week_ago),
                        finished_at: Some(one_week_ago + Duration::minutes(8)),
                        hosts_discovered: None,
                        estimated_remaining_secs: None,
                        discovery_id: None,
                        scanned: None,
                        reason: Some(DiscoveryTerminalReason::Completed),
                        last_update_at: Some(one_week_ago + Duration::minutes(8)),
                        daemon_version: daemon.base.version.as_ref().map(|v| v.to_string()),
                    }),
                },
                name: "Discovery".to_string(),
                daemon_id: daemon.id,
                site_id: hq.id,
                tags: vec![],
            },
            scan_count: 0,
            force_full_scan: false,
            integration_targets: snmp_network_targets(),
        });
    }

    // ===== DC Unified discovery =====
    let dc = find_site("Data Center");
    if let Some(daemon) = find_daemon("DC") {
        let dc_subnet_ids = find_subnets_for_site(dc.id);
        discoveries.push(Discovery {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: DiscoveryBase {
                discovery_type: unified(daemon, Some(dc_subnet_ids)),
                run_type: RunType::AdHoc {
                    last_run: Some(now - Duration::days(3)),
                },
                name: "Discovery".to_string(),
                daemon_id: daemon.id,
                site_id: dc.id,
                tags: vec![],
            },
            scan_count: 0,
            force_full_scan: false,
            integration_targets: snmp_and_docker_targets(),
        });
    }

    discoveries
}
