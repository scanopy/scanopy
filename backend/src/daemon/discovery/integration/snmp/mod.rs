//! SNMP discovery integration.
//!
//! Probe: credentialed SNMP check on UDP ports 161/1161.
//! Execute: walks ifTable, queries LLDP/CDP/ARP/Entity-MIB/Bridge-FDB,
//!          enriches HostData with system info, ip_addresses, and interfaces.
//!
//! Also contains low-level SNMP utilities (queries, session management, OIDs, types).

pub mod oids;
pub mod queries;
pub mod session;
pub mod types;
pub mod values;

// Re-export commonly used items
pub use queries::{
    query_arp_table, query_bridge_fdb, query_cdp_neighbors, query_entity_physical,
    query_ip_addr_table, query_lldp_local, query_lldp_neighbors, query_port_vlan_membership,
    query_system_info, query_vlan_table, walk_if_table,
};
pub use session::SNMP_WALK_TIMEOUT;
pub use types::{
    ArpEntry, BridgeFdbEntry, CdpNeighbor, DeviceInventory, IfTableEntry, IpAddrEntry,
    LldpLocalInfo, LldpNeighbor, PortVlanMembership, SystemInfo, VlanInfo,
};

use std::net::IpAddr;
use std::time::Duration;

use anyhow::{Error, Result};
use async_trait::async_trait;
use tokio::time::timeout;
use tracing::debug;
use uuid::Uuid;

use crate::{
    daemon::utils::scanner::try_snmp_with_credential_on_port,
    server::{
        credentials::r#impl::{
            mapping::{
                CredentialQueryPayload, CredentialQueryPayloadDiscriminants, SnmpQueryCredential,
            },
            types::CredentialAssignment,
        },
        hosts::r#impl::base::{Host, HostBase},
        interfaces::r#impl::base::{
            IfAdminStatus, IfOperStatus, Interface, InterfaceBase, if_type,
        },
        ip_addresses::r#impl::base::{IPAddress, IPAddressBase},
        ports::r#impl::base::PortType,
        services::r#impl::patterns::ClientProbe,
        shared::types::entities::EntitySource,
        snmp::resolution::lldp::{LldpChassisId, LldpPortId},
        subnets::r#impl::base::Subnet,
    },
};

use super::{DiscoveryIntegration, IntegrationContext, ProbeContext, ProbeFailure, ProbeSuccess};
use crate::daemon::discovery::service::ops::HostData;

/// Handle returned by a successful SNMP probe — carries the working credential and port.
pub struct SnmpProbeHandle {
    pub credential: SnmpQueryCredential,
    pub port: u16,
}

pub struct SnmpIntegration;

#[async_trait]
impl DiscoveryIntegration for SnmpIntegration {
    fn credential_type(&self) -> CredentialQueryPayloadDiscriminants {
        CredentialQueryPayloadDiscriminants::Snmp
    }

    fn estimated_seconds(&self) -> u32 {
        15
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(300)
    }

    // No probe_gate_ports — SNMP does its own UDP port probing.

    async fn probe(&self, ctx: &ProbeContext<'_>) -> Result<ProbeSuccess, ProbeFailure> {
        let snmp_cred = match ctx.credential {
            CredentialQueryPayload::Snmp(cred) => cred,
            _ => {
                return Err(ProbeFailure {
                    message: "Expected SNMP credential".to_string(),
                });
            }
        };

        let snmp_ports: &[u16] = &[161, 1161];

        // Try the provided credential on each SNMP port
        for &port in snmp_ports {
            if ctx.cancel.is_cancelled() {
                return Err(ProbeFailure {
                    message: "Cancelled".to_string(),
                });
            }

            match try_snmp_with_credential_on_port(ctx.ip, snmp_cred, port).await {
                Ok(Some(detected_port)) => {
                    return Ok(ProbeSuccess {
                        client_probe: ClientProbe::Snmp,
                        ports: vec![PortType::new_udp(detected_port)],
                        handle: Some(Box::new(SnmpProbeHandle {
                            credential: snmp_cred.clone(),
                            port: detected_port,
                        })),
                    });
                }
                Ok(None) => continue,
                Err(e) => {
                    tracing::debug!(
                        ip = %ctx.ip,
                        port = port,
                        error = %e,
                        "SNMP credential probe failed"
                    );
                }
            }
        }

        // No "public" fallback here — the daemon injects a broadcast SNMP credential
        // with community "public" into credential_mappings, so it's tried as its own
        // integration dispatch. No special-casing needed.

        Err(ProbeFailure {
            message: format!("SNMP not responding on {} with any credential", ctx.ip),
        })
    }

