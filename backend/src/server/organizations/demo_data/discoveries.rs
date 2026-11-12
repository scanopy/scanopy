use super::*;
use crate::daemon::discovery::types::base::DiscoveryTerminalReason;
use crate::daemon::discovery::types::warnings::{
    ClaimSource, DiscoveryWarning, ProvisionalSubnet, SnmpWalkGroup, UnconfirmedHost,
};
use crate::server::credentials::r#impl::mapping::CredentialQueryPayloadDiscriminants;
use crate::server::credentials::r#impl::run_results::{
    CredentialRunOutcome, CredentialRunResult, WakeOnLanResult,
};
use crate::server::credentials::r#impl::types::ssh_script::{SshScriptOutcome, SshScriptRun};
use crate::server::credentials::r#impl::types::{
    CredentialIntegration, CredentialTypeDiscriminants,
};

// ============================================================================
// Discoveries
// ============================================================================

pub(super) fn generate_discoveries(
    sites: &[Site],
    subnets: &[Subnet],
    daemons: &[Daemon],
    hosts: &[&Host],
    interfaces: &[Interface],
    credentials: &[Credential],
    now: DateTime<Utc>,
) -> Vec<Discovery> {
    let find_site = |name: &str| sites.iter().find(|n| n.base.name.contains(name)).unwrap();
    let find_daemon = |name: &str| daemons.iter().find(|d| d.base.name.contains(name));
    // The subnets a site's scan targets. Container bridges are left out, as the target picker
    // leaves them out: the Docker integration reads their containers from the runtime.
    let find_subnets_for_site = |site_id: Uuid| -> Vec<Uuid> {
        subnets
            .iter()
            .filter(|s| s.base.site_id == site_id && !s.is_container_bridge_subnet())
            .map(|s| s.id)
            .collect()
    };

    // Credential IDs for integration targets.
    let default_snmp_cred_id =
        super::credentials::find_default(credentials, CredentialTypeDiscriminants::SnmpCommunity)
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
    let branch_snmp_cred_id = credentials
        .iter()
        .find(|c| c.base.name == "Branch Office SNMPv3")
        .unwrap()
        .id;
    let docker_socket_cred_id =
        super::credentials::find_default(credentials, CredentialTypeDiscriminants::DockerSocket).id;

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
            // Both historical runs below completed, so the next run is a light scan.
            scan_count: 2,
            force_full_scan: false,
            next_run_is_full_scan: false,
            integration_targets: snmp_and_docker_targets(),
        });

        // Historical — the first run, a light scan, completed 3 weeks ago
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
                        finished_at: Some(three_weeks_ago + Duration::minutes(6)),
                        hosts_discovered: None,
                        estimated_remaining_secs: None,
                        discovery_id: None,
                        scanned: None,
                        reason: Some(DiscoveryTerminalReason::Completed),
                        last_update_at: Some(three_weeks_ago + Duration::minutes(6)),
                        daemon_version: daemon.base.version.as_ref().map(|v| v.to_string()),
                        last_activity_at: None,
                    }),
                },
                name: "Discovery".to_string(),
                daemon_id: daemon.id,
                site_id: hq.id,
                tags: vec![],
            },
            scan_count: 0,
            force_full_scan: false,
            next_run_is_full_scan: false,
            integration_targets: snmp_network_targets(),
        });

        // What each HQ credential produced on a scan: SNMP collected from the network devices,
        // the inventory script ran on jenkins-ci, and the backup NAS woke for the scan.
        let hq_credential_results = vec![
            CredentialRunResult {
                credential_id: network_devices_cred_id,
                outcome: CredentialRunOutcome::Collected {
                    hosts: 9,
                    hosts_with_guests_listed: vec![],
                },
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
        ];

        // Historical — the second run, a full scan of all ports, completed 1 week ago
        let one_week_ago = now - Duration::weeks(1);
        let mut hq_unified_full = hq_unified;
        if let DiscoveryType::Unified { scan_settings, .. } = &mut hq_unified_full {
            scan_settings.is_full_scan = true;
        }
        discoveries.push(Discovery {
            id: Uuid::new_v4(),
            created_at: one_week_ago,
            updated_at: one_week_ago,
            base: DiscoveryBase {
                discovery_type: hq_unified_full.clone(),
                run_type: RunType::Historical {
                    results: Box::new(DiscoveryUpdatePayload {
                        session_id: Uuid::new_v4(),
                        daemon_id: daemon.id,
                        site_id: hq.id,
                        phase: DiscoveryPhase::Complete,
                        discovery_type: hq_unified_full,
                        progress: 100,
                        error: None,
                        // One warning per WarningRemedy group, so the report's four sections
                        // (and more than one Severity icon) all have something to show.
                        warnings: hq_open_warnings(network_devices_cred_id)
                            .into_iter()
                            .chain([
                                // FixInScanopy: a credential stored without a target, which no scan
                                // sends to a daemon.
                                DiscoveryWarning::CredentialWithoutTarget {
                                    integration: CredentialQueryPayloadDiscriminants::Snmp,
                                    credential_id: Some(branch_snmp_cred_id),
                                },
                                // ClearsOnTheNextScan: a partial ARP-table read was discarded
                                // rather than recorded.
                                DiscoveryWarning::SnmpWalkPartialDiscarded {
                                    address: IpAddr::V4(Ipv4Addr::new(10, 0, 40, 20)),
                                    group: SnmpWalkGroup::ArpTable,
                                    credential_id: Some(network_devices_cred_id),
                                    snmp_version: None,
                                },
                                // NothingToDo: a firewall has no bridge ports to number, and no
                                // later scan changes that.
                                DiscoveryWarning::SnmpWalkUnsupported {
                                    address: IpAddr::V4(Ipv4Addr::new(10, 0, 1, 1)),
                                    group: SnmpWalkGroup::BridgePortNumbering,
                                    credential_id: Some(network_devices_cred_id),
                                    snmp_version: None,
                                },
                            ])
                            // The annex range, assumed around the address the HQ switch's LLDP table
                            // heard the annex uplink advertise. The subnet's resolve modal reads its
                            // evidence from this warning.
                            .chain(annex_range_warning(subnets, hosts))
                            // The two hosts it knew only from other devices' reports, which each
                            // resolve modal links back to as the scan that found it.
                            .chain(unconfirmed_host_warnings(hosts, interfaces))
                            .collect(),
                        credential_results: hq_credential_results.clone(),
                        started_at: Some(one_week_ago),
                        finished_at: Some(one_week_ago + Duration::minutes(35)),
                        hosts_discovered: None,
                        estimated_remaining_secs: None,
                        discovery_id: None,
                        scanned: None,
                        reason: Some(DiscoveryTerminalReason::Completed),
                        last_update_at: Some(one_week_ago + Duration::minutes(35)),
                        daemon_version: daemon.base.version.as_ref().map(|v| v.to_string()),
                        last_activity_at: None,
                    }),
                },
                name: "Discovery".to_string(),
                daemon_id: daemon.id,
                site_id: hq.id,
                tags: vec![],
            },
            scan_count: 0,
            force_full_scan: false,
            next_run_is_full_scan: false,
            integration_targets: snmp_network_targets(),
        });

        // Historical — the latest run, 2 days ago, still couldn't reach 10.0.1.15 or read the
        // core switch's LLDP table, so those two stay on Home. The rest of the week-old warnings
        // were fixed and stay only in that run's report.
        let two_days_ago = now - Duration::days(2);
        let hq_latest = unified(daemon, Some(hq_subnet_ids.clone()));
        discoveries.push(Discovery {
            id: Uuid::new_v4(),
            created_at: two_days_ago,
            updated_at: two_days_ago,
            base: DiscoveryBase {
                discovery_type: hq_latest.clone(),
                run_type: RunType::Historical {
                    results: Box::new(DiscoveryUpdatePayload {
                        session_id: Uuid::new_v4(),
                        daemon_id: daemon.id,
                        site_id: hq.id,
                        phase: DiscoveryPhase::Complete,
                        discovery_type: hq_latest,
                        progress: 100,
                        error: None,
                        warnings: hq_open_warnings(network_devices_cred_id),
                        credential_results: hq_credential_results,
                        started_at: Some(two_days_ago),
                        finished_at: Some(two_days_ago + Duration::minutes(7)),
                        hosts_discovered: None,
                        estimated_remaining_secs: None,
                        discovery_id: None,
                        scanned: None,
                        reason: Some(DiscoveryTerminalReason::Completed),
                        last_update_at: Some(two_days_ago + Duration::minutes(7)),
                        last_activity_at: None,
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
            next_run_is_full_scan: false,
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
            // The historical run below completed.
            scan_count: 1,
            force_full_scan: false,
            next_run_is_full_scan: false,
            integration_targets: snmp_and_docker_targets(),
        });

        // Historical — the run 3 days ago. It found Proxmox VE on dc-proxmox-hv01 with no Proxmox
        // credential, so the report prompts for one. The credential was added since.
        let three_days_ago = now - Duration::days(3);
        let dc_unified = unified(daemon, Some(find_subnets_for_site(dc.id)));
        let find_cred_id =
            |name: &str| credentials.iter().find(|c| c.base.name == name).unwrap().id;
        let dc_proxmox_host_id = hosts
            .iter()
            .find(|h| h.base.name.value().as_str() == "dc-proxmox-hv01")
            .map(|h| h.id);
        discoveries.push(Discovery {
            id: Uuid::new_v4(),
            created_at: three_days_ago,
            updated_at: three_days_ago,
            base: DiscoveryBase {
                discovery_type: dc_unified.clone(),
                run_type: RunType::Historical {
                    results: Box::new(DiscoveryUpdatePayload {
                        session_id: Uuid::new_v4(),
                        daemon_id: daemon.id,
                        site_id: dc.id,
                        phase: DiscoveryPhase::Complete,
                        discovery_type: dc_unified,
                        progress: 100,
                        error: None,
                        warnings: dc_proxmox_host_id
                            .map(|host_id| DiscoveryWarning::IntegrationWithoutCredential {
                                host_id: Some(host_id),
                                integration: CredentialIntegration::Proxmox,
                            })
                            .into_iter()
                            // The core switch reported 64 interfaces and the SNMPv3 read
                            // returned 52, so Home lists the gap as an Issue.
                            .chain([DiscoveryWarning::ClaimedCountUnderRead {
                                address: IpAddr::V4(Ipv4Addr::new(172, 16, 0, 2)),
                                group: SnmpWalkGroup::Interfaces,
                                source: ClaimSource::IfNumber,
                                expected: 64,
                                observed: 52,
                                credential_id: Some(find_cred_id("Data Center SNMPv3")),
                                snmp_version: None,
                            }])
                            .collect(),
                        // SNMPv3 read the firewall and the switch, and gNMI the switch.
                        credential_results: vec![
                            CredentialRunResult {
                                credential_id: find_cred_id("Data Center SNMPv3"),
                                outcome: CredentialRunOutcome::Collected {
                                    hosts: 2,
                                    hosts_with_guests_listed: vec![],
                                },
                            },
                            CredentialRunResult {
                                credential_id: find_cred_id("Arista gNMI"),
                                outcome: CredentialRunOutcome::Collected {
                                    hosts: 1,
                                    hosts_with_guests_listed: vec![],
                                },
                            },
                        ],
                        started_at: Some(three_days_ago),
                        finished_at: Some(three_days_ago + Duration::minutes(12)),
                        hosts_discovered: None,
                        estimated_remaining_secs: None,
                        discovery_id: None,
                        scanned: None,
                        reason: Some(DiscoveryTerminalReason::Completed),
                        last_update_at: Some(three_days_ago + Duration::minutes(12)),
                        daemon_version: daemon.base.version.as_ref().map(|v| v.to_string()),
                        last_activity_at: None,
                    }),
                },
                name: "Discovery".to_string(),
                daemon_id: daemon.id,
                site_id: dc.id,
                tags: vec![],
            },
            scan_count: 0,
            force_full_scan: false,
            next_run_is_full_scan: false,
            integration_targets: snmp_network_targets(),
        });
    }

    discoveries
}

/// The two HQ warnings no scan has cleared yet, so Home lists them as Issues.
fn hq_open_warnings(network_devices_cred_id: Uuid) -> Vec<DiscoveryWarning> {
    vec![
        // FixInScanopy: nothing answered at this address for the SNMP credential.
        DiscoveryWarning::CredentialTargetNotResponding {
            address: IpAddr::V4(Ipv4Addr::new(10, 0, 1, 15)),
            integration: CredentialQueryPayloadDiscriminants::Snmp,
            credential_id: Some(network_devices_cred_id),
        },
        // CheckTheDevice: the switch claims LLDP (it answered lldpLocChassisId) but its
        // neighbour table came back empty.
        DiscoveryWarning::ClaimedCapabilityEmpty {
            address: IpAddr::V4(Ipv4Addr::new(10, 0, 1, 3)),
            group: SnmpWalkGroup::Lldp,
            source: ClaimSource::LldpLocalIdentity,
            credential_id: Some(network_devices_cred_id),
            snmp_version: None,
        },
    ]
}

/// The warning the HQ scan raised when it assumed the annex's range: the address the annex uplink
/// advertised over LLDP, and the HQ switch that heard it.
fn annex_range_warning(subnets: &[Subnet], hosts: &[&Host]) -> Option<DiscoveryWarning> {
    let annex = subnets
        .iter()
        .find(|s| s.base.name().contains("HQ Annex"))?;
    let switch = hosts
        .iter()
        .find(|h| h.base.name.value().as_str() == "unifi-usw-48")?;
    Some(DiscoveryWarning::ProvisionalSubnetInferred(
        ProvisionalSubnet {
            cidr: annex.base.cidr.value().to_string(),
            subnet_id: Some(annex.id),
            addresses: vec!["10.0.50.1".to_string()],
            sys_names: vec![],
            seen_by_host_ids: vec![switch.id],
            widened_by_vlan: false,
        },
    ))
}

/// One `UnconfirmedHostInferred` warning per unconfirmed demo host: the HQ scan reported each one,
/// by the chassis ID a neighbour advertised or the MAC a switch learned (on the host's interface).
///
/// Every unconfirmed host gets one: its resolve modal links back to this scan. A host with no
/// identifier is a broken demo row, so it panics rather than quietly going without.
fn unconfirmed_host_warnings(hosts: &[&Host], interfaces: &[Interface]) -> Vec<DiscoveryWarning> {
    hosts
        .iter()
        .filter_map(|host| {
            let EntitySource::Inferred { via } = host.base.source else {
                return None;
            };
            let identifier = host
                .base
                .chassis_id
                .as_ref()
                .map(|c| c.value().to_string())
                .or_else(|| {
                    interfaces
                        .iter()
                        .filter(|i| i.base.host_id == host.id)
                        .find_map(|i| i.base.mac_address.as_ref().map(|m| m.value().to_string()))
                })
                .unwrap_or_else(|| {
                    panic!(
                        "unconfirmed demo host {} has no chassis ID or interface MAC to be reported by",
                        host.id
                    )
                });
            Some(DiscoveryWarning::UnconfirmedHostInferred(UnconfirmedHost {
                host_id: Some(host.id),
                via,
                identifier,
                sys_name: None,
            }))
        })
        .collect()
}
