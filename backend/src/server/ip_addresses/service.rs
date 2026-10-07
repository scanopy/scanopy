use uuid::Uuid;

use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    ip_addresses::r#impl::base::IPAddress,
    shared::{
        events::bus::EventBus,
        services::traits::{ChildCrudService, CrudService, EventBusService},
        storage::{filter::StorableFilter, generic::GenericPostgresStorage, traits::Storage},
        types::api::ValidationError,
    },
    tags::entity_tags::EntityTagService,
};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

pub struct IPAddressService {
    storage: Arc<GenericPostgresStorage<IPAddress>>,
    event_bus: Arc<EventBus>,
}

impl EventBusService<IPAddress> for IPAddressService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_site_id(&self, entity: &IPAddress) -> Option<Uuid> {
        Some(entity.base.site_id)
    }

    fn get_organization_id(&self, _entity: &IPAddress) -> Option<Uuid> {
        None
    }
}

impl CrudService<IPAddress> for IPAddressService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<IPAddress>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        None
    }
}

impl ChildCrudService<IPAddress> for IPAddressService {}

impl IPAddressService {
    pub fn new(storage: Arc<GenericPostgresStorage<IPAddress>>, event_bus: Arc<EventBus>) -> Self {
        Self { storage, event_bus }
    }

    /// Get all IP addresses for a specific host, ordered by position
    pub async fn get_for_host(&self, host_id: &Uuid) -> Result<Vec<IPAddress>> {
        // SCD2: only live rows. Closed historical copies are out of scope
        // for current-state reads.
        let filter = StorableFilter::<IPAddress>::new_from_host_ids(&[*host_id]).live();
        self.storage.get_all_ordered(filter, "position ASC").await
    }

    /// Get IP addresses for multiple hosts, ordered by position within each host.
    /// `at = None` reads live rows; `Some(t)` reads SCD2 state as of `t`
    /// (snapshot-view hydration). Reconciliation natural-key matching passes
    /// `None` so historical copies never match.
    pub async fn get_for_hosts(
        &self,
        host_ids: &[Uuid],
        at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<HashMap<Uuid, Vec<IPAddress>>> {
        if host_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let filter = StorableFilter::<IPAddress>::new_from_host_ids(host_ids).live_or_as_of(at);
        let ip_addresses = self.storage.get_all_ordered(filter, "position ASC").await?;

        let mut result: HashMap<Uuid, Vec<IPAddress>> = HashMap::new();
        for ip_address in ip_addresses {
            result
                .entry(ip_address.base.host_id)
                .or_default()
                .push(ip_address);
        }

        Ok(result)
    }

    /// Get all IP addresses for a specific subnet.
    /// SCD2: live rows only. The sole caller is discovery subnet↔VLAN
    /// reconciliation, which must not let closed historical copies (from prior
    /// snapshots) resurrect stale native-VLAN links.
    pub async fn get_for_subnet(&self, subnet_id: &Uuid) -> Result<Vec<IPAddress>> {
        let filter = StorableFilter::<IPAddress>::new_from_subnet_id(subnet_id).live();
        self.storage.get_all(filter).await
    }

    /// How many distinct addresses each subnet on these sites holds, live or as of `at`.
    ///
    /// Distinct by address rather than by row: an HA virtual IP sits on both peers as two rows
    /// and occupies one address. Subnets holding none are absent from the map.
    pub async fn used_by_subnet(
        &self,
        site_ids: &[Uuid],
        at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<HashMap<Uuid, u64>> {
        let filter = StorableFilter::<IPAddress>::new_from_site_ids(site_ids).live_or_as_of(at);
        let counts = self
            .count_distinct_by_group(filter, "ip_addresses.subnet_id", "ip_addresses.ip_address")
            .await?;
        Ok(counts
            .into_iter()
            .filter_map(|g| Some((g.value?.parse::<Uuid>().ok()?, g.count)))
            .collect())
    }

    // =========================================================================
    // Position management helpers (for direct API operations)
    // =========================================================================

    /// Get the next available position for a new IP address on a host.
    /// Returns the count of existing IP addresses (which becomes the next position).
    pub async fn get_next_position_for_host(&self, host_id: &Uuid) -> Result<i32> {
        let existing = self.get_for_host(host_id).await?;
        Ok(existing.len() as i32)
    }

    /// Validate that a position is valid for an IP address update.
    /// Position must be within range [0, count-1] and not conflict with other IP addresses.
    pub async fn validate_position_for_update(
        &self,
        ip_address_id: &Uuid,
        host_id: &Uuid,
        new_position: i32,
    ) -> Result<()> {
        let all_ip_addresses = self.get_for_host(host_id).await?;
        let max_position = (all_ip_addresses.len() as i32) - 1;

        if new_position < 0 || new_position > max_position {
            return Err(ValidationError::new(format!(
                "IP address position {} is out of range. Valid positions are 0 to {}.",
                new_position, max_position
            ))
            .into());
        }

        // Check for duplicate position (excluding self)
        if all_ip_addresses
            .iter()
            .any(|i| i.id != *ip_address_id && i.base.position == new_position)
        {
            return Err(ValidationError::new(format!(
                "IP address position {} is already used by another IP address.",
                new_position
            ))
            .into());
        }

        Ok(())
    }

    /// Renumber all IP addresses for a host to ensure sequential positions (0, 1, 2, ...).
    /// Called after deleting IP address(es) to close gaps.
    pub async fn renumber_ip_addresses_for_host(
        &self,
        host_id: &Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<()> {
        let mut ip_addresses = self.get_for_host(host_id).await?;

        // IP addresses are already ordered by position, so just assign sequential positions
        let mut needs_update = false;
        for (i, ip) in ip_addresses.iter_mut().enumerate() {
            let expected_position = i as i32;
            if ip.base.position != expected_position {
                needs_update = true;
                ip.base.position = expected_position;
            }
        }

        // Only update if there are gaps to close
        if needs_update {
            for ip in &mut ip_addresses {
                self.update(ip, authentication.clone()).await?;
            }
        }

        Ok(())
    }
}