    async fn execute(
        &self,
        ctx: &IntegrationContext<'_>,
        host_data: &mut HostData,
    ) -> Result<(), Error> {
        // Downcast probe handle to get the working credential and port
        let handle = ctx
            .probe_handle
            .and_then(|h| h.downcast_ref::<SnmpProbeHandle>())
            .ok_or_else(|| anyhow::anyhow!("SNMP execute called without SnmpProbeHandle"))?;

        let credential = &handle.credential;
        let port = handle.port;
        let ip = ctx.ip;

        // Query system info
        let system_info = match timeout(SNMP_WALK_TIMEOUT, query_system_info(ip, credential, port))
            .await
            .unwrap_or_else(|_| Err(anyhow::anyhow!("system info query timeout")))
        {
            Ok(info)
                if info.sys_descr.is_some()
                    || info.sys_name.is_some()
                    || info.sys_object_id.is_some() =>
            {
                tracing::debug!(
                    ip = %ip,
                    sys_name = ?info.sys_name,
                    "SNMP system info retrieved"
                );
                Some(info)
            }
            Ok(_) => {
                tracing::debug!(ip = %ip, "SNMP system_info returned no data");
                None
            }
            Err(e) => {
                tracing::debug!(ip = %ip, error = %e, "SNMP system_info query failed");
                None
            }
        };

        if ctx.cancel.is_cancelled() {
            return Err(anyhow::anyhow!("Discovery was cancelled"));
        }

        // Walk interface table
        let snmp_if_entries = match timeout(SNMP_WALK_TIMEOUT, walk_if_table(ip, credential, port))
            .await
            .unwrap_or_else(|_| Err(anyhow::anyhow!("ifTable walk timeout")))
        {
            Ok(entries) => {
                tracing::debug!(ip = %ip, if_count = entries.len(), "SNMP ifTable walked");
                entries
            }
            Err(e) => {
                tracing::debug!(ip = %ip, error = %e, "SNMP ifTable walk failed");
                Vec::new()
            }
        };

        // Query LLDP neighbors
        let lldp_neighbors = match timeout(
            SNMP_WALK_TIMEOUT,
            query_lldp_neighbors(ip, credential, port),
        )
        .await
        .unwrap_or_else(|_| Err(anyhow::anyhow!("LLDP query timeout")))
        {
            Ok(neighbors) => {
                tracing::debug!(ip = %ip, count = neighbors.len(), "LLDP neighbors discovered");
                neighbors
            }
            Err(e) => {
                tracing::debug!(ip = %ip, error = %e, "LLDP query failed");
                Vec::new()
            }
        };

        // Query CDP neighbors (Cisco devices)
        let cdp_neighbors =
            match timeout(SNMP_WALK_TIMEOUT, query_cdp_neighbors(ip, credential, port))
                .await
                .unwrap_or_else(|_| Err(anyhow::anyhow!("CDP query timeout")))
            {
                Ok(neighbors) => {
                    tracing::debug!(ip = %ip, count = neighbors.len(), "CDP neighbors discovered");
                    neighbors
                }
                Err(e) => {
                    tracing::debug!(ip = %ip, error = %e, "CDP query failed");
                    Vec::new()
                }
            };

        // Query ipAddrTable for IP->ifIndex+netMask mappings
        let ip_addr_table =
            match timeout(SNMP_WALK_TIMEOUT, query_ip_addr_table(ip, credential, port)).await {
                Ok(res) => res.unwrap_or_default(),
                Err(_) => {
                    tracing::debug!(ip = %ip, "SNMP ipAddrTable walk timeout");
                    Default::default()
                }
            };

        // Query ARP table for remote host discovery
        let arp_entries =
            match timeout(SNMP_WALK_TIMEOUT, query_arp_table(ip, credential, port)).await {
                Ok(res) => res.unwrap_or_default(),
                Err(_) => {
                    tracing::debug!(ip = %ip, "SNMP ARP table walk timeout");
                    Default::default()
                }
            };
        tracing::info!(ip = %ip, count = arp_entries.len(), "ARP table entries collected");

        // Query ENTITY-MIB for hardware inventory
        let device_inventory = match timeout(
            SNMP_WALK_TIMEOUT,
            query_entity_physical(ip, credential, port),
        )
        .await
        {
            Ok(res) => res.unwrap_or(None),
            Err(_) => {
                tracing::debug!(ip = %ip, "SNMP ENTITY-MIB walk timeout");
                None
            }
        };
        tracing::info!(
            ip = %ip,
            has_inventory = device_inventory.is_some(),
            "ENTITY-MIB inventory queried"
        );

        // Query bridge FDB for MAC-to-port mappings
        let bridge_fdb =
            match timeout(SNMP_WALK_TIMEOUT, query_bridge_fdb(ip, credential, port)).await {
                Ok(res) => res.unwrap_or_default(),
                Err(_) => {
                    tracing::debug!(ip = %ip, "SNMP bridge FDB walk timeout");
                    Default::default()
                }
            };
        tracing::info!(ip = %ip, count = bridge_fdb.len(), "Bridge FDB entries collected");

        let network_id = host_data.host.base.network_id;

        // Query VLAN table for VLAN names and persist as VLAN entities
        let vlan_table =
            match timeout(SNMP_WALK_TIMEOUT, query_vlan_table(ip, credential, port)).await {
                Ok(res) => res.unwrap_or_default(),
                Err(_) => {
                    tracing::debug!(ip = %ip, "SNMP VLAN table walk timeout");
                    Default::default()
                }
            };
        let vlan_number_to_uuid: std::collections::HashMap<u16, Uuid> = if !vlan_table.is_empty() {
            tracing::info!(
                ip = %ip,
                count = vlan_table.len(),
                vlans = ?vlan_table.iter().map(|v| format!("{}={}", v.vlan_id, v.name)).collect::<Vec<_>>(),
                "VLAN table entries collected"
            );
            match ctx.ops.upsert_vlans(&vlan_table, network_id).await {
                Ok(mapping) => mapping,
                Err(e) => {
                    tracing::warn!(ip = %ip, error = %e, "Failed to upsert VLANs, VLAN IDs will not be resolved");
                    std::collections::HashMap::new()
                }
            }
        } else {
            std::collections::HashMap::new()
        };

        // Query per-port VLAN membership
        let port_vlan_membership = match timeout(
            SNMP_WALK_TIMEOUT,
            query_port_vlan_membership(ip, credential, port),
        )
        .await
        {
            Ok(res) => res.unwrap_or_default(),
            Err(_) => {
                tracing::debug!(ip = %ip, "SNMP port VLAN membership walk timeout");
                Default::default()
            }
        };
        tracing::info!(ip = %ip, count = port_vlan_membership.len(), "Port VLAN memberships collected");

        // Query local LLDP identity
        let lldp_local =
            match timeout(SNMP_WALK_TIMEOUT, query_lldp_local(ip, credential, port)).await {
                Ok(res) => res.unwrap_or(None),
                Err(_) => {
                    tracing::debug!(ip = %ip, "SNMP LLDP local identity query timeout");
                    None
                }
            };
        tracing::info!(
            ip = %ip,
            has_lldp_local = lldp_local.is_some(),
            "LLDP local identity queried"
        );

        let network_id = host_data.host.base.network_id;

        // --- Hostname enrichment: use SNMP sysName as fallback if DNS didn't provide one ---
        if let Some(ref info) = system_info
            && let Some(ref sys_name) = info.sys_name
        {
            host_data.with_hostname_fallback(sys_name.clone());
        }

        // --- MAC enrichment from ipAddrTable when ARP didn't provide one ---
        if let Some(ip_entry) = ip_addr_table.get(&ip)
            && let Some(entry) = snmp_if_entries
                .iter()
                .find(|e| e.if_index == ip_entry.if_index)
            && let Some(mac) = entry.if_phys_address
        {
            tracing::debug!(
                ip = %ip,
                if_index = ip_entry.if_index,
                mac = ?mac,
                "ipAddrTable MAC enrichment"
            );
            host_data.with_mac_for_ip(ip, mac);
        }

        // --- Enrich host fields from SNMP system info ---
        if let Some(ref info) = system_info {
            if let Some(ref v) = info.sys_descr {
                host_data.with_sys_descr(v.clone());
            }
            if let Some(ref v) = info.sys_object_id {
                host_data.with_sys_object_id(v.clone());
            }
            if let Some(ref v) = info.sys_location {
                host_data.with_sys_location(v.clone());
            }
            if let Some(ref v) = info.sys_contact {
                host_data.with_sys_contact(v.clone());
            }
            if let Some(ref v) = info.sys_name {
                host_data.with_sys_name(v.clone());
            }
        }

        // --- Set chassis_id from LLDP local identity ---
        if let Some(ref local) = lldp_local
            && let Some(chassis) =
                LldpChassisId::from_snmp(local.chassis_id_subtype, &local.chassis_id_bytes)
        {
            host_data.with_chassis_id(match &chassis {
                LldpChassisId::NetworkAddress(addr) => addr.to_string(),
                LldpChassisId::MacAddress(s)
                | LldpChassisId::ChassisComponent(s)
                | LldpChassisId::InterfaceAlias(s)
                | LldpChassisId::PortComponent(s)
                | LldpChassisId::InterfaceName(s)
                | LldpChassisId::LocallyAssigned(s) => s.clone(),
            });
        }

        // --- Add ENTITY-MIB hardware inventory ---
        if let Some(ref inventory) = device_inventory {
            if let Some(ref v) = inventory.manufacturer {
                host_data.with_manufacturer(v.clone());
            }
            if let Some(ref v) = inventory.model {
                host_data.with_model(v.clone());
            }
            if let Some(ref v) = inventory.serial_number {
                host_data.with_serial_number(v.clone());
            }
        }

        // --- Credential assignment for the working SNMP credential ---
        if let Some(cred_id) = ctx.credential_id {
            host_data.add_credential_assignment(CredentialAssignment {
                credential_id: cred_id,
                ip_address_ids: None,
            });
        }

        // --- Convert SNMP ifTable entries to Interface entities ---
        for entry in &snmp_if_entries {
            let interface = convert_snmp_if_entry(
                entry,
                network_id,
                &lldp_neighbors,
                &cdp_neighbors,
                &bridge_fdb,
                &port_vlan_membership,
                &vlan_number_to_uuid,
            );
            host_data.add_if_entry(interface);
        }

        // --- Discover remote subnets from ipAddrTable ---
        let scanning_subnet = ctx.scanning_subnet;
        let mut discovered_subnets: Vec<Subnet> = Vec::new();

        for (entry_ip, entry) in &ip_addr_table {
            let mask = match entry.net_mask {
                Some(m) => m,
                None => continue,
            };

            // Only handle IPv4
            let (entry_ipv4, mask_ipv4) = match (entry_ip, mask) {
                (IpAddr::V4(eip), IpAddr::V4(mip)) => (*eip, mip),
                _ => continue,
            };

            // Skip loopback, link-local
            let octets = entry_ipv4.octets();
            if octets[0] == 127 || (octets[0] == 169 && octets[1] == 254) {
                continue;
            }

            // Skip /32 and /0
            let mask_octets = mask_ipv4.octets();
            let mask_u32 = u32::from_be_bytes(mask_octets);
            if mask_u32 == 0xFFFFFFFF || mask_u32 == 0 {
                continue;
            }

            // Build network from IP + mask
            let ipv4_network = match ipnetwork::Ipv4Network::with_netmask(entry_ipv4, mask_ipv4) {
                Ok(n) => n,
                Err(_) => continue,
            };
            let ip_network = ipnetwork::IpNetwork::V4(ipv4_network);

            // Skip if this is the current scanning subnet
            if let Some(subnet) = scanning_subnet {
                let new_cidr_str = format!("{}/{}", ipv4_network.network(), ipv4_network.prefix());
                if new_cidr_str == subnet.base.cidr.to_string() {
                    continue;
                }
            }

            // Get interface name for subnet typing
            let if_name = snmp_if_entries
                .iter()
                .find(|e| e.if_index == entry.if_index)
                .and_then(|e| e.if_name.clone())
                .unwrap_or_default();

            if let Some(new_subnet) = Subnet::from_discovery(if_name, &ip_network, network_id) {
                tracing::info!(
                    ip = %ip,
                    cidr = %new_subnet.base.cidr,
                    "Discovered remote subnet via ipAddrTable"
                );

                match ctx.ops.create_subnet(&new_subnet, ctx.cancel).await {
                    Ok(created_subnet) => {
                        // Build an interface for the host on this subnet
                        let if_mac = snmp_if_entries
                            .iter()
                            .find(|e| e.if_index == entry.if_index)
                            .and_then(|e| e.if_phys_address);

                        host_data.add_ip_address(IPAddress::new(IPAddressBase {
                            network_id,
                            host_id: Uuid::nil(),
                            name: None,
                            subnet_id: created_subnet.id,
                            ip_address: *entry_ip,
                            mac_address: if_mac,
                            position: 0,
                        }));

                        discovered_subnets.push(created_subnet);
                    }
                    Err(e) => {
                        tracing::warn!(
                            ip = %ip,
                            cidr = %new_subnet.base.cidr,
                            error = %e,
                            "Failed to create discovered subnet"
                        );
                    }
                }
            }
        }

        // --- Create loopback interface if this host has a SOFTWARE_LOOPBACK ifEntry ---
        let has_loopback_if_entry = snmp_if_entries
            .iter()
            .any(|e| e.if_type == Some(if_type::SOFTWARE_LOOPBACK));
        if has_loopback_if_entry {
            let loopback_subnet = Subnet::from_discovery(
                "lo".to_string(),
                &ipnetwork::IpNetwork::V4(
                    ipnetwork::Ipv4Network::new(std::net::Ipv4Addr::new(127, 0, 0, 1), 8).unwrap(),
                ),
                network_id,
            );
            if let Some(loopback_subnet) = loopback_subnet {
                match ctx.ops.create_subnet(&loopback_subnet, ctx.cancel).await {
                    Ok(created_loopback) => {
                        host_data.add_ip_address(IPAddress::new(IPAddressBase {
                            network_id,
                            host_id: Uuid::nil(),
                            name: Some("lo".to_string()),
                            subnet_id: created_loopback.id,
                            ip_address: std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
                            mac_address: None,
                            position: 0,
                        }));
                    }
                    Err(e) => {
                        tracing::debug!(
                            error = %e,
                            "Failed to create loopback subnet for SNMP host"
                        );
                    }
                }
            }
        }

        // --- Discover remote hosts from ARP table ---
        // Only create hosts for ARP entries on SNMP-discovered remote subnets
        for arp_entry in &arp_entries {
            // Skip entries on the current scanning subnet
            if let Some(subnet) = scanning_subnet
                && subnet.base.cidr.contains(&arp_entry.ip_address)
            {
                continue;
            }

            // Find matching SNMP-discovered subnet
            let matching_subnet = discovered_subnets
                .iter()
                .find(|s| s.base.cidr.contains(&arp_entry.ip_address));

            if let Some(remote_subnet) = matching_subnet {
                let arp_interface = IPAddress::new(IPAddressBase {
                    network_id,
                    host_id: Uuid::nil(),
                    name: None,
                    subnet_id: remote_subnet.id,
                    ip_address: arp_entry.ip_address,
                    mac_address: Some(arp_entry.mac_address),
                    position: 0,
                });

                let arp_host = Host::new(HostBase {
                    network_id,
                    source: EntitySource::Discovery,
                    ..Default::default()
                });

                tracing::info!(
                    ip = %arp_entry.ip_address,
                    mac = %arp_entry.mac_address,
                    subnet = %remote_subnet.base.cidr,
                    "Discovered remote host via ARP table"
                );

                if let Err(e) = ctx
                    .ops
                    .create_host(
                        arp_host,
                        vec![arp_interface],
                        vec![],
                        vec![],
                        vec![],
                        vec![],
                        ctx.cancel,
                    )
                    .await
                {
                    tracing::debug!(
                        ip = %arp_entry.ip_address,
                        error = %e,
                        "Failed to create ARP-discovered host"
                    );
                }
            }
        }

        Ok(())
    }
}

