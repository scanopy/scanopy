//! Subnets

use super::*;

pub(super) fn generate_subnets(
    networks: &[Network],
    tags: &[Tag],
    docker_hq_svc_id: Uuid,
    docker_dc_svc_id: Uuid,
    now: DateTime<Utc>,
) -> Vec<Subnet> {
    let hq = networks
        .iter()
        .find(|n| n.base.name == "Headquarters")
        .unwrap();
    let dc = networks
        .iter()
        .find(|n| n.base.name == "Data Center")
        .unwrap();

    let monitoring_tag = tags
        .iter()
        .find(|t| t.base.name == "Monitoring")
        .map(|t| t.id);

    vec![
        // ===== Headquarters subnets (8) =====
        Subnet {
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
                cidr: SubnetCidr::new(
                    SubnetCidrValue(IpCidr::V4(
                        Ipv4Cidr::new(Ipv4Addr::new(10, 0, 1, 0), 24).unwrap(),
                    )),
                    AttributeSource::DaemonSelfReport,
                ),
                network_id: hq.id,
                name: "HQ Management".to_string(),
                description: Some("Network management and monitoring".to_string()),
                subnet_type: SubnetType::Management,
                virtualization_service_id: None,
                source: EntitySource::Manual,
                tags: monitoring_tag.into_iter().collect(),
            },
        },
        Subnet {
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
                cidr: SubnetCidr::new(
                    SubnetCidrValue(IpCidr::V4(
                        Ipv4Cidr::new(Ipv4Addr::new(10, 0, 10, 0), 24).unwrap(),
                    )),
                    AttributeSource::DaemonSelfReport,
                ),
                network_id: hq.id,
                name: "HQ Office LAN".to_string(),
                description: Some("Office workstations".to_string()),
                subnet_type: SubnetType::Lan,
                virtualization_service_id: None,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        Subnet {
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
                cidr: SubnetCidr::new(
                    SubnetCidrValue(IpCidr::V4(
                        Ipv4Cidr::new(Ipv4Addr::new(10, 0, 20, 0), 24).unwrap(),
                    )),
                    AttributeSource::DaemonSelfReport,
                ),
                network_id: hq.id,
                name: "HQ Servers".to_string(),
                description: Some("On-premises servers and hypervisors".to_string()),
                subnet_type: SubnetType::Lan,
                virtualization_service_id: None,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        Subnet {
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
                cidr: SubnetCidr::new(
                    SubnetCidrValue(IpCidr::V4(
                        Ipv4Cidr::new(Ipv4Addr::new(10, 0, 40, 0), 24).unwrap(),
                    )),
                    AttributeSource::DaemonSelfReport,
                ),
                network_id: hq.id,
                name: "HQ Storage".to_string(),
                description: Some("Storage area network".to_string()),
                subnet_type: SubnetType::Storage,
                virtualization_service_id: None,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        Subnet {
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
                cidr: SubnetCidr::new(
                    SubnetCidrValue(IpCidr::V4(
                        Ipv4Cidr::new(Ipv4Addr::new(10, 0, 30, 0), 24).unwrap(),
                    )),
                    AttributeSource::DaemonSelfReport,
                ),
                network_id: hq.id,
                name: "HQ IoT".to_string(),
                description: Some("Smart office devices".to_string()),
                subnet_type: SubnetType::IoT,
                virtualization_service_id: None,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        Subnet {
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
                cidr: SubnetCidr::new(
                    SubnetCidrValue(IpCidr::V4(
                        Ipv4Cidr::new(Ipv4Addr::new(172, 17, 0, 0), 16).unwrap(),
                    )),
                    AttributeSource::DaemonSelfReport,
                ),
                network_id: hq.id,
                name: "HQ Docker Bridge".to_string(),
                description: Some("Docker container network".to_string()),
                subnet_type: SubnetType::DockerBridge,
                virtualization_service_id: Some(docker_hq_svc_id),
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        Subnet {
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
                cidr: SubnetCidr::new(
                    SubnetCidrValue(IpCidr::V4(
                        Ipv4Cidr::new(Ipv4Addr::new(10, 0, 100, 0), 24).unwrap(),
                    )),
                    AttributeSource::DaemonSelfReport,
                ),
                network_id: hq.id,
                name: "HQ Guest WiFi".to_string(),
                description: Some("Guest wireless network".to_string()),
                subnet_type: SubnetType::Guest,
                virtualization_service_id: None,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        // HQ Annex: only known so far because the HQ core switch's LLDP table
        // advertises a neighbor on this range. Nothing here has been scanned
        // directly yet, so the range itself is provisional (cidr_source is
        // Inferred-tier rather than a daemon self-report or manual entry).
        Subnet {
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
                cidr: SubnetCidr::new(
                    SubnetCidrValue(IpCidr::V4(
                        Ipv4Cidr::new(Ipv4Addr::new(10, 0, 50, 0), 24).unwrap(),
                    )),
                    AttributeSource::LldpNeighbourAddress,
                ),
                network_id: hq.id,
                name: "HQ Annex".to_string(),
                description: Some(
                    "Recently added office annex, not yet scanned directly".to_string(),
                ),
                subnet_type: SubnetType::Lan,
                virtualization_service_id: None,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        // ===== Data Center subnets (6) =====
        Subnet {
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
                cidr: SubnetCidr::new(
                    SubnetCidrValue(IpCidr::V4(
                        Ipv4Cidr::new(Ipv4Addr::new(172, 16, 0, 0), 24).unwrap(),
                    )),
                    AttributeSource::DaemonSelfReport,
                ),
                network_id: dc.id,
                name: "DC Management".to_string(),
                description: Some("Data center management network".to_string()),
                subnet_type: SubnetType::Management,
                virtualization_service_id: None,
                source: EntitySource::Manual,
                tags: monitoring_tag.into_iter().collect(),
            },
        },
        Subnet {
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
                cidr: SubnetCidr::new(
                    SubnetCidrValue(IpCidr::V4(
                        Ipv4Cidr::new(Ipv4Addr::new(172, 16, 10, 0), 24).unwrap(),
                    )),
                    AttributeSource::DaemonSelfReport,
                ),
                network_id: dc.id,
                name: "DC Compute".to_string(),
                description: Some("Compute and hypervisor hosts".to_string()),
                subnet_type: SubnetType::Lan,
                virtualization_service_id: None,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        Subnet {
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
                cidr: SubnetCidr::new(
                    SubnetCidrValue(IpCidr::V4(
                        Ipv4Cidr::new(Ipv4Addr::new(172, 16, 20, 0), 24).unwrap(),
                    )),
                    AttributeSource::DaemonSelfReport,
                ),
                network_id: dc.id,
                name: "DC Storage".to_string(),
                description: Some("Storage network".to_string()),
                subnet_type: SubnetType::Storage,
                virtualization_service_id: None,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        Subnet {
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
                cidr: SubnetCidr::new(
                    SubnetCidrValue(IpCidr::V4(
                        Ipv4Cidr::new(Ipv4Addr::new(172, 16, 30, 0), 24).unwrap(),
                    )),
                    AttributeSource::DaemonSelfReport,
                ),
                network_id: dc.id,
                name: "DC DMZ".to_string(),
                description: Some("Demilitarized zone for public-facing services".to_string()),
                subnet_type: SubnetType::Dmz,
                virtualization_service_id: None,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        Subnet {
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
                cidr: SubnetCidr::new(
                    SubnetCidrValue(IpCidr::V4(
                        Ipv4Cidr::new(Ipv4Addr::new(172, 18, 0, 0), 16).unwrap(),
                    )),
                    AttributeSource::DaemonSelfReport,
                ),
                network_id: dc.id,
                name: "DC Docker Bridge".to_string(),
                description: Some("Docker container network".to_string()),
                subnet_type: SubnetType::DockerBridge,
                virtualization_service_id: Some(docker_dc_svc_id),
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        Subnet {
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
                cidr: SubnetCidr::new(
                    SubnetCidrValue(IpCidr::V4(
                        Ipv4Cidr::new(Ipv4Addr::new(10, 8, 0, 0), 24).unwrap(),
                    )),
                    AttributeSource::DaemonSelfReport,
                ),
                network_id: dc.id,
                name: "DC VPN Tunnel".to_string(),
                description: Some("VPN tunnel to headquarters".to_string()),
                subnet_type: SubnetType::VpnTunnel,
                virtualization_service_id: None,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
    ]
}
