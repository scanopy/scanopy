use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};
use tokio::sync::RwLock;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::server::interfaces::r#impl::base::InterfaceDataComplete;
use crate::{
    daemon::runtime::state::BufferedEntities,
    server::{
        daemons::r#impl::api::ScannedEntityIds,
        hosts::r#impl::{api::DiscoveryHostRequest, api::HostResponse, base::Host},
        subnets::r#impl::base::Subnet,
    },
};

/// Entity state in the buffer - tracks lifecycle from discovery to server confirmation.
#[derive(Clone, Debug)]
pub enum BufferedEntity<T> {
    /// Discovered by daemon, not yet confirmed by server.
    Pending(T),
    /// Confirmed by server with actual data (may have different ID after deduplication).
    Created { pending_id: Uuid, actual: T },
}

impl<T> BufferedEntity<T> {
    pub fn is_pending(&self) -> bool {
        matches!(self, BufferedEntity::Pending(_))
    }

    pub fn is_created(&self) -> bool {
        matches!(self, BufferedEntity::Created { .. })
    }

    pub fn get_data(&self) -> &T {
        match self {
            BufferedEntity::Pending(t) => t,
            BufferedEntity::Created { actual, .. } => actual,
        }
    }
}

/// Thread-safe buffer for accumulating discovered entities with lifecycle tracking.
///
/// In both modes, discovery adds entities to this buffer. The flush mechanism differs:
/// - **DaemonPoll**: Entities are immediately sent to server and marked as Created
/// - **ServerPoll**: Server polls pending entities and responds with Created confirmations
pub struct EntityBuffer {
    /// Subnets keyed by daemon-generated ID for lookup
    subnets: Arc<RwLock<HashMap<Uuid, BufferedEntity<Subnet>>>>,
    /// Hosts keyed by daemon-generated ID
    hosts: Arc<RwLock<HashMap<Uuid, BufferedEntity<DiscoveryHostRequest>>>>,
    /// Canonical (server-assigned) VLAN IDs scanned during this session.
    /// VLAN upserts return server-side IDs synchronously, so there's no
    /// Pending → Created lifecycle — push the IDs directly after the upsert
    /// call returns. Cleared by `clear_all` at session end.
    vlans: Arc<RwLock<HashSet<Uuid>>>,
}