/// Convert SNMP ifTable entry to Interface entity with LLDP/CDP/FDB neighbor data.
/// Uses Uuid::nil() for host_id as placeholder - server will set correct host_id.
fn convert_snmp_if_entry(
    entry: &IfTableEntry,
    network_id: Uuid,
    lldp_neighbors: &[LldpNeighbor],
    cdp_neighbors: &[CdpNeighbor],
    bridge_fdb: &[BridgeFdbEntry],
    port_vlan_membership: &[PortVlanMembership],
    vlan_number_to_uuid: &std::collections::HashMap<u16, Uuid>,
) -> Interface {
    // Find LLDP neighbor data for this port (match by local_port_index == if_index)
    let lldp_neighbor = lldp_neighbors
        .iter()
        .find(|n| n.local_port_index == entry.if_index);

    // Find CDP neighbor data for this port
    let cdp_neighbor = cdp_neighbors
        .iter()
        .find(|n| n.local_port_index == entry.if_index);

    // Convert LLDP chassis ID using subtype + raw bytes via from_snmp()
    let lldp_chassis_id = lldp_neighbor.and_then(|n| {
        let subtype = n.remote_chassis_id_subtype?;
        let bytes = n.remote_chassis_id_bytes.as_ref()?;
        LldpChassisId::from_snmp(subtype, bytes)
    });

    // Convert LLDP port ID using subtype + raw bytes via from_snmp()
    let lldp_port_id = lldp_neighbor.and_then(|n| {
        let subtype = n.remote_port_id_subtype?;
        let bytes = n.remote_port_id_bytes.as_ref()?;
        LldpPortId::from_snmp(subtype, bytes)
    });

    // Find VLAN membership for this port
    let vlan_membership = port_vlan_membership
        .iter()
        .find(|m| m.if_index == entry.if_index);

    // Collect learned MACs from bridge FDB for this port.
    // Single-MAC ports are used for neighbor resolution server-side;
    // multi-MAC ports indicate uplinks where LLDP/CDP is the better source
    // for direct neighbor identification.
    let fdb_macs: Vec<String> = bridge_fdb
        .iter()
        .filter(|fdb| fdb.if_index == Some(entry.if_index) && fdb.status == 3)
        .map(|fdb| fdb.mac_address.to_string())
        .collect();

    Interface::new(InterfaceBase {
        host_id: Uuid::nil(), // Placeholder - server will set correct host_id
        network_id,
        if_index: entry.if_index,
        if_descr: entry.if_descr.clone().unwrap_or_default(),
        if_name: entry.if_name.clone(),
        if_alias: entry.if_alias.clone(),
        if_type: entry.if_type.unwrap_or(1), // 1 = "other"
        speed_bps: entry.if_speed.map(|s| s as i64),
        admin_status: IfAdminStatus::from(entry.if_admin_status.unwrap_or(1)),
        oper_status: IfOperStatus::from(entry.if_oper_status.unwrap_or(1)),
        mac_address: entry.if_phys_address, // MAC from SNMP ifPhysAddress
        ip_address_id: None,                // Linked server-side via MAC matching
        neighbor: None,                     // Resolved server-side from LLDP/CDP data
        // LLDP raw data
        lldp_chassis_id,
        lldp_port_id,
        lldp_sys_name: lldp_neighbor.and_then(|n| n.remote_sys_name.clone()),
        lldp_port_desc: lldp_neighbor.and_then(|n| n.remote_port_desc.clone()),
        lldp_mgmt_addr: lldp_neighbor.and_then(|n| n.remote_mgmt_addr),
        lldp_sys_desc: lldp_neighbor.and_then(|n| n.remote_sys_desc.clone()),
        // CDP raw data
        cdp_device_id: cdp_neighbor.and_then(|n| n.remote_device_id.clone()),
        cdp_port_id: cdp_neighbor.and_then(|n| n.remote_port_id.clone()),
        cdp_platform: cdp_neighbor.and_then(|n| n.remote_platform.clone()),
        cdp_address: cdp_neighbor.and_then(|n| n.remote_address),
        // Bridge FDB data
        fdb_macs: if fdb_macs.is_empty() {
            None
        } else {
            Some(fdb_macs)
        },
        // VLAN data: resolved to entity UUIDs by caller via vlan_number_to_uuid mapping
        native_vlan_id: vlan_membership
            .and_then(|m| m.native_vlan)
            .and_then(|vid| vlan_number_to_uuid.get(&vid).copied()),
        vlan_ids: vlan_membership
            .map(|m| {
                m.tagged_vlans
                    .iter()
                    .filter_map(|vid| vlan_number_to_uuid.get(vid).copied())
                    .collect::<Vec<_>>()
            })
            .filter(|v| !v.is_empty()),
    })
}

