//! The demo's "Plant" site: the default generated estate, with the daemon that scans it and its
//! discovery history.

use super::large_estate::{EstateParams, generate_large_estate, plant_site};
use super::*;
use crate::daemon::discovery::types::base::DiscoveryTerminalReason;
use crate::daemon::discovery::types::warnings::DiscoveryWarning;
use crate::server::credentials::r#impl::mapping::CredentialQueryPayloadDiscriminants;
use crate::server::credentials::r#impl::run_results::{CredentialRunOutcome, CredentialRunResult};
use crate::server::credentials::r#impl::types::CredentialTypeDiscriminants;

/// Adds the Plant site and everything on it to `data`. Plant gets no snapshot: the hand-built
/// sites carry the snapshot story, and cloning 4,000 hosts' rows would dominate every reset.
pub(super) fn add_plant_site(
    data: &mut DemoData,
    organization_id: Uuid,
    user_id: Uuid,
    use_case: UseCase,
    now: DateTime<Utc>,
) {
    let site = plant_site(organization_id, now);
    let params = EstateParams::default();
    let mut estate = generate_large_estate(&params, &site, organization_id, now);
    // As `DemoData::generate` does for the hand-built hosts: the OS is matched from what SNMP
    // reported once everything else about the host is known.
    for hws in &mut estate.hosts_with_services {
        hws.host.base.match_os_from_system_strings();
    }

    let docker_bridge = {
        let daemon_host = estate
            .hosts_with_services
            .iter_mut()
            .find(|h| h.host.id == estate.daemon_host_id)
            .expect("the daemon host is one of the estate's hosts");
        run_docker(daemon_host, &site, now)
    };

    let host_ips: Vec<&IPAddress> = estate
        .hosts_with_services
        .iter()
        .flat_map(|h| h.ip_addresses.iter())
        .collect();
    let daemon_host = estate
        .hosts_with_services
        .iter()
        .find(|h| h.host.id == estate.daemon_host_id)
        .expect("the daemon host is one of the estate's hosts");

    let daemon = Daemon {
        id: Uuid::new_v4(),
        created_at: now,
        updated_at: now,
        base: DaemonBase {
            host_id: daemon_host.host.id,
            site_id: site.id,
            url: format!("https://{}:8443", daemon_host.host.base.name),
            last_seen: Some(now),
            mode: DaemonMode::DaemonPoll,
            name: "Plant Daemon".to_string(),
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
    };
    let mut daemons = vec![daemon];
    let api_keys = generate_api_keys(&daemons, now);
    bind_daemon_api_keys(&mut daemons, &api_keys);
    let daemon_interfaced_subnets = generate_daemon_interfaced_subnets(&daemons, &host_ips);
    let daemon = &daemons[0];

    // The SNMP credentials every demo site carries, which read the estate's switches.
    let default_snmp = super::credentials::find_default(
        &data.credentials,
        CredentialTypeDiscriminants::SnmpCommunity,
    )
    .id;
    let network_snmp = data
        .credentials
        .iter()
        .find(|c| c.base.name == "Network Devices")
        .expect("the demo defines the Network Devices credential")
        .id;
    let snmp_targets = vec![
        IntegrationTarget::Site {
            credential_id: default_snmp,
        },
        IntegrationTarget::Site {
            credential_id: network_snmp,
        },
    ];

    // Plant's one Issue: the first access switch didn't answer SNMP on the last scan, so Home
    // lists it on this site too. The run collected from every other switch and firewall.
    let unanswered: Vec<DiscoveryWarning> = estate
        .hosts_with_services
        .iter()
        .find(|h| h.host.base.name.value().as_str().starts_with("acc-sw-"))
        .and_then(|h| h.ip_addresses.first())
        .map(|ip| DiscoveryWarning::CredentialTargetNotResponding {
            address: ip.base.ip_address,
            integration: CredentialQueryPayloadDiscriminants::Snmp,
            credential_id: Some(network_snmp),
        })
        .into_iter()
        .collect();
    let collected_hosts = (params.switch_count() + params.firewalls - unanswered.len()) as u32;

    let subnet_ids: Vec<Uuid> = estate.subnets.iter().map(|s| s.id).collect();
    let unified = DiscoveryType::Unified {
        host_id: daemon.base.host_id,
        subnet_ids: Some(subnet_ids),
        host_naming_fallback: HostNamingFallback::BestService,
        scan_settings: ScanSettings::default(),
    };
    let started = now - Duration::days(1);
    let finished = started + Duration::minutes(94);
    let discoveries = vec![
        Discovery {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: DiscoveryBase {
                discovery_type: unified.clone(),
                run_type: RunType::AdHoc {
                    last_run: Some(started),
                },
                name: "Discovery".to_string(),
                daemon_id: daemon.id,
                site_id: site.id,
                tags: vec![],
            },
            // The historical run below completed.
            scan_count: 1,
            force_full_scan: false,
            next_run_is_full_scan: false,
            integration_targets: snmp_targets.clone(),
        },
        Discovery {
            id: Uuid::new_v4(),
            created_at: started,
            updated_at: finished,
            base: DiscoveryBase {
                discovery_type: unified.clone(),
                run_type: RunType::Historical {
                    results: Box::new(DiscoveryUpdatePayload {
                        session_id: Uuid::new_v4(),
                        daemon_id: daemon.id,
                        site_id: site.id,
                        phase: DiscoveryPhase::Complete,
                        discovery_type: unified,
                        progress: 100,
                        error: None,
                        warnings: unanswered,
                        credential_results: vec![CredentialRunResult {
                            credential_id: network_snmp,
                            outcome: CredentialRunOutcome::Collected {
                                hosts: collected_hosts,
                                hosts_with_guests_listed: vec![],
                            },
                        }],
                        started_at: Some(started),
                        finished_at: Some(finished),
                        hosts_discovered: None,
                        estimated_remaining_secs: None,
                        discovery_id: None,
                        scanned: None,
                        reason: Some(DiscoveryTerminalReason::Completed),
                        last_update_at: Some(finished),
                        daemon_version: daemon.base.version.as_ref().map(|v| v.to_string()),
                        last_activity_at: None,
                    }),
                },
                name: "Discovery".to_string(),
                daemon_id: daemon.id,
                site_id: site.id,
                tags: vec![],
            },
            scan_count: 0,
            force_full_scan: false,
            next_run_is_full_scan: false,
            integration_targets: snmp_targets,
        },
    ];

    data.site_credential_assignments
        .push(SiteCredentialAssignment {
            site_id: site.id,
            credential_ids: vec![default_snmp, network_snmp],
        });
    data.topologies.extend(generate_topologies(
        std::slice::from_ref(&site),
        &data.tags,
        use_case,
        now,
    ));
    data.sites.push(site);
    data.subnets.extend(estate.subnets);
    data.subnets.push(docker_bridge);
    data.vlans.extend(estate.vlans);
    data.subnet_vlan_records.extend(estate.subnet_vlan_records);
    data.hosts_with_services.extend(estate.hosts_with_services);
    data.interfaces.extend(estate.interfaces);
    data.neighbor_updates.extend(estate.neighbor_updates);
    data.host_neighbor_updates
        .extend(estate.host_neighbor_updates);
    data.daemon_interfaced_subnets
        .extend(daemon_interfaced_subnets);
    data.api_keys.extend(api_keys);
    data.daemons.extend(daemons);
    data.discoveries.extend(discoveries);
}

/// The daemon runs in Docker, as the documented install does, so its host carries the Docker
/// service and an address on the default bridge, and the bridge subnet the service owns.
fn run_docker(host: &mut HostWithServices, site: &Site, now: DateTime<Utc>) -> Subnet {
    use crate::server::subnets::r#impl::base::{SubnetName, SubnetTypeAttributed};

    let docker_service_id = Uuid::new_v4();
    let bridge = Subnet {
        valid_from: now,
        valid_to: None,
        lineage_id: None,
        last_seen_at: now,
        last_discovery_id: None,
        first_discovery_id: None,
        id: Uuid::new_v4(),
        created_at: now,
        updated_at: now,
        base: SubnetBase {
            vlan_ids: Vec::new(),
            cidr: SubnetCidr::new(
                SubnetCidrValue(IpCidr::V4(
                    Ipv4Cidr::new(Ipv4Addr::new(172, 17, 0, 0), 16).unwrap(),
                )),
                AttributeSource::DaemonSelfReport,
            ),
            site_id: site.id,
            name: SubnetName::unattributed("Plant Docker Bridge".to_string()),
            description: Some("Docker container network on the monitoring server".to_string()),
            subnet_type: SubnetTypeAttributed::unattributed(SubnetType::DockerBridge),
            virtualization_service_id: Some(docker_service_id),
            source: EntitySource::Discovery,
            tags: vec![],
        },
    };

    let eth0 = host.ip_addresses[0].clone();
    let mut docker0 = eth0.clone();
    docker0.id = Uuid::new_v4();
    docker0.base.subnet_id = bridge.id;
    docker0.base.ip_address = IpAddr::V4(Ipv4Addr::new(172, 17, 0, 1));
    docker0.base.mac_address = Some(MacEvidence::new(
        MacEvidenceValue(MacAddress::new([0x02, 0x42, 0xac, 0x11, 0x00, 0x01])),
        AttributeSource::ArpReply,
    ));
    docker0.base.name = Some("docker0".to_string());
    docker0.base.position = 1;
    host.ip_addresses.push(docker0);

    if let Some((mut service, port)) = create_service_with_id(
        docker_service_id,
        "Docker",
        "Docker Daemon",
        &host.host,
        &eth0,
        Some(PortType::Docker),
        vec![],
        now,
    ) {
        service.base.source = EntitySource::Discovery;
        host.ports.extend(port);
        host.services.push(service);
    }

    bridge
}