impl EntityBuffer {
    pub fn new() -> Self {
        Self {
            subnets: Arc::new(RwLock::new(HashMap::new())),
            hosts: Arc::new(RwLock::new(HashMap::new())),
            vlans: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    // ========================================================================
    // Subnet methods
    // ========================================================================

    /// Add a discovered subnet (pending state).
    pub async fn push_subnet(&self, subnet: Subnet) {
        let mut subnets = self.subnets.write().await;
        subnets.insert(subnet.id, BufferedEntity::Pending(subnet));
    }

    /// Mark subnet as created with actual server data.
    /// Returns the ID mapping if it changed (pending_id, actual_id).
    pub async fn mark_subnet_created(
        &self,
        pending_id: Uuid,
        actual: Subnet,
    ) -> Option<(Uuid, Uuid)> {
        let mut subnets = self.subnets.write().await;
        if let Some(entry) = subnets.get_mut(&pending_id) {
            let id_changed = pending_id != actual.id;
            *entry = BufferedEntity::Created {
                pending_id,
                actual: actual.clone(),
            };
            if id_changed {
                return Some((pending_id, actual.id));
            }
        }
        None
    }

    /// Get a subnet by its pending (daemon-generated) ID.
    /// Returns the actual server data if created, or pending data as fallback.
    pub async fn get_subnet(&self, pending_id: &Uuid) -> Option<Subnet> {
        let subnets = self.subnets.read().await;
        subnets.get(pending_id).map(|e| e.get_data().clone())
    }

    /// Wait for a subnet to be confirmed by server (with timeout).
    /// Returns None if timeout expires or cancellation is signaled before confirmation.
    pub async fn await_subnet(
        &self,
        pending_id: &Uuid,
        timeout: Duration,
        cancel: &CancellationToken,
    ) -> Option<Subnet> {
        let deadline = Instant::now() + timeout;
        loop {
            // Check cancellation first - allows quick exit when discovery is cancelled
            if cancel.is_cancelled() {
                return None;
            }
            {
                let subnets = self.subnets.read().await;
                if let Some(entry) = subnets.get(pending_id)
                    && entry.is_created()
                {
                    return Some(entry.get_data().clone());
                }
            }
            if Instant::now() > deadline {
                return None;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    // ========================================================================
    // Host methods
    // ========================================================================

    /// Add a discovered host with its children (ip_addresses, ports, services).
    ///
    /// If a host with the same ID already exists and is pending, merges children
    /// (ip_addresses, ports, services, interfaces) into the existing entry.
    /// This is critical for Docker discovery where all containers share the daemon's host_id.
    pub async fn push_host(&self, host: DiscoveryHostRequest) {
        let mut hosts = self.hosts.write().await;

        match hosts.get_mut(&host.host.id) {
            Some(BufferedEntity::Pending(existing)) => {
                // Merge children into existing pending entry
                existing.ip_addresses.extend(host.ip_addresses);
                existing.ports.extend(host.ports);
                existing.services.extend(host.services);
                existing.interfaces.extend(host.interfaces);
                existing
                    .host
                    .base
                    .credential_assignments
                    .extend(host.host.base.credential_assignments);
            }
            Some(BufferedEntity::Created { .. }) | None => {
                // No existing pending entry - insert new one
                hosts.insert(host.host.id, BufferedEntity::Pending(host));
            }
        }
    }

    /// Mark host as created with actual server data.
    /// Accepts HostResponse (which includes children) and extracts the Host.
    pub async fn mark_host_created(&self, pending_id: Uuid, actual: HostResponse) {
        let mut hosts = self.hosts.write().await;
        if let Some(entry) = hosts.get_mut(&pending_id) {
            // Update the host in the request with the actual server data
            // Convert HostResponse to Host using the to_host() method
            if let BufferedEntity::Pending(_) = entry {
                let updated_req = DiscoveryHostRequest {
                    host: actual.to_host(),
                    ip_addresses: actual.ip_addresses,
                    ports: actual.ports,
                    services: actual.services,
                    interfaces: actual.interfaces,
                    subnets: vec![],
                    interfaces_complete: true,
                    interface_data_complete: InterfaceDataComplete::default(),
                    // This daemon is this build; it submits the current shape by construction.
                    superseded_wire_shape: false,
                };
                *entry = BufferedEntity::Created {
                    pending_id,
                    actual: updated_req,
                };
            }
        }
    }

    /// Get a host by its pending (daemon-generated) ID.
    pub async fn get_host(&self, pending_id: &Uuid) -> Option<DiscoveryHostRequest> {
        let hosts = self.hosts.read().await;
        hosts.get(pending_id).map(|e| e.get_data().clone())
    }

    /// Wait for a host to be confirmed by server (with timeout).
    /// Returns None if timeout expires or cancellation is signaled before confirmation.
    pub async fn await_host(
        &self,
        pending_id: &Uuid,
        timeout: Duration,
        cancel: &CancellationToken,
    ) -> Option<Host> {
        let deadline = Instant::now() + timeout;
        loop {
            // Check cancellation first - allows quick exit when discovery is cancelled
            if cancel.is_cancelled() {
                return None;
            }
            {
                let hosts = self.hosts.read().await;
                if let Some(entry) = hosts.get(pending_id)
                    && entry.is_created()
                {
                    return Some(entry.get_data().host.clone());
                }
            }
            if Instant::now() > deadline {
                return None;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    // ========================================================================
    // Polling methods (for ServerPoll mode)
    // ========================================================================

    /// Get all pending entities for sending to server (non-destructive).
    /// Entities remain in buffer until the discovery session ends.
    ///
    /// This is used by ServerPoll mode where:
    /// 1. Server polls daemon → get_pending() returns pending entities
    /// 2. Server processes entities → sends confirmation back
    /// 3. Daemon receives confirmation → mark_*_created() updates state
    /// 4. Session ends → clear_all() removes all entries
    pub async fn get_pending(&self) -> BufferedEntities {
        let hosts = {
            let hosts = self.hosts.read().await;
            hosts
                .values()
                .filter(|e| e.is_pending())
                .map(|e| e.get_data().clone())
                .collect()
        };

        let subnets = {
            let subnets = self.subnets.read().await;
            subnets
                .values()
                .filter(|e| e.is_pending())
                .map(|e| e.get_data().clone())
                .collect()
        };

        BufferedEntities { hosts, subnets }
    }

    // ========================================================================
    // VLAN methods
    // ========================================================================

    /// Record canonical VLAN IDs returned by the server's VLAN upsert endpoint.
    /// Daemon collects these so `get_scanned_ids` can include vlan_ids in the
    /// terminal payload's `ScannedEntityIds`.
    pub async fn push_scanned_vlans(&self, ids: impl IntoIterator<Item = Uuid>) {
        let mut vlans = self.vlans.write().await;
        vlans.extend(ids);
    }

    /// Collect canonical IDs of every entity confirmed (`Created`) by the
    /// server during this session. Called at terminal phase so the result
    /// rides the terminal `DiscoveryUpdatePayload.scanned` field over the
    /// wire to the server, where per-entity FK-update subscribers consume
    /// it via the in-memory `EntityOperation::Created` event scope.
    ///
    /// Notes:
    /// - host children (ip_addresses, ports, services, interfaces) are
    ///   reachable nested inside each `Created` host entry; bindings are
    ///   nested one level further inside `service.base.bindings`.
    /// - `subnet_vlan_ids` is left empty: subnet_vlan rows are created
    ///   server-side from interface SNMP vlan info; the daemon never sees
    ///   their canonical IDs. The SubnetVlan FK-update subscriber no-ops
    ///   for now; server-side enrichment can fill this in later.
    pub async fn get_scanned_ids(&self) -> ScannedEntityIds {
        let mut out = ScannedEntityIds::default();

        {
            let subnets = self.subnets.read().await;
            for entry in subnets.values() {
                if let BufferedEntity::Created { actual, .. } = entry {
                    out.subnet_ids.push(actual.id);
                }
            }
        }

        {
            let hosts = self.hosts.read().await;
            for entry in hosts.values() {
                if let BufferedEntity::Created { actual, .. } = entry {
                    out.host_ids.push(actual.host.id);
                    for ip in &actual.ip_addresses {
                        out.ip_address_ids.push(ip.id);
                    }
                    for port in &actual.ports {
                        out.port_ids.push(port.id);
                    }
                    for service in &actual.services {
                        out.service_ids.push(service.id);
                        for binding in &service.base.bindings {
                            out.binding_ids.push(binding.id());
                        }
                    }
                    for iface in &actual.interfaces {
                        out.interface_ids.push(iface.id);
                    }
                }
            }
        }

        {
            let vlans = self.vlans.read().await;
            out.vlan_ids = vlans.iter().copied().collect();
        }

        out
    }

    /// Clear all entities from buffer (both pending and created).
    /// Call at session boundaries when all await_*() calls have completed.
    ///
    /// This is the cleanup step at the end of discovery sessions:
    /// - Created entries: confirmed by server, no longer needed
    /// - Pending entries: timed out or never confirmed, stale
    pub async fn clear_all(&self) {
        {
            let mut hosts = self.hosts.write().await;
            hosts.clear();
        }
        {
            let mut subnets = self.subnets.write().await;
            subnets.clear();
        }
        {
            let mut vlans = self.vlans.write().await;
            vlans.clear();
        }
    }

    /// Check if the buffer is empty.
    pub async fn is_empty(&self) -> bool {
        let hosts = self.hosts.read().await;
        let subnets = self.subnets.read().await;
        hosts.is_empty() && subnets.is_empty()
    }

    /// Get the count of buffered items without draining.
    pub async fn count(&self) -> (usize, usize) {
        let hosts = self.hosts.read().await;
        let subnets = self.subnets.read().await;
        (hosts.len(), subnets.len())
    }

    /// Get count of pending items only.
    pub async fn pending_count(&self) -> (usize, usize) {
        let hosts = self.hosts.read().await;
        let subnets = self.subnets.read().await;
        let pending_hosts = hosts.values().filter(|e| e.is_pending()).count();
        let pending_subnets = subnets.values().filter(|e| e.is_pending()).count();
        (pending_hosts, pending_subnets)
    }
}

impl Default for EntityBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::shared::attribution::AttributeSource;
    use crate::server::subnets::r#impl::base::{SubnetCidr, SubnetCidrValue};
    use crate::server::{
        hosts::r#impl::{
            base::{Host, HostBase},
            name::{HostName, HostNameSources},
        },
        shared::types::entities::EntitySource,
    };

    #[tokio::test]
    async fn test_entity_buffer_push_and_get_pending() {
        let buffer = EntityBuffer::new();

        // Push a host
        let host = DiscoveryHostRequest {
            host: Host::new(HostBase {
                name: HostName::manual("test-host".to_string()),
                hostname: None,
                tags: vec![],
                network_id: Uuid::new_v4(),
                description: None,
                source: EntitySource::Manual,
                virtualization_metadata: None,
                virtualization_service_id: None,
                hidden: false,
                sys_descr: None,
                sys_object_id: None,
                sys_location: None,
                sys_contact: None,
                management_url: None,
                chassis_id: None,
                sys_name: None,
                manufacturer: None,
                model: None,
                serial_number: None,
                firmware_revision: None,
                software_revision: None,
                credential_assignments: vec![],
            }),
            ip_addresses: vec![],
            ports: vec![],
            services: vec![],
            interfaces: vec![],
            subnets: vec![],
            interfaces_complete: true,
            interface_data_complete: InterfaceDataComplete::default(),
            superseded_wire_shape: false,
        };
        buffer.push_host(host).await;

        // Verify buffer has content
        assert!(!buffer.is_empty().await);
        assert_eq!(buffer.count().await, (1, 0));

        // Get pending - should return hosts without clearing
        let entities = buffer.get_pending().await;
        assert_eq!(entities.hosts.len(), 1);
        assert!(entities.subnets.is_empty());

        // Verify buffer still has content (non-destructive)
        assert!(!buffer.is_empty().await);
        assert_eq!(buffer.pending_count().await, (1, 0));
    }

    #[tokio::test]
    async fn test_entity_buffer_concurrent_access() {
        let buffer = Arc::new(EntityBuffer::new());

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let buf = buffer.clone();
                tokio::spawn(async move {
                    let host = DiscoveryHostRequest {
                        host: Host::new(HostBase {
                            name: HostName::manual(format!("host-{}", i)),
                            hostname: None,
                            tags: vec![],
                            network_id: Uuid::new_v4(),
                            description: None,
                            source: EntitySource::Manual,
                            virtualization_metadata: None,
                            virtualization_service_id: None,
                            hidden: false,
                            sys_descr: None,
                            sys_object_id: None,
                            sys_location: None,
                            sys_contact: None,
                            management_url: None,
                            chassis_id: None,
                            sys_name: None,
                            manufacturer: None,
                            model: None,
                            serial_number: None,
                            firmware_revision: None,
                            software_revision: None,
                            credential_assignments: vec![],
                        }),
                        ip_addresses: vec![],
                        ports: vec![],
                        services: vec![],
                        interfaces: vec![],
                        subnets: vec![],
                        interfaces_complete: true,
                        interface_data_complete: InterfaceDataComplete::default(),
                        superseded_wire_shape: false,
                    };
                    buf.push_host(host).await;
                })
            })
            .collect();

        for handle in handles {
            handle.await.unwrap();
        }

        let entities = buffer.get_pending().await;
        assert_eq!(entities.hosts.len(), 10);
    }

    #[tokio::test]
    async fn test_entity_buffer_lifecycle() {
        use crate::server::subnets::r#impl::{base::SubnetBase, types::SubnetType};
        use chrono::Utc;
        use cidr::{IpCidr, Ipv4Cidr};
        use std::net::Ipv4Addr;

        let buffer = EntityBuffer::new();
        let network_id = Uuid::new_v4();
        let now = Utc::now();

        // Push a subnet
        let subnet = Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            base: SubnetBase {
                name: "test-subnet".to_string(),
                cidr: SubnetCidr::new(
                    SubnetCidrValue(IpCidr::V4(
                        Ipv4Cidr::new(Ipv4Addr::new(192, 168, 1, 0), 24).unwrap(),
                    )),
                    AttributeSource::DaemonSelfReport,
                ),
                network_id,
                description: None,
                subnet_type: SubnetType::Unknown,
                virtualization_service_id: None,
                source: EntitySource::Manual,
                tags: vec![],
            },
        };
        let pending_id = subnet.id;
        buffer.push_subnet(subnet.clone()).await;

        // Verify it's pending
        assert_eq!(buffer.pending_count().await, (0, 1));

        // Mark as created (same ID)
        buffer.mark_subnet_created(pending_id, subnet.clone()).await;

        // Verify it's created
        assert_eq!(buffer.pending_count().await, (0, 0));
        assert_eq!(buffer.count().await, (0, 1));

        // Get the subnet
        let retrieved = buffer.get_subnet(&pending_id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, pending_id);
    }

    #[tokio::test]
    async fn test_get_pending_only_returns_pending() {
        use crate::server::subnets::r#impl::{base::SubnetBase, types::SubnetType};
        use chrono::Utc;
        use cidr::{IpCidr, Ipv4Cidr};
        use std::net::Ipv4Addr;

        let buffer = EntityBuffer::new();
        let network_id = Uuid::new_v4();
        let now = Utc::now();

        // Push two subnets
        let subnet1 = Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            base: SubnetBase {
                name: "subnet-1".to_string(),
                cidr: SubnetCidr::new(
                    SubnetCidrValue(IpCidr::V4(
                        Ipv4Cidr::new(Ipv4Addr::new(192, 168, 1, 0), 24).unwrap(),
                    )),
                    AttributeSource::DaemonSelfReport,
                ),
                network_id,
                description: None,
                subnet_type: SubnetType::Unknown,
                virtualization_service_id: None,
                source: EntitySource::Manual,
                tags: vec![],
            },
        };
        let subnet2 = Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            base: SubnetBase {
                name: "subnet-2".to_string(),
                cidr: SubnetCidr::new(
                    SubnetCidrValue(IpCidr::V4(
                        Ipv4Cidr::new(Ipv4Addr::new(192, 168, 2, 0), 24).unwrap(),
                    )),
                    AttributeSource::DaemonSelfReport,
                ),
                network_id,
                description: None,
                subnet_type: SubnetType::Unknown,
                virtualization_service_id: None,
                source: EntitySource::Manual,
                tags: vec![],
            },
        };

        buffer.push_subnet(subnet1.clone()).await;
        buffer.push_subnet(subnet2.clone()).await;

        // Mark one as created
        buffer
            .mark_subnet_created(subnet1.id, subnet1.clone())
            .await;

        // get_pending should only return the pending subnet (subnet2)
        let pending = buffer.get_pending().await;
        assert_eq!(pending.subnets.len(), 1);
        assert_eq!(pending.subnets[0].id, subnet2.id);

        // Buffer still has both (one pending, one created)
        assert_eq!(buffer.count().await, (0, 2));
        assert_eq!(buffer.pending_count().await, (0, 1));
    }

    #[tokio::test]
    async fn test_clear_all_removes_pending_and_created() {
        use crate::server::subnets::r#impl::{base::SubnetBase, types::SubnetType};
        use chrono::Utc;
        use cidr::{IpCidr, Ipv4Cidr};
        use std::net::Ipv4Addr;

        let buffer = EntityBuffer::new();
        let network_id = Uuid::new_v4();
        let now = Utc::now();

        // Push two subnets
        let subnet1 = Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            base: SubnetBase {
                name: "subnet-1".to_string(),
                cidr: SubnetCidr::new(
                    SubnetCidrValue(IpCidr::V4(
                        Ipv4Cidr::new(Ipv4Addr::new(192, 168, 1, 0), 24).unwrap(),
                    )),
                    AttributeSource::DaemonSelfReport,
                ),
                network_id,
                description: None,
                subnet_type: SubnetType::Unknown,
                virtualization_service_id: None,
                source: EntitySource::Manual,
                tags: vec![],
            },
        };
        let subnet2 = Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            base: SubnetBase {
                name: "subnet-2".to_string(),
                cidr: SubnetCidr::new(
                    SubnetCidrValue(IpCidr::V4(
                        Ipv4Cidr::new(Ipv4Addr::new(192, 168, 2, 0), 24).unwrap(),
                    )),
                    AttributeSource::DaemonSelfReport,
                ),
                network_id,
                description: None,
                subnet_type: SubnetType::Unknown,
                virtualization_service_id: None,
                source: EntitySource::Manual,
                tags: vec![],
            },
        };