/// Perform a complete SNMP poll of a device.
/// Returns system info, interface table, and neighbor information.
#[allow(dead_code)]
pub async fn poll_device(
    ip: IpAddr,
    credential: &SnmpQueryCredential,
    port: u16,
) -> Result<(
    SystemInfo,
    Vec<IfTableEntry>,
    Vec<LldpNeighbor>,
    Vec<CdpNeighbor>,
)> {
    debug!("Starting SNMP poll of {}", ip);

    let system_info = timeout(SNMP_WALK_TIMEOUT, query_system_info(ip, credential, port))
        .await
        .map_err(|_| anyhow::anyhow!("System info query timeout"))??;

    let interfaces = timeout(SNMP_WALK_TIMEOUT, walk_if_table(ip, credential, port))
        .await
        .map_err(|_| anyhow::anyhow!("ifTable walk timeout"))?
        .unwrap_or_default();

    let lldp_neighbors = timeout(
        SNMP_WALK_TIMEOUT,
        query_lldp_neighbors(ip, credential, port),
    )
    .await
    .unwrap_or(Ok(vec![]))
    .unwrap_or_default();

    let cdp_neighbors = timeout(SNMP_WALK_TIMEOUT, query_cdp_neighbors(ip, credential, port))
        .await
        .unwrap_or(Ok(vec![]))
        .unwrap_or_default();

    debug!(
        "SNMP poll of {} complete: {} ip_addresses, {} LLDP neighbors, {} CDP neighbors",
        ip,
        interfaces.len(),
        lldp_neighbors.len(),
        cdp_neighbors.len()
    );

    Ok((system_info, interfaces, lldp_neighbors, cdp_neighbors))
}

