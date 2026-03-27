//! LLDP resolution trait and implementation.
//!
//! This module provides:
//! - `LldpResolver` trait for LLDP neighbor resolution database lookups
//! - `LldpResolverImpl` production implementation using database services

use std::net::IpAddr;
use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::server::{
    hosts::r#impl::base::Host,
    if_entries::{r#impl::base::IfEntry, service::IfEntryService},
    interfaces::{r#impl::base::Interface, service::InterfaceService},
    shared::{
        services::traits::CrudService,
        storage::{filter::StorableFilter, generic::GenericPostgresStorage, traits::Storage},
    },
};

/// Trait for LLDP resolution database lookups.
///
/// This trait abstracts database access for LLDP resolution, enabling:
/// - Dependency injection in the resolution methods on enums
/// - Easier testing with mock implementations
/// - Clean separation between LLDP types and database layer
#[async_trait]
pub trait LldpResolver: Send + Sync {
    /// Find host by MAC address (via interfaces.mac_address).
    async fn find_host_by_mac(&self, mac: &str, network_id: Uuid) -> Option<Uuid>;

    /// Find host by IP address (via interfaces table).
    async fn find_host_by_ip(&self, ip: &IpAddr, network_id: Uuid) -> Option<Uuid>;

    /// Find host by interface name (via if_entries.if_descr).
    async fn find_host_by_if_name(&self, name: &str, network_id: Uuid) -> Option<Uuid>;

    /// Find host by chassis_id field on hosts table.
    async fn find_host_by_chassis_id(&self, chassis_id: &str, network_id: Uuid) -> Option<Uuid>;

    /// Find host by sys_name field on hosts table (used for CDP resolution).
    async fn find_host_by_sys_name(&self, sys_name: &str, network_id: Uuid) -> Option<Uuid>;

    /// Find if_entry by MAC address.
    async fn find_if_entry_by_mac(&self, mac: &str, host_id: Uuid) -> Option<Uuid>;

    /// Find if_entry by name (if_descr or if_alias).
    async fn find_if_entry_by_name(&self, name: &str, host_id: Uuid) -> Option<Uuid>;

    /// Find if_entry by IP address (via interface_id FK).
    async fn find_if_entry_by_ip(&self, ip: &IpAddr, host_id: Uuid) -> Option<Uuid>;
}

/// Production implementation of `LldpResolver`.
///
/// Uses database services to look up entities for LLDP neighbor resolution.
pub struct LldpResolverImpl {
    if_entry_service: Arc<IfEntryService>,
    interface_service: Arc<InterfaceService>,
    host_storage: Arc<GenericPostgresStorage<Host>>,
}

impl LldpResolverImpl {
    pub fn new(
        if_entry_service: Arc<IfEntryService>,
        interface_service: Arc<InterfaceService>,
        host_storage: Arc<GenericPostgresStorage<Host>>,
    ) -> Self {
        Self {
            if_entry_service,
            interface_service,
            host_storage,
        }
    }
}

#[async_trait]
impl LldpResolver for LldpResolverImpl {
    async fn find_host_by_mac(&self, mac: &str, network_id: Uuid) -> Option<Uuid> {
        let mac_addr: mac_address::MacAddress = mac.parse().ok()?;

        // Primary: Interface MAC (populated from ARP or SNMP ipAddrTable enrichment)
        let filter =
            StorableFilter::<Interface>::new_from_network_ids(&[network_id]).mac_address(&mac_addr);
        if let Ok(Some(interface)) = self.interface_service.get_one(filter).await {
            return Some(interface.base.host_id);
        }

        // Fallback: IfEntry MAC (from SNMP ifPhysAddress, always present for SNMP hosts)
        let filter =
            StorableFilter::<IfEntry>::new_from_network_ids(&[network_id]).mac_address(&mac_addr);
        let entry = self.if_entry_service.get_one(filter).await.ok()??;
        Some(entry.base.host_id)
    }

    async fn find_host_by_ip(&self, ip: &IpAddr, network_id: Uuid) -> Option<Uuid> {
        let filter =
            StorableFilter::<Interface>::new_from_network_ids(&[network_id]).ip_address(*ip);
        let interface = self.interface_service.get_one(filter).await.ok()??;

        Some(interface.base.host_id)
    }

    async fn find_host_by_if_name(&self, name: &str, network_id: Uuid) -> Option<Uuid> {
        let filter = StorableFilter::<IfEntry>::new_from_network_ids(&[network_id]).if_descr(name);
        let entry = self.if_entry_service.get_one(filter).await.ok()??;

        Some(entry.base.host_id)
    }

    async fn find_host_by_chassis_id(&self, chassis_id: &str, network_id: Uuid) -> Option<Uuid> {
        let filter =
            StorableFilter::<Host>::new_from_network_ids(&[network_id]).chassis_id(chassis_id);
        let host = self.host_storage.get_one(filter).await.ok()??;

        Some(host.id)
    }

    async fn find_host_by_sys_name(&self, sys_name: &str, network_id: Uuid) -> Option<Uuid> {
        let filter = StorableFilter::<Host>::new_from_network_ids(&[network_id]).sys_name(sys_name);
        let host = self.host_storage.get_one(filter).await.ok()??;

        Some(host.id)
    }

    async fn find_if_entry_by_mac(&self, mac: &str, host_id: Uuid) -> Option<Uuid> {
        // Parse MAC string to MacAddress type
        let mac_addr: mac_address::MacAddress = mac.parse().ok()?;

        // Find if_entry with this MAC on the specified host
        let filter =
            StorableFilter::<IfEntry>::new_from_host_ids(&[host_id]).mac_address(&mac_addr);
        let entry = self.if_entry_service.get_one(filter).await.ok()??;

        Some(entry.id)
    }

    async fn find_if_entry_by_name(&self, name: &str, host_id: Uuid) -> Option<Uuid> {
        // Try if_descr first (long name: "GigabitEthernet1/0/1")
        let filter = StorableFilter::<IfEntry>::new_from_host_ids(&[host_id]).if_descr(name);
        if let Ok(Some(entry)) = self.if_entry_service.get_one(filter).await {
            return Some(entry.id);
        }
        // Try if_name (short name: "Gi1/0/1")
        let filter = StorableFilter::<IfEntry>::new_from_host_ids(&[host_id]).if_name(name);
        let entry = self.if_entry_service.get_one(filter).await.ok()??;
        Some(entry.id)
    }

    async fn find_if_entry_by_ip(&self, ip: &IpAddr, host_id: Uuid) -> Option<Uuid> {
        // Find interface with this IP on the target host
        let filter = StorableFilter::<Interface>::new_from_host_ids(&[host_id]).ip_address(*ip);
        let interface = self.interface_service.get_one(filter).await.ok()??;

        // Find IfEntry linked to this interface via interface_id FK
        let filter = StorableFilter::<IfEntry>::new_from_interface_id(&interface.id);
        let entry = self.if_entry_service.get_one(filter).await.ok()??;

        Some(entry.id)
    }
}
