//! SNMP interface data for the demo hosts.
//!
//! Kept as one file rather than split further: this is one hand-authored interface table
//! (the HQ/DC split lives in the inline comments below), not a set of independent
//! responsibilities -- splitting it would fragment a single dataset without adding clarity.

use super::*;

/// Build the SNMP interface rows for the demo hosts, plus the neighbor links
/// that `run_populate_demo` resolves to IDs once the hosts exist.
///
/// `neighbor_interface_id` is a self-FK to `interfaces(id)`, and the whole set is
/// inserted by one non-transactional `create_many`. That resolves today only
/// because all ~85 rows fit in a single chunk (36 bound columns per row, so
/// 65535 / 36 = 1820 rows per chunk), leaving a row free to reference a later row
/// in the same statement. Adding switches past that boundary would make a chunk-1
/// row pointing at a chunk-2 row fail deterministically — at which point the
/// insert needs one shared transaction with the FK deferred, not smaller chunks.
pub(super) fn generate_interfaces(
    networks: &[Network],
    hosts: &[&Host],
    ip_addresses: &[&IPAddress],
    vlans: &[Vlan],
    now: DateTime<Utc>,
) -> (Vec<Interface>, Vec<NeighborUpdate>) {
    let mut interfaces = Vec::new();
    let mut neighbor_updates = Vec::new();

    // By title rather than by stored name: the annex uplink has no name and is titled by the
    // chassis ID its neighbour advertised.
    let find_host = |name: &str| {
        hosts
            .iter()
            .find(|h| h.display_name(&[]).as_deref() == Some(name))
            .copied()
    };
    let find_ip_address = |host_id: Uuid| {
        ip_addresses
            .iter()
            .find(|i| i.base.host_id == host_id)
            .copied()
    };

    // VLAN lookup: (network_id, vlan_number) → VLAN entity UUID
    let find_vlan = |network_id: Uuid, vlan_number: u16| -> Option<Uuid> {
        vlans
            .iter()
            .find(|v| v.base.network_id == network_id && v.base.vlan_number == vlan_number)
            .map(|v| v.id)
    };
    let find_vlans = |network_id: Uuid, vlan_numbers: &[u16]| -> Option<Vec<Uuid>> {
        let ids: Vec<Uuid> = vlan_numbers
            .iter()
            .filter_map(|&n| find_vlan(network_id, n))
            .collect();
        if ids.is_empty() { None } else { Some(ids) }
    };

    // ========================================================================
    // HQ: pfSense firewall — multiple ip_addresses
    // ========================================================================
    if let Some(host) = find_host("pfsense-fw01") {
        let network = networks
            .iter()
            .find(|n| n.id == host.base.network_id)
            .unwrap();
        let ip_address = find_ip_address(host.id);

        // WAN interface
        interfaces.push(Interface {
            display_name: None,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: InterfaceBase {
                neighbor_candidates: Default::default(),
                host_id: host.id,
                network_id: network.id,
                if_index: Some(1),
                if_descr: Some("igb0".to_string()),
                if_name: None,
                if_alias: Some("WAN".to_string()),
                if_type: Some(6),
                speed_bps: Some(1_000_000_000),
                admin_status: Some(IfAdminStatus::Up),
                oper_status: Some(IfOperStatus::Up),
                mac_address: Some(MacEvidence::new(
                    MacEvidenceValue(MacAddress::new([0xa4, 0xbe, 0x2b, 0x10, 0x01, 0x10])),
                    AttributeSource::ArpReply,
                )),
                ip_address_id: None,
                ip_configured: false,
                fdb_macs: None,
                native_vlan_id: None,
                vlan_ids: None,
            },
        });

        // LAN interface — connected to HQ switch port 1
        interfaces.push(Interface {
            display_name: None,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: InterfaceBase {
                neighbor_candidates: Default::default(),
                host_id: host.id,
                network_id: network.id,
                if_index: Some(2),
                if_descr: Some("igb1".to_string()),
                if_name: None,
                if_alias: Some("LAN".to_string()),
                if_type: Some(6),
                speed_bps: Some(1_000_000_000),
                admin_status: Some(IfAdminStatus::Up),
                oper_status: Some(IfOperStatus::Up),
                mac_address: Some(MacEvidence::new(
                    MacEvidenceValue(MacAddress::new([0xa4, 0xbe, 0x2b, 0x10, 0x01, 0x11])),
                    AttributeSource::ArpReply,
                )),
                ip_address_id: ip_address.map(|i| i.id),
                ip_configured: false,
                fdb_macs: None,
                native_vlan_id: find_vlan(network.id, 1),
                vlan_ids: find_vlans(network.id, &[10, 20, 30, 100]),
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "pfsense-fw01".to_string(),
            source_if_index: 2,
            target_host_name: "unifi-usw-48".to_string(),
            target_if_index: 1,
        });

        // OPT1 interface (disabled)
        interfaces.push(Interface {
            display_name: None,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: InterfaceBase {
                neighbor_candidates: Default::default(),
                host_id: host.id,
                network_id: network.id,
                if_index: Some(3),
                if_descr: Some("igb2".to_string()),
                if_name: None,
                if_alias: Some("OPT1".to_string()),
                if_type: Some(6),
                speed_bps: Some(1_000_000_000),
                admin_status: Some(IfAdminStatus::Down),
                oper_status: Some(IfOperStatus::Down),
                mac_address: Some(MacEvidence::new(
                    MacEvidenceValue(MacAddress::new([0xa4, 0xbe, 0x2b, 0x10, 0x01, 0x12])),
                    AttributeSource::ArpReply,
                )),
                ip_address_id: None,
                ip_configured: false,
                fdb_macs: None,
                native_vlan_id: None,
                vlan_ids: None,
            },
        });
    }

    // ========================================================================
    // HQ: TrueNAS — bonded ip_addresses, connected to switch port 2
    // ========================================================================
    if let Some(host) = find_host("truenas-primary") {
        let network = networks
            .iter()
            .find(|n| n.id == host.base.network_id)
            .unwrap();
        let ip_address = find_ip_address(host.id);

        interfaces.push(Interface {
            display_name: None,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: InterfaceBase {
                neighbor_candidates: Default::default(),
                host_id: host.id,
                network_id: network.id,
                if_index: Some(1),
                if_descr: Some("lagg0".to_string()),
                if_name: None,
                if_alias: Some("LACP Bond".to_string()),
                if_type: Some(161),
                speed_bps: Some(10_000_000_000),
                admin_status: Some(IfAdminStatus::Up),
                oper_status: Some(IfOperStatus::Up),
                mac_address: Some(MacEvidence::new(
                    MacEvidenceValue(MacAddress::new([0xd0, 0x50, 0x99, 0x40, 0x17, 0x10])),
                    AttributeSource::ArpReply,
                )),
                ip_address_id: ip_address.map(|i| i.id),
                ip_configured: false,
                fdb_macs: None,
                native_vlan_id: find_vlan(network.id, 20),
                vlan_ids: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "truenas-primary".to_string(),
            source_if_index: 1,
            target_host_name: "unifi-usw-48".to_string(),
            target_if_index: 2,
        });
    }

    // ========================================================================
    // HQ: Proxmox HV01 — with loopback, connected to switch port 3
    // ========================================================================
    if let Some(host) = find_host("proxmox-hv01") {
        let network = networks
            .iter()
            .find(|n| n.id == host.base.network_id)
            .unwrap();
        let ip_address = find_ip_address(host.id);

        interfaces.push(Interface {
            display_name: None,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: InterfaceBase {
                neighbor_candidates: Default::default(),
                host_id: host.id,
                network_id: network.id,
                if_index: Some(1),
                if_descr: Some("eno1".to_string()),
                if_name: None,
                if_alias: Some("Primary NIC".to_string()),
                if_type: Some(6),
                speed_bps: Some(10_000_000_000),
                admin_status: Some(IfAdminStatus::Up),
                oper_status: Some(IfOperStatus::Up),
                mac_address: Some(MacEvidence::new(
                    MacEvidenceValue(MacAddress::new([0xf8, 0xbc, 0x12, 0x20, 0x08, 0x10])),
                    AttributeSource::ArpReply,
                )),
                ip_address_id: ip_address.map(|i| i.id),
                ip_configured: false,
                fdb_macs: None,
                native_vlan_id: find_vlan(network.id, 20),
                vlan_ids: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "proxmox-hv01".to_string(),
            source_if_index: 1,
            target_host_name: "unifi-usw-48".to_string(),
            target_if_index: 3,
        });

        // Loopback
        interfaces.push(Interface {
            display_name: None,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: InterfaceBase {
                neighbor_candidates: Default::default(),
                host_id: host.id,
                network_id: network.id,
                if_index: Some(2),
                if_descr: Some("lo".to_string()),
                if_name: None,
                if_alias: None,
                if_type: Some(24),
                speed_bps: None,
                admin_status: Some(IfAdminStatus::Up),
                oper_status: Some(IfOperStatus::Up),
                mac_address: None,
                ip_address_id: None,
                ip_configured: false,
                fdb_macs: None,
                native_vlan_id: None,
                vlan_ids: None,
            },
        });
    }

    // ========================================================================
    // HQ: Proxmox HV02 — connected to switch port 4
    // ========================================================================
    if let Some(host) = find_host("proxmox-hv02") {
        let network = networks
            .iter()
            .find(|n| n.id == host.base.network_id)
            .unwrap();
        let ip_address = find_ip_address(host.id);

        interfaces.push(Interface {
            display_name: None,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: InterfaceBase {
                neighbor_candidates: Default::default(),
                host_id: host.id,
                network_id: network.id,
                if_index: Some(1),
                if_descr: Some("eno1".to_string()),
                if_name: None,
                if_alias: Some("Primary NIC".to_string()),
                if_type: Some(6),
                speed_bps: Some(10_000_000_000),
                admin_status: Some(IfAdminStatus::Up),
                oper_status: Some(IfOperStatus::Up),
                mac_address: Some(MacEvidence::new(
                    MacEvidenceValue(MacAddress::new([0xf8, 0xbc, 0x12, 0x20, 0x09, 0x10])),
                    AttributeSource::ArpReply,
                )),
                ip_address_id: ip_address.map(|i| i.id),
                ip_configured: false,
                fdb_macs: None,
                native_vlan_id: find_vlan(network.id, 20),
                vlan_ids: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "proxmox-hv02".to_string(),
            source_if_index: 1,
            target_host_name: "unifi-usw-48".to_string(),
            target_if_index: 4,
        });
    }

    // ========================================================================
    // HQ: docker-prod01 — connected to switch port 5
    // ========================================================================
    if let Some(host) = find_host("docker-prod01") {
        let network = networks
            .iter()
            .find(|n| n.id == host.base.network_id)
            .unwrap();
        let ip_address = find_ip_address(host.id);

        interfaces.push(Interface {
            display_name: None,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: InterfaceBase {
                neighbor_candidates: Default::default(),
                host_id: host.id,
                network_id: network.id,
                if_index: Some(1),
                if_descr: Some("eth0".to_string()),
                if_name: None,
                if_alias: Some("Primary NIC".to_string()),
                if_type: Some(6),
                speed_bps: Some(10_000_000_000),
                admin_status: Some(IfAdminStatus::Up),
                oper_status: Some(IfOperStatus::Up),
                mac_address: Some(MacEvidence::new(
                    MacEvidenceValue(MacAddress::new([0xf8, 0xbc, 0x12, 0x20, 0x13, 0x10])),
                    AttributeSource::ArpReply,
                )),
                ip_address_id: ip_address.map(|i| i.id),
                ip_configured: false,
                fdb_macs: None,
                native_vlan_id: find_vlan(network.id, 20),
                vlan_ids: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "docker-prod01".to_string(),
            source_if_index: 1,
            target_host_name: "unifi-usw-48".to_string(),
            target_if_index: 5,
        });
    }

    // ========================================================================
    // HQ Switch — unifi-usw-48 (48 ports)
    // ========================================================================
    if let Some(host) = find_host("unifi-usw-48") {
        let network = networks
            .iter()
            .find(|n| n.id == host.base.network_id)
            .unwrap();
        let ip_address = find_ip_address(host.id);

        // Port 1 ↔ pfsense-fw01
        interfaces.push(Interface {
            display_name: None,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: InterfaceBase {
                neighbor_candidates: Default::default(),
                host_id: host.id,
                network_id: network.id,
                if_index: Some(1),
                if_descr: Some("Port 1/0/1".to_string()),
                if_name: None,
                if_alias: Some("pfSense uplink".to_string()),
                if_type: Some(6),
                speed_bps: Some(1_000_000_000),
                admin_status: Some(IfAdminStatus::Up),
                oper_status: Some(IfOperStatus::Up),
                mac_address: Some(MacEvidence::new(
                    MacEvidenceValue(MacAddress::new([0xfc, 0xec, 0xda, 0x10, 0x03, 0x01])),
                    AttributeSource::ArpReply,
                )),
                ip_address_id: ip_address.map(|i| i.id),
                ip_configured: false,
                fdb_macs: None,
                native_vlan_id: find_vlan(network.id, 1),
                vlan_ids: find_vlans(network.id, &[10, 20, 30, 100]),
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "unifi-usw-48".to_string(),
            source_if_index: 1,
            target_host_name: "pfsense-fw01".to_string(),
            target_if_index: 2,
        });

        // Port 2 ↔ truenas-primary
        interfaces.push(Interface {
            display_name: None,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: InterfaceBase {
                neighbor_candidates: Default::default(),
                host_id: host.id,
                network_id: network.id,
                if_index: Some(2),
                if_descr: Some("Port 1/0/2".to_string()),
                if_name: None,
                if_alias: Some("TrueNAS uplink".to_string()),
                if_type: Some(6),
                speed_bps: Some(10_000_000_000),
                admin_status: Some(IfAdminStatus::Up),
                oper_status: Some(IfOperStatus::Up),
                mac_address: Some(MacEvidence::new(
                    MacEvidenceValue(MacAddress::new([0xfc, 0xec, 0xda, 0x10, 0x03, 0x02])),
                    AttributeSource::ArpReply,
                )),
                ip_address_id: None,
                ip_configured: false,
                fdb_macs: None,
                native_vlan_id: find_vlan(network.id, 20),
                vlan_ids: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "unifi-usw-48".to_string(),
            source_if_index: 2,
            target_host_name: "truenas-primary".to_string(),
            target_if_index: 1,
        });

        // Port 3 ↔ proxmox-hv01
        interfaces.push(Interface {
            display_name: None,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: InterfaceBase {
                neighbor_candidates: Default::default(),
                host_id: host.id,
                network_id: network.id,
                if_index: Some(3),
                if_descr: Some("Port 1/0/3".to_string()),
                if_name: None,
                if_alias: Some("Proxmox HV01 uplink".to_string()),
                if_type: Some(6),
                speed_bps: Some(10_000_000_000),
                admin_status: Some(IfAdminStatus::Up),
                oper_status: Some(IfOperStatus::Up),
                mac_address: Some(MacEvidence::new(
                    MacEvidenceValue(MacAddress::new([0xfc, 0xec, 0xda, 0x10, 0x03, 0x03])),
                    AttributeSource::ArpReply,
                )),
                ip_address_id: None,
                ip_configured: false,
                fdb_macs: None,
                native_vlan_id: find_vlan(network.id, 20),
                vlan_ids: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "unifi-usw-48".to_string(),
            source_if_index: 3,
            target_host_name: "proxmox-hv01".to_string(),
            target_if_index: 1,
        });

        // Port 4 ↔ proxmox-hv02
        interfaces.push(Interface {
            display_name: None,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: InterfaceBase {
                neighbor_candidates: Default::default(),
                host_id: host.id,
                network_id: network.id,
                if_index: Some(4),
                if_descr: Some("Port 1/0/4".to_string()),
                if_name: None,
                if_alias: Some("Proxmox HV02 uplink".to_string()),
                if_type: Some(6),
                speed_bps: Some(10_000_000_000),
                admin_status: Some(IfAdminStatus::Up),
                oper_status: Some(IfOperStatus::Up),
                mac_address: Some(MacEvidence::new(
                    MacEvidenceValue(MacAddress::new([0xfc, 0xec, 0xda, 0x10, 0x03, 0x04])),
                    AttributeSource::ArpReply,
                )),
                ip_address_id: None,
                ip_configured: false,
                fdb_macs: None,
                native_vlan_id: find_vlan(network.id, 20),
                vlan_ids: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "unifi-usw-48".to_string(),
            source_if_index: 4,
            target_host_name: "proxmox-hv02".to_string(),
            target_if_index: 1,
        });

        // Port 5 ↔ docker-prod01
        interfaces.push(Interface {
            display_name: None,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: InterfaceBase {
                neighbor_candidates: Default::default(),
                host_id: host.id,
                network_id: network.id,
                if_index: Some(5),
                if_descr: Some("Port 1/0/5".to_string()),
                if_name: None,
                if_alias: Some("Docker host uplink".to_string()),
                if_type: Some(6),
                speed_bps: Some(10_000_000_000),
                admin_status: Some(IfAdminStatus::Up),
                oper_status: Some(IfOperStatus::Up),
                mac_address: Some(MacEvidence::new(
                    MacEvidenceValue(MacAddress::new([0xfc, 0xec, 0xda, 0x10, 0x03, 0x05])),
                    AttributeSource::ArpReply,
                )),
                ip_address_id: None,
                ip_configured: false,
                fdb_macs: None,
                native_vlan_id: find_vlan(network.id, 20),
                vlan_ids: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "unifi-usw-48".to_string(),
            source_if_index: 5,
            target_host_name: "docker-prod01".to_string(),
            target_if_index: 1,
        });

        // Port 6 ↔ unifi-ap-lobby (deferred via NeighborUpdate)
        interfaces.push(Interface {
            display_name: None,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: InterfaceBase {
                neighbor_candidates: Default::default(),
                host_id: host.id,
                network_id: network.id,
                if_index: Some(6),
                if_descr: Some("Port 1/0/6".to_string()),
                if_name: None,
                if_alias: Some("UniFi AP".to_string()),
                if_type: Some(6),
                speed_bps: Some(1_000_000_000),
                admin_status: Some(IfAdminStatus::Up),
                oper_status: Some(IfOperStatus::Up),
                mac_address: Some(MacEvidence::new(
                    MacEvidenceValue(MacAddress::new([0xfc, 0xec, 0xda, 0x10, 0x03, 0x06])),
                    AttributeSource::ArpReply,
                )),
                ip_address_id: None,
                ip_configured: false,
                fdb_macs: None,
                native_vlan_id: find_vlan(network.id, 1),
                vlan_ids: find_vlans(network.id, &[10, 20, 30, 100]),
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "unifi-usw-48".to_string(),
            source_if_index: 6,
            target_host_name: "unifi-ap-lobby".to_string(),
            target_if_index: 1,
        });

        // Ports 7-48 — empty/down
        for port_num in 7..=48 {
            interfaces.push(Interface {
                display_name: None,
                valid_from: now,
                valid_to: None,
                lineage_id: None,
                last_seen_at: now,
                last_discovery_id: None,
                first_discovery_id: None,
                id: Uuid::new_v4(),
                created_at: now,
                updated_at: now,
                base: InterfaceBase {
                    neighbor_candidates: Default::default(),
                    host_id: host.id,
                    network_id: network.id,
                    if_index: Some(port_num),
                    if_descr: Some(format!("Port 1/0/{}", port_num)),
                    if_name: None,
                    if_alias: None,
                    if_type: Some(6),
                    speed_bps: Some(1_000_000_000),
                    admin_status: Some(IfAdminStatus::Up),
                    oper_status: Some(IfOperStatus::Down),
                    mac_address: Some(MacEvidence::new(
                        MacEvidenceValue(MacAddress::new([
                            0xfc,
                            0xec,
                            0xda,
                            0x10,
                            0x03,
                            port_num as u8,
                        ])),
                        AttributeSource::ArpReply,
                    )),
                    ip_address_id: None,
                    ip_configured: false,
                    fdb_macs: None,
                    native_vlan_id: None,
                    vlan_ids: None,
                },
            });
        }
    }

    // ========================================================================
    // HQ: unifi-ap-lobby — single Interface for LLDP neighbor with switch port 6
    // ========================================================================
    if let Some(host) = find_host("unifi-ap-lobby") {
        let network = networks
            .iter()
            .find(|n| n.id == host.base.network_id)
            .unwrap();
        let ip_address = find_ip_address(host.id);

        interfaces.push(Interface {
            display_name: None,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: InterfaceBase {
                neighbor_candidates: Default::default(),
                host_id: host.id,
                network_id: network.id,
                if_index: Some(1),
                if_descr: Some("eth0".to_string()),
                if_name: None,
                if_alias: Some("Ethernet".to_string()),
                if_type: Some(6),
                speed_bps: Some(1_000_000_000),
                admin_status: Some(IfAdminStatus::Up),
                oper_status: Some(IfOperStatus::Up),
                mac_address: Some(MacEvidence::new(
                    MacEvidenceValue(MacAddress::new([0xfc, 0xec, 0xda, 0x30, 0x23, 0x10])),
                    AttributeSource::ArpReply,
                )),
                ip_address_id: ip_address.map(|i| i.id),
                ip_configured: false,
                fdb_macs: None,
                native_vlan_id: find_vlan(network.id, 1),
                vlan_ids: find_vlans(network.id, &[10, 20, 30, 100]),
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "unifi-ap-lobby".to_string(),
            source_if_index: 1,
            target_host_name: "unifi-usw-48".to_string(),
            target_if_index: 6,
        });
    }

    // ========================================================================
    // HQ: hq-annex-uplink — known only from the HQ core switch's LLDP table, never
    // scanned directly, so ifIndex/ifType/status are unknown rather than absent-and-zero.
    // ========================================================================
    if let Some(host) = find_host("hq-annex-uplink") {
        let network = networks
            .iter()
            .find(|n| n.id == host.base.network_id)
            .unwrap();
        let ip_address = find_ip_address(host.id);

        interfaces.push(Interface {
            display_name: None,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: InterfaceBase {
                neighbor_candidates: Default::default(),
                host_id: host.id,
                network_id: network.id,
                if_index: None,
                if_descr: Some("Uplink".to_string()),
                if_name: None,
                if_alias: None,
                if_type: None,
                speed_bps: None,
                admin_status: None,
                oper_status: None,
                mac_address: None,
                ip_address_id: ip_address.map(|i| i.id),
                ip_configured: false,
                fdb_macs: None,
                native_vlan_id: None,
                vlan_ids: None,
            },
        });
    }

    // ========================================================================
    // DC: dc-fw01 — connected to DC switch port 1
    // ========================================================================
    if let Some(host) = find_host("dc-fw01") {
        let network = networks
            .iter()
            .find(|n| n.id == host.base.network_id)
            .unwrap();
        let ip_address = find_ip_address(host.id);

        interfaces.push(Interface {
            display_name: None,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: InterfaceBase {
                neighbor_candidates: Default::default(),
                host_id: host.id,
                network_id: network.id,
                if_index: Some(1),
                if_descr: Some("port1".to_string()),
                if_name: None,
                if_alias: Some("LAN".to_string()),
                if_type: Some(6),
                speed_bps: Some(10_000_000_000),
                admin_status: Some(IfAdminStatus::Up),
                oper_status: Some(IfOperStatus::Up),
                mac_address: Some(MacEvidence::new(
                    MacEvidenceValue(MacAddress::new([0x70, 0x4c, 0xa5, 0xdc, 0x01, 0x10])),
                    AttributeSource::ArpReply,
                )),
                ip_address_id: ip_address.map(|i| i.id),
                ip_configured: false,
                fdb_macs: None,
                native_vlan_id: find_vlan(network.id, 1),
                vlan_ids: find_vlans(network.id, &[10, 20, 30, 100]),
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "dc-fw01".to_string(),
            source_if_index: 1,
            target_host_name: "dc-switch-01".to_string(),
            target_if_index: 1,
        });
    }

    // ========================================================================
    // DC: dc-proxmox-hv01 — connected to DC switch port 2
    // ========================================================================
    if let Some(host) = find_host("dc-proxmox-hv01") {
        let network = networks
            .iter()
            .find(|n| n.id == host.base.network_id)
            .unwrap();
        let ip_address = find_ip_address(host.id);

        interfaces.push(Interface {
            display_name: None,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: InterfaceBase {
                neighbor_candidates: Default::default(),
                host_id: host.id,
                network_id: network.id,
                if_index: Some(1),
                if_descr: Some("eno1".to_string()),
                if_name: None,
                if_alias: Some("Primary NIC".to_string()),
                if_type: Some(6),
                speed_bps: Some(10_000_000_000),
                admin_status: Some(IfAdminStatus::Up),
                oper_status: Some(IfOperStatus::Up),
                mac_address: Some(MacEvidence::new(
                    MacEvidenceValue(MacAddress::new([0xf8, 0xbc, 0x12, 0xdc, 0x07, 0x10])),
                    AttributeSource::ArpReply,
                )),
                ip_address_id: ip_address.map(|i| i.id),
                ip_configured: false,
                fdb_macs: None,
                native_vlan_id: find_vlan(network.id, 20),
                vlan_ids: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "dc-proxmox-hv01".to_string(),
            source_if_index: 1,
            target_host_name: "dc-switch-01".to_string(),
            target_if_index: 2,
        });
    }

    // ========================================================================
    // DC: dc-docker01 — connected to DC switch port 3
    // ========================================================================
    if let Some(host) = find_host("dc-docker01") {
        let network = networks
            .iter()
            .find(|n| n.id == host.base.network_id)
            .unwrap();
        let ip_address = find_ip_address(host.id);

        interfaces.push(Interface {
            display_name: None,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: InterfaceBase {
                neighbor_candidates: Default::default(),
                host_id: host.id,
                network_id: network.id,
                if_index: Some(1),
                if_descr: Some("eth0".to_string()),
                if_name: None,
                if_alias: Some("Primary NIC".to_string()),
                if_type: Some(6),
                speed_bps: Some(10_000_000_000),
                admin_status: Some(IfAdminStatus::Up),
                oper_status: Some(IfOperStatus::Up),
                mac_address: Some(MacEvidence::new(
                    MacEvidenceValue(MacAddress::new([0xf8, 0xbc, 0x12, 0xdc, 0x11, 0x10])),
                    AttributeSource::ArpReply,
                )),
                ip_address_id: ip_address.map(|i| i.id),
                ip_configured: false,
                fdb_macs: None,
                native_vlan_id: find_vlan(network.id, 20),
                vlan_ids: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "dc-docker01".to_string(),
            source_if_index: 1,
            target_host_name: "dc-switch-01".to_string(),
            target_if_index: 3,
        });
    }

    // ========================================================================
    // DC: haproxy-lb01 — connected to DC switch port 4
    // ========================================================================
    if let Some(host) = find_host("haproxy-lb01") {
        let network = networks
            .iter()
            .find(|n| n.id == host.base.network_id)
            .unwrap();
        let ip_address = find_ip_address(host.id);

        interfaces.push(Interface {
            display_name: None,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: InterfaceBase {
                neighbor_candidates: Default::default(),
                host_id: host.id,
                network_id: network.id,
                if_index: Some(1),
                if_descr: Some("eth0".to_string()),
                if_name: None,
                if_alias: Some("Primary NIC".to_string()),
                if_type: Some(6),
                speed_bps: Some(10_000_000_000),
                admin_status: Some(IfAdminStatus::Up),
                oper_status: Some(IfOperStatus::Up),
                mac_address: Some(MacEvidence::new(
                    MacEvidenceValue(MacAddress::new([0xf8, 0xbc, 0x12, 0xdc, 0x04, 0x10])),
                    AttributeSource::ArpReply,
                )),
                ip_address_id: ip_address.map(|i| i.id),
                ip_configured: false,
                fdb_macs: None,
                native_vlan_id: find_vlan(network.id, 20),
                vlan_ids: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "haproxy-lb01".to_string(),
            source_if_index: 1,
            target_host_name: "dc-switch-01".to_string(),
            target_if_index: 4,
        });
    }

    // ========================================================================
    // DC Switch — dc-switch-01 (24 ports)
    // ========================================================================
    if let Some(host) = find_host("dc-switch-01") {
        let network = networks
            .iter()
            .find(|n| n.id == host.base.network_id)
            .unwrap();
        let ip_address = find_ip_address(host.id);

        // Port 1 ↔ dc-fw01
        interfaces.push(Interface {
            display_name: None,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: InterfaceBase {
                neighbor_candidates: Default::default(),
                host_id: host.id,
                network_id: network.id,
                if_index: Some(1),
                if_descr: Some("Port 1/0/1".to_string()),
                if_name: None,
                if_alias: Some("Firewall uplink".to_string()),
                if_type: Some(6),
                speed_bps: Some(10_000_000_000),
                admin_status: Some(IfAdminStatus::Up),
                oper_status: Some(IfOperStatus::Up),
                mac_address: Some(MacEvidence::new(
                    MacEvidenceValue(MacAddress::new([0x00, 0x1c, 0x73, 0xdc, 0x02, 0x01])),
                    AttributeSource::ArpReply,
                )),
                ip_address_id: ip_address.map(|i| i.id),
                ip_configured: false,
                fdb_macs: None,
                native_vlan_id: find_vlan(network.id, 1),
                vlan_ids: find_vlans(network.id, &[10, 20, 30, 100]),
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "dc-switch-01".to_string(),
            source_if_index: 1,
            target_host_name: "dc-fw01".to_string(),
            target_if_index: 1,
        });

        // Port 2 ↔ dc-proxmox-hv01
        interfaces.push(Interface {
            display_name: None,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: InterfaceBase {
                neighbor_candidates: Default::default(),
                host_id: host.id,
                network_id: network.id,
                if_index: Some(2),
                if_descr: Some("Port 1/0/2".to_string()),
                if_name: None,
                if_alias: Some("Proxmox uplink".to_string()),
                if_type: Some(6),
                speed_bps: Some(10_000_000_000),
                admin_status: Some(IfAdminStatus::Up),
                oper_status: Some(IfOperStatus::Up),
                mac_address: Some(MacEvidence::new(
                    MacEvidenceValue(MacAddress::new([0x00, 0x1c, 0x73, 0xdc, 0x02, 0x02])),
                    AttributeSource::ArpReply,
                )),
                ip_address_id: None,
                ip_configured: false,
                fdb_macs: None,
                native_vlan_id: find_vlan(network.id, 20),
                vlan_ids: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "dc-switch-01".to_string(),
            source_if_index: 2,
            target_host_name: "dc-proxmox-hv01".to_string(),
            target_if_index: 1,
        });

        // Port 3 ↔ dc-docker01
        interfaces.push(Interface {
            display_name: None,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: InterfaceBase {
                neighbor_candidates: Default::default(),
                host_id: host.id,
                network_id: network.id,
                if_index: Some(3),
                if_descr: Some("Port 1/0/3".to_string()),
                if_name: None,
                if_alias: Some("Docker host uplink".to_string()),
                if_type: Some(6),
                speed_bps: Some(10_000_000_000),
                admin_status: Some(IfAdminStatus::Up),
                oper_status: Some(IfOperStatus::Up),
                mac_address: Some(MacEvidence::new(
                    MacEvidenceValue(MacAddress::new([0x00, 0x1c, 0x73, 0xdc, 0x02, 0x03])),
                    AttributeSource::ArpReply,
                )),
                ip_address_id: None,
                ip_configured: false,
                fdb_macs: None,
                native_vlan_id: find_vlan(network.id, 20),
                vlan_ids: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "dc-switch-01".to_string(),
            source_if_index: 3,
            target_host_name: "dc-docker01".to_string(),
            target_if_index: 1,
        });

        // Port 4 ↔ haproxy-lb01
        interfaces.push(Interface {
            display_name: None,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: InterfaceBase {
                neighbor_candidates: Default::default(),
                host_id: host.id,
                network_id: network.id,
                if_index: Some(4),
                if_descr: Some("Port 1/0/4".to_string()),
                if_name: None,
                if_alias: Some("HAProxy uplink".to_string()),
                if_type: Some(6),
                speed_bps: Some(10_000_000_000),
                admin_status: Some(IfAdminStatus::Up),
                oper_status: Some(IfOperStatus::Up),
                mac_address: Some(MacEvidence::new(
                    MacEvidenceValue(MacAddress::new([0x00, 0x1c, 0x73, 0xdc, 0x02, 0x04])),
                    AttributeSource::ArpReply,
                )),
                ip_address_id: None,
                ip_configured: false,
                fdb_macs: None,
                native_vlan_id: find_vlan(network.id, 20),
                vlan_ids: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "dc-switch-01".to_string(),
            source_if_index: 4,
            target_host_name: "haproxy-lb01".to_string(),
            target_if_index: 1,
        });

        // Ports 5-24 — empty/down
        for port_num in 5..=24 {
            interfaces.push(Interface {
                display_name: None,
                valid_from: now,
                valid_to: None,
                lineage_id: None,
                last_seen_at: now,
                last_discovery_id: None,
                first_discovery_id: None,
                id: Uuid::new_v4(),
                created_at: now,
                updated_at: now,
                base: InterfaceBase {
                    neighbor_candidates: Default::default(),
                    host_id: host.id,
                    network_id: network.id,
                    if_index: Some(port_num),
                    if_descr: Some(format!("Port 1/0/{}", port_num)),
                    if_name: None,
                    if_alias: None,
                    if_type: Some(6),
                    speed_bps: Some(1_000_000_000),
                    admin_status: Some(IfAdminStatus::Up),
                    oper_status: Some(IfOperStatus::Down),
                    mac_address: Some(MacEvidence::new(
                        MacEvidenceValue(MacAddress::new([
                            0x00,
                            0x1c,
                            0x73,
                            0xdc,
                            0x02,
                            port_num as u8,
                        ])),
                        AttributeSource::ArpReply,
                    )),
                    ip_address_id: None,
                    ip_configured: false,
                    fdb_macs: None,
                    native_vlan_id: None,
                    vlan_ids: None,
                },
            });
        }
    }

    (interfaces, neighbor_updates)
}