#[cfg(test)]
mod tests {
    use super::values::{value_to_i32, value_to_mac, value_to_string};
    use snmp2::Value;

    #[test]
    fn test_value_to_string() {
        let value = Value::OctetString(b"test string");
        assert_eq!(value_to_string(&value), Some("test string".to_string()));
    }

    #[test]
    fn test_value_to_i32() {
        let value = Value::Integer(42);
        assert_eq!(value_to_i32(&value), Some(42));
    }

    #[test]
    fn test_value_to_mac() {
        let mac_bytes: [u8; 6] = [0xDE, 0xAD, 0xBE, 0xEF, 0x12, 0x34];
        let value = Value::OctetString(&mac_bytes);
        let mac = value_to_mac(&value).unwrap();
        assert_eq!(mac.bytes(), [0xDE, 0xAD, 0xBE, 0xEF, 0x12, 0x34]);
    }

    #[test]
    fn test_convert_snmp_if_entry_with_vlan_data() {
        use super::convert_snmp_if_entry;
        use super::types::{IfTableEntry, PortVlanMembership};
        use uuid::Uuid;

        let entry = IfTableEntry {
            if_index: 5,
            if_descr: Some("GigabitEthernet0/5".to_string()),
            ..Default::default()
        };

        let membership = vec![
            PortVlanMembership {
                if_index: 5,
                native_vlan: Some(10),
                tagged_vlans: vec![20, 30],
            },
            PortVlanMembership {
                if_index: 7,
                native_vlan: Some(20),
                tagged_vlans: vec![],
            },
        ];

        let result = convert_snmp_if_entry(
            &entry,
            Uuid::nil(),
            &[],
            &[],
            &[],
            &membership,
            &std::collections::HashMap::new(),
        );

        assert_eq!(result.base.native_vlan_id, None);
        assert_eq!(result.base.vlan_ids, None);
    }