        buffer.push_subnet(subnet1.clone()).await;
        buffer.push_subnet(subnet2.clone()).await;

        // Mark one as created, one stays pending
        buffer
            .mark_subnet_created(subnet1.id, subnet1.clone())
            .await;

        // Verify we have one created, one pending
        assert_eq!(buffer.count().await, (0, 2));
        assert_eq!(buffer.pending_count().await, (0, 1));

        // clear_all removes everything
        buffer.clear_all().await;

        // Buffer should be empty
        assert!(buffer.is_empty().await);
        assert_eq!(buffer.count().await, (0, 0));
        assert_eq!(buffer.pending_count().await, (0, 0));
    }

    #[tokio::test]
    async fn test_full_server_poll_lifecycle() {
        // This test simulates the full ServerPoll mode lifecycle:
        // 1. Discovery pushes subnet to buffer
        // 2. Server polls → get_pending() returns pending entities
        // 3. Server processes and sends confirmation
        // 4. Daemon receives confirmation → mark_subnet_created()
        // 5. await_subnet() can now find the created subnet
        // 6. clear_created() removes confirmed entities

        use crate::server::subnets::r#impl::{base::SubnetBase, types::SubnetType};
        use chrono::Utc;
        use cidr::{IpCidr, Ipv4Cidr};
        use std::net::Ipv4Addr;
        use std::time::Duration;

        let buffer = EntityBuffer::new();
        let network_id = Uuid::new_v4();
        let now = Utc::now();

        // Step 1: Discovery pushes subnet
        let pending_id = Uuid::new_v4();
        let subnet = Subnet {
            id: pending_id,
            created_at: now,
            updated_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            base: SubnetBase {
                name: "discovered-subnet".to_string(),
                cidr: SubnetCidr::new(
                    SubnetCidrValue(IpCidr::V4(
                        Ipv4Cidr::new(Ipv4Addr::new(192, 168, 1, 0), 24).unwrap(),
                    )),
                    AttributeSource::DaemonSelfReport,
                ),
                network_id,
                description: None,
                subnet_type: SubnetType::Unknown,
                virtualization_service_id: None,
                source: EntitySource::Manual,
                tags: vec![],
            },
        };
        buffer.push_subnet(subnet.clone()).await;
        assert_eq!(buffer.pending_count().await, (0, 1));

        // Step 2: Server polls - get_pending() returns pending entities
        let polled = buffer.get_pending().await;
        assert_eq!(polled.subnets.len(), 1);
        assert_eq!(polled.subnets[0].id, pending_id);

        // Entry is still in buffer (non-destructive)
        assert_eq!(buffer.pending_count().await, (0, 1));

        // Step 3: Server processes (simulated - creates with same or different ID)
        // In this case, server deduped to existing subnet with different ID
        let actual_id = Uuid::new_v4();
        let actual_subnet = Subnet {
            id: actual_id,
            created_at: now,
            updated_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            base: subnet.base.clone(),
        };

        // Step 4: Daemon receives confirmation
        let id_changed = buffer
            .mark_subnet_created(pending_id, actual_subnet.clone())
            .await;
        assert!(id_changed.is_some());
        let (old_id, new_id) = id_changed.unwrap();
        assert_eq!(old_id, pending_id);
        assert_eq!(new_id, actual_id);

        // Step 5: await_subnet can now find the created subnet
        let cancel = CancellationToken::new();
        let found = buffer
            .await_subnet(&pending_id, Duration::from_millis(100), &cancel)
            .await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, actual_id);

        // Step 6: Session ends - clear_all removes all entities
        buffer.clear_all().await;
        assert!(buffer.is_empty().await);
    }

    #[tokio::test]
    async fn test_push_host_merges_children_for_same_host_id() {
        // This test verifies that push_host merges children when called multiple times
        // with the same host_id. This is critical for Docker discovery where all containers
        // share the daemon's host_id - without merging, only the last container survives.
        use crate::server::ip_addresses::r#impl::base::{IPAddress, IPAddressBase};
        use crate::server::services::r#impl::base::{Service, ServiceBase};
        use chrono::Utc;
        use std::net::{IpAddr, Ipv4Addr};

        let buffer = EntityBuffer::new();
        let network_id = Uuid::new_v4();
        let host_id = Uuid::new_v4(); // Same host_id for all pushes
        let subnet_id = Uuid::new_v4();
        let now = Utc::now();

        // First push: host with 2 ip_addresses
        let host1 = DiscoveryHostRequest {
            host: Host::new(HostBase {
                name: HostName::manual("daemon-host".to_string()),
                hostname: None,
                tags: vec![],
                network_id,
                description: None,
                source: EntitySource::Manual,
                virtualization_metadata: None,
                virtualization_service_id: None,
                hidden: false,
                sys_descr: None,
                sys_object_id: None,
                sys_location: None,
                sys_contact: None,
                management_url: None,
                chassis_id: None,
                sys_name: None,
                manufacturer: None,
                model: None,
                serial_number: None,
                firmware_revision: None,
                software_revision: None,
                credential_assignments: vec![],
            }),
            ip_addresses: vec![
                IPAddress {
                    id: Uuid::new_v4(),
                    created_at: now,
                    updated_at: now,
                    valid_from: now,
                    valid_to: None,
                    lineage_id: None,
                    last_seen_at: now,
                    last_discovery_id: None,
                    first_discovery_id: None,
                    base: IPAddressBase {
                        network_id,
                        host_id,
                        subnet_id,
                        ip_address: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10)),
                        mac_address: None,
                        name: Some("eth0".to_string()),
                        position: 0,
                    },
                },
                IPAddress {
                    id: Uuid::new_v4(),
                    created_at: now,
                    updated_at: now,
                    valid_from: now,
                    valid_to: None,
                    lineage_id: None,
                    last_seen_at: now,
                    last_discovery_id: None,
                    first_discovery_id: None,
                    base: IPAddressBase {
                        network_id,
                        host_id,
                        subnet_id,
                        ip_address: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 11)),
                        mac_address: None,
                        name: Some("eth1".to_string()),
                        position: 1,
                    },
                },
            ],
            ports: vec![],
            services: vec![Service {
                id: Uuid::new_v4(),
                created_at: now,
                updated_at: now,
                valid_from: now,
                valid_to: None,
                lineage_id: None,
                last_seen_at: now,
                last_discovery_id: None,
                first_discovery_id: None,
                base: ServiceBase {
                    network_id,
                    host_id,
                    name: "container-1".to_string(),
                    ..Default::default()
                },
            }],
            interfaces: vec![],
            subnets: vec![],
            interfaces_complete: true,
            interface_data_complete: InterfaceDataComplete::default(),
            superseded_wire_shape: false,
        };
        // Set the host ID to match our shared host_id
        let mut host1 = host1;
        host1.host.id = host_id;
        buffer.push_host(host1).await;

        // Second push: same host_id with 1 different interface and 1 different service
        let host2 = DiscoveryHostRequest {
            host: Host::new(HostBase {
                name: HostName::manual("daemon-host".to_string()),
                hostname: None,
                tags: vec![],
                network_id,
                description: None,
                source: EntitySource::Manual,
                virtualization_metadata: None,
                virtualization_service_id: None,
                hidden: false,
                sys_descr: None,
                sys_object_id: None,
                sys_location: None,
                sys_contact: None,
                management_url: None,
                chassis_id: None,
                sys_name: None,
                manufacturer: None,
                model: None,
                serial_number: None,
                firmware_revision: None,
                software_revision: None,
                credential_assignments: vec![],
            }),
            ip_addresses: vec![IPAddress {
                id: Uuid::new_v4(),
                created_at: now,
                updated_at: now,
                valid_from: now,
                valid_to: None,
                lineage_id: None,
                last_seen_at: now,
                last_discovery_id: None,
                first_discovery_id: None,
                base: IPAddressBase {
                    network_id,
                    host_id,
                    subnet_id,
                    ip_address: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 12)),
                    mac_address: None,
                    name: Some("eth2".to_string()),
                    position: 2,
                },
            }],
            ports: vec![],
            services: vec![Service {
                id: Uuid::new_v4(),
                created_at: now,
                updated_at: now,
                valid_from: now,
                valid_to: None,
                lineage_id: None,
                last_seen_at: now,
                last_discovery_id: None,
                first_discovery_id: None,
                base: ServiceBase {
                    network_id,
                    host_id,
                    name: "container-2".to_string(),
                    ..Default::default()
                },
            }],
            interfaces: vec![],
            subnets: vec![],
            interfaces_complete: true,
            interface_data_complete: InterfaceDataComplete::default(),
            superseded_wire_shape: false,
        };
        let mut host2 = host2;
        host2.host.id = host_id;
        buffer.push_host(host2).await;

        // Verify: should have 1 host with 3 ip_addresses and 2 services
        let pending = buffer.get_pending().await;
        assert_eq!(pending.hosts.len(), 1, "Should have exactly 1 host");

        let host = &pending.hosts[0];
        assert_eq!(host.host.id, host_id);
        assert_eq!(
            host.ip_addresses.len(),
            3,
            "Should have 3 ip_addresses after merge"
        );
        assert_eq!(host.services.len(), 2, "Should have 2 services after merge");

        // Verify interface names to confirm correct merge
        let interface_names: Vec<_> = host
            .ip_addresses
            .iter()
            .filter_map(|i| i.base.name.clone())
            .collect();
        assert!(interface_names.contains(&"eth0".to_string()));
        assert!(interface_names.contains(&"eth1".to_string()));
        assert!(interface_names.contains(&"eth2".to_string()));

        // Verify service names
        let service_names: Vec<_> = host.services.iter().map(|s| s.base.name.clone()).collect();
        assert!(service_names.contains(&"container-1".to_string()));
        assert!(service_names.contains(&"container-2".to_string()));
    }

    #[tokio::test]
    async fn get_scanned_ids_empty_buffer_returns_default() {
        let buffer = EntityBuffer::new();
        let scanned = buffer.get_scanned_ids().await;
        assert!(scanned.host_ids.is_empty());
        assert!(scanned.subnet_ids.is_empty());
        assert!(scanned.vlan_ids.is_empty());
        assert!(scanned.ip_address_ids.is_empty());
        assert!(scanned.port_ids.is_empty());
        assert!(scanned.service_ids.is_empty());
        assert!(scanned.interface_ids.is_empty());
        assert!(scanned.binding_ids.is_empty());
    }

    #[tokio::test]
    async fn get_scanned_ids_excludes_pending_only_includes_created() {
        use crate::server::subnets::r#impl::{base::SubnetBase, types::SubnetType};
        use chrono::Utc;
        use cidr::{IpCidr, Ipv4Cidr};
        use std::net::Ipv4Addr;

        let buffer = EntityBuffer::new();
        let now = Utc::now();
        let network_id = Uuid::new_v4();

        // One Pending subnet, one Created subnet — only Created should be reported.
        let pending_subnet = Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            base: SubnetBase {
                name: "pending".to_string(),
                cidr: SubnetCidr::new(
                    SubnetCidrValue(IpCidr::V4(
                        Ipv4Cidr::new(Ipv4Addr::new(10, 0, 1, 0), 24).unwrap(),
                    )),
                    AttributeSource::DaemonSelfReport,
                ),
                network_id,
                description: None,
                subnet_type: SubnetType::Unknown,
                virtualization_service_id: None,
                source: EntitySource::Manual,
                tags: vec![],
            },
        };
        buffer.push_subnet(pending_subnet).await;

        let created_subnet = Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            base: SubnetBase {
                name: "created".to_string(),
                cidr: SubnetCidr::new(
                    SubnetCidrValue(IpCidr::V4(
                        Ipv4Cidr::new(Ipv4Addr::new(10, 0, 2, 0), 24).unwrap(),
                    )),
                    AttributeSource::DaemonSelfReport,
                ),
                network_id,
                description: None,
                subnet_type: SubnetType::Unknown,
                virtualization_service_id: None,
                source: EntitySource::Manual,
                tags: vec![],
            },
        };
        let pending_id = created_subnet.id;
        buffer.push_subnet(created_subnet.clone()).await;
        buffer
            .mark_subnet_created(pending_id, created_subnet.clone())
            .await;

        let scanned = buffer.get_scanned_ids().await;
        assert_eq!(
            scanned.subnet_ids,
            vec![pending_id],
            "Only Created subnets should be in scanned"
        );
    }

    #[tokio::test]
    async fn get_scanned_ids_collects_host_children_recursively() {
        use crate::server::ip_addresses::r#impl::base::{IPAddress, IPAddressBase};
        use crate::server::services::r#impl::base::{Service, ServiceBase};
        use chrono::Utc;
        use std::net::{IpAddr, Ipv4Addr};

        let buffer = EntityBuffer::new();
        let now = Utc::now();
        let network_id = Uuid::new_v4();
        let host_id = Uuid::new_v4();
        let subnet_id = Uuid::new_v4();

        let ip_id = Uuid::new_v4();
        let service_id = Uuid::new_v4();

        let host_req = DiscoveryHostRequest {
            host: Host::new(HostBase {
                name: HostName::manual("test-host".to_string()),
                hostname: None,
                tags: vec![],
                network_id,
                description: None,
                source: EntitySource::Manual,
                virtualization_metadata: None,
                virtualization_service_id: None,
                hidden: false,
                sys_descr: None,
                sys_object_id: None,
                sys_location: None,
                sys_contact: None,
                management_url: None,
                chassis_id: None,
                sys_name: None,
                manufacturer: None,
                model: None,
                serial_number: None,
                firmware_revision: None,
                software_revision: None,
                credential_assignments: vec![],
            }),
            ip_addresses: vec![IPAddress {
                id: ip_id,
                created_at: now,
                updated_at: now,
                valid_from: now,
                valid_to: None,
                lineage_id: None,
                last_seen_at: now,
                last_discovery_id: None,
                first_discovery_id: None,
                base: IPAddressBase {
                    network_id,
                    host_id,
                    subnet_id,
                    ip_address: IpAddr::V4(Ipv4Addr::new(10, 0, 1, 5)),
                    mac_address: None,
                    name: None,
                    position: 0,
                },
            }],
            ports: vec![],
            services: vec![Service {
                id: service_id,
                created_at: now,
                updated_at: now,
                valid_from: now,
                valid_to: None,
                lineage_id: None,
                last_seen_at: now,
                last_discovery_id: None,
                first_discovery_id: None,
                base: ServiceBase {
                    network_id,
                    host_id,
                    name: "svc".to_string(),
                    ..Default::default()
                },
            }],
            interfaces: vec![],
            subnets: vec![],
            interfaces_complete: true,
            interface_data_complete: InterfaceDataComplete::default(),
            superseded_wire_shape: false,
        };
        let mut host_req = host_req;
        host_req.host.id = host_id;

        buffer.push_host(host_req.clone()).await;

        // Convert pending → Created via mark_host_created. mark_host_created
        // wants a HostResponse, so synthesize one from the request data.
        let host_response =
            crate::server::hosts::r#impl::api::HostResponse::from_host_with_children(
                host_req.host.clone(),
                host_req.ip_addresses.clone(),
                host_req.ports.clone(),
                host_req.services.clone(),
                host_req.interfaces.clone(),
            );
        buffer.mark_host_created(host_id, host_response).await;

        let scanned = buffer.get_scanned_ids().await;
        assert_eq!(scanned.host_ids, vec![host_id]);
        assert_eq!(scanned.ip_address_ids, vec![ip_id]);
        assert_eq!(scanned.service_ids, vec![service_id]);
    }

    #[tokio::test]
    async fn push_scanned_vlans_round_trips_through_get_scanned_ids() {
        let buffer = EntityBuffer::new();
        let v1 = Uuid::new_v4();
        let v2 = Uuid::new_v4();

        buffer.push_scanned_vlans([v1, v2]).await;

        let scanned = buffer.get_scanned_ids().await;
        assert_eq!(scanned.vlan_ids.len(), 2);
        assert!(scanned.vlan_ids.contains(&v1));
        assert!(scanned.vlan_ids.contains(&v2));
    }

    #[tokio::test]
    async fn clear_all_clears_vlan_ids_too() {
        let buffer = EntityBuffer::new();
        buffer.push_scanned_vlans([Uuid::new_v4()]).await;
        buffer.clear_all().await;
        let scanned = buffer.get_scanned_ids().await;
        assert!(scanned.vlan_ids.is_empty());
    }
}