    #[test]
    fn test_convert_snmp_if_entry_no_vlan_data() {
        use super::convert_snmp_if_entry;
        use super::types::IfTableEntry;
        use uuid::Uuid;

        let entry = IfTableEntry {
            if_index: 3,
            if_descr: Some("Loopback0".to_string()),
            ..Default::default()
        };

        let result = convert_snmp_if_entry(
            &entry,
            Uuid::nil(),
            &[],
            &[],
            &[],
            &[],
            &std::collections::HashMap::new(),
        );

        assert_eq!(result.base.native_vlan_id, None);
        assert_eq!(result.base.vlan_ids, None);
    }

    #[test]
    fn test_convert_snmp_if_entry_empty_tagged_vlans() {
        use super::convert_snmp_if_entry;
        use super::types::{IfTableEntry, PortVlanMembership};
        use uuid::Uuid;

        let entry = IfTableEntry {
            if_index: 1,
            if_descr: Some("FastEthernet0/1".to_string()),
            ..Default::default()
        };

        // Access port: native VLAN only, no tagged VLANs
        let membership = vec![PortVlanMembership {
            if_index: 1,
            native_vlan: Some(10),
            tagged_vlans: vec![],
        }];

        let result = convert_snmp_if_entry(
            &entry,
            Uuid::nil(),
            &[],
            &[],
            &[],
            &membership,
            &std::collections::HashMap::new(),
        );

        assert_eq!(result.base.native_vlan_id, None);
        // Empty tagged_vlans should be stored as None (filtered)
        assert_eq!(result.base.vlan_ids, None);
    }
}
