//! Bulk insertion of a generated demo dataset.
//!
//! Split out of `handlers.rs` because it is the one part of the demo flow with real ordering
//! constraints — foreign keys between the entity types decide the sequence of writes — and
//! because taking a `&ServiceFactory` rather than an `AppState` makes it directly drivable from
//! `src/tests/demo_data_seeding.rs` against a testcontainers Postgres with the migrations applied.

use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::auth::service::hash_password;
use crate::server::bindings::r#impl::base::Binding;
use crate::server::hosts::r#impl::base::Host;
use crate::server::interface_neighbors::r#impl::base::Neighbor;
use crate::server::organizations::demo_data::DemoData;
use crate::server::services::r#impl::base::Service;
use crate::server::shared::services::factory::ServiceFactory;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::traits::{Entity, Storable, Storage};
use crate::server::shared::types::api::{ApiError, ApiResult};
use crate::server::snapshots::types::base::{Snapshot, SnapshotBase};
use crate::server::subnets::r#impl::base::Subnet;
use crate::server::tags::entity_tags::EntityTag;
use crate::server::users::r#impl::base::{User, UserBase};
use crate::server::users::r#impl::permissions::UserOrgPermissions;
use chrono::{DateTime, Utc};
use email_address::EmailAddress;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use super::handlers::DEMO_USER_ID;

/// A demo dataset that cannot be written in a single-layer hoist.
///
/// Both variants mean `demo_data.rs` and this module have drifted apart, not that a user did
/// anything wrong — hence `internal_error`.
#[derive(Debug)]
pub(crate) enum DemoSeedError {
    /// A row names a virtualizing service the dataset never defines.
    UnresolvableOwner { owner: Uuid },
    /// A row that has to be hoisted is itself virtualized by something not yet written, so one
    /// hoisted layer is not enough to order the batch.
    NestedOwner { hoisted: Uuid, names: Uuid },
}

impl std::fmt::Display for DemoSeedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnresolvableOwner { owner } => write!(
                f,
                "demo data declares virtualization_service_id {owner}, which matches no service \
                 in the dataset"
            ),
            Self::NestedOwner { hoisted, names } => write!(
                f,
                "demo data entity {hoisted} must be written early because something is \
                 virtualized by it, but it is itself virtualized by {names}. The hoist below is \
                 single-layer; extend it to order these before adding such a case."
            ),
        }
    }
}

impl From<DemoSeedError> for ApiError {
    fn from(e: DemoSeedError) -> Self {
        ApiError::internal_error(&e.to_string())
    }
}

/// A batch split so that no row is written before the service its `virtualization_service_id`
/// names.
#[derive(Debug)]
pub(crate) struct VirtualizationHoist {
    /// Hosts that own a hoisted service — written first, because `services.host_id` FKs to them.
    pub owner_hosts: Vec<Host>,
    /// The virtualizing services themselves.
    pub owner_services: Vec<Service>,
    pub deferred_hosts: Vec<Host>,
    pub deferred_services: Vec<Service>,
}

/// Split a batch so every virtualizing service is written before the rows that name it.
///
/// Demo seeding writes subnets and hosts before services, but a DockerBridge subnet names the
/// runtime service that owns it, a VM host names its hypervisor service, and a container service
/// names its runtime — all now real foreign keys into `services`. Discovery hit the same problem
/// and broke the cycle by materialising the owner's row early (`hosts/service/create.rs:537`);
/// this is that idea for a bulk batch. It is not a true cycle: `services` FKs only to `hosts` and
/// `networks` — bindings live in their own table — so an owner service can be written as soon as
/// its host row exists.
///
/// **Where this deliberately differs from discovery.** `resolve_owner_service_id`
/// (`hosts/service/create.rs:36`) degrades an id it cannot resolve to `None`, which is right
/// there: the daemon mints service ids in memory and cannot resolve them across submissions, so
/// one unowned bridge beats a host that will not create. Demo data mints both ends of every
/// reference itself, so an id that does not resolve is an authoring bug — and nulling it would
/// quietly delete the exact relationship the demo exists to show (containers under their runtime,
/// VMs under their Proxmox host). So this errors where discovery degrades.
pub(crate) fn hoist_virtualization_owners(
    subnets: &[Subnet],
    hosts: &[Host],
    services: &[Service],
    already_written: &HashSet<Uuid>,
) -> Result<VirtualizationHoist, DemoSeedError> {
    let owner_ids: HashSet<Uuid> = subnets
        .iter()
        .filter_map(|s| s.base.virtualization_service_id)
        .chain(
            hosts
                .iter()
                .filter_map(|h| h.base.virtualization_service_id),
        )
        .chain(
            services
                .iter()
                .filter_map(|s| s.base.virtualization_service_id),
        )
        .filter(|id| !already_written.contains(id))
        .collect();

    let services_by_id: HashMap<Uuid, &Service> = services.iter().map(|s| (s.id, s)).collect();

    let mut owner_host_ids: HashSet<Uuid> = HashSet::new();
    for owner in &owner_ids {
        let service = services_by_id
            .get(owner)
            .ok_or(DemoSeedError::UnresolvableOwner { owner: *owner })?;
        // A service to be hoisted cannot itself be waiting on an unwritten owner.
        if let Some(names) = service.base.virtualization_service_id
            && !already_written.contains(&names)
        {
            return Err(DemoSeedError::NestedOwner {
                hoisted: service.id,
                names,
            });
        }
        owner_host_ids.insert(service.base.host_id);
    }

    let (owner_hosts, deferred_hosts): (Vec<Host>, Vec<Host>) = hosts
        .iter()
        .cloned()
        .partition(|h| owner_host_ids.contains(&h.id));

    // Nor can a host that has to be hoisted carry one.
    for host in &owner_hosts {
        if let Some(names) = host.base.virtualization_service_id
            && !already_written.contains(&names)
        {
            return Err(DemoSeedError::NestedOwner {
                hoisted: host.id,
                names,
            });
        }
    }

    let (owner_services, deferred_services): (Vec<Service>, Vec<Service>) = services
        .iter()
        .cloned()
        .partition(|s| owner_ids.contains(&s.id));

    Ok(VirtualizationHoist {
        owner_hosts,
        owner_services,
        deferred_hosts,
        deferred_services,
    })
}

/// Collect EntityTag records from tagged entities into the accumulator.
fn collect_entity_tags<T: Entity>(entities: &[T], out: &mut Vec<EntityTag>) {
    use crate::server::tags::entity_tags::EntityTagBase;
    for entity in entities {
        if let Some(tags) = entity.get_tags() {
            for &tag_id in tags {
                out.push(EntityTag::new(EntityTagBase::new(
                    entity.id(),
                    T::entity_type(),
                    tag_id,
                )));
            }
        }
    }
}

/// Generate and insert a full demo dataset into an organization.
///
/// Assumes the organization has just been reset — there are no collisions, so this uses bulk
/// `create_many` throughout instead of the per-entity create/upsert paths.
pub(crate) async fn seed_demo_data(
    services: &ServiceFactory,
    organization_id: Uuid,
    user_id: Uuid,
    entity: AuthenticatedEntity,
) -> ApiResult<()> {
    let demo_data = DemoData::generate(organization_id, user_id);
    insert_demo_data(services, demo_data, organization_id, user_id, entity).await
}

/// Insert an already-generated dataset.
///
/// Split from [`seed_demo_data`] so a caller can hold the `DemoData` it seeded: every id in it is
/// minted per call by `Uuid::new_v4()`, so regenerating produces a different graph and there is
/// otherwise no way to assert that what landed in the database is what was declared.
pub(crate) async fn insert_demo_data(
    services: &ServiceFactory,
    demo_data: DemoData,
    organization_id: Uuid,
    user_id: Uuid,
    entity: AuthenticatedEntity,
) -> ApiResult<()> {
    // Collect all entity tags to bulk insert at the end (single INSERT).
    let mut all_entity_tags: Vec<EntityTag> = Vec::new();

    // Insert entities in dependency order using bulk inserts.
    // Since we just reset the org, there are no collisions — we use service-level
    // create_many (publishes one event per scope per entity type) instead of
    // per-entity create() for speed. Entities without event subscribers use
    // storage().create_many() directly.

    // 1. Tags (no dependencies)
    services
        .tag_service
        .storage()
        .create_many(&demo_data.tags)
        .await?;

    // 2. Credentials (depends on organization)
    services
        .credential_service
        .storage()
        .create_many(&demo_data.credentials)
        .await?;

    // 3. Networks (depends on organization, tags)
    let created_networks = services
        .network_service
        .storage()
        .create_many(&demo_data.networks)
        .await?;
    collect_entity_tags(&created_networks, &mut all_entity_tags);

    // 3.5. Network-credential associations — one bulk insert across all
    // networks (no per-network lock/delete; the org was just reset).
    let network_cred_pairs: Vec<(Uuid, Uuid)> = demo_data
        .network_credential_assignments
        .iter()
        .flat_map(|a| {
            let network_id = a.network_id;
            a.credential_ids
                .iter()
                .map(move |&cred_id| (network_id, cred_id))
        })
        .collect();
    services
        .credential_service
        .create_network_credentials(&network_cred_pairs)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    // 3.8. Flatten hosts, ip_addresses, ports, services from HostWithServices bundles. Hoisted
    // ahead of the writes below because step 3.9 needs to see every host and service at once to
    // work out the order they can go in.
    let mut all_hosts = Vec::new();
    let mut all_ip_addresses = Vec::new();
    let mut all_ports = Vec::new();
    let mut all_services: Vec<Service> = Vec::new();
    for hws in &demo_data.hosts_with_services {
        let host_id = hws.host.id;
        let network_id = hws.host.base.network_id;
        all_hosts.push(hws.host.clone());
        all_ip_addresses.extend(hws.ip_addresses.clone());
        all_ports.extend(
            hws.ports
                .iter()
                .cloned()
                .map(|p| p.with_host(host_id, network_id)),
        );
        all_services.extend(hws.services.clone());
    }

    // 3.9. Owner-service row phase. Subnets are written before hosts and hosts before services,
    // but a DockerBridge subnet names the Docker daemon service that owns it and a VM host names
    // its Proxmox service — both foreign keys into `services`. Write those services, and the
    // bare-metal hosts they run on, ahead of everything that names them.
    let hoist = hoist_virtualization_owners(
        &demo_data.subnets,
        &all_hosts,
        &all_services,
        &HashSet::new(),
    )?;
    let created_owner_hosts = services
        .host_service
        .create_many(&hoist.owner_hosts, entity.clone())
        .await?;
    collect_entity_tags(&created_owner_hosts, &mut all_entity_tags);
    let created_owner_services = services
        .service_service
        .create_many(&hoist.owner_services, entity.clone())
        .await?;
    collect_entity_tags(&created_owner_services, &mut all_entity_tags);

    // 4. Subnets (depends on networks, and on the owner services written above)
    let created_subnets = services
        .subnet_service
        .create_many(&demo_data.subnets, entity.clone())
        .await?;
    collect_entity_tags(&created_subnets, &mut all_entity_tags);

    // 4.5. VLANs (depends on networks)
    services
        .vlan_service
        .storage()
        .create_many(&demo_data.vlans)
        .await?;

    // 4.6. Subnet↔VLAN junction rows (depend on subnets + VLANs). One bulk
    // insert, no per-subnet lock (fresh org). Derived to mirror the discovery
    // reconciler, so demo subnets show their VLANs like a real deployment.
    services
        .vlan_service
        .subnet_vlan_storage
        .create_many(&demo_data.subnet_vlan_records)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    // 5. The rest of the hosts — bypass discover_host (no collisions in fresh org). Every host
    // here that is a VM names a hypervisor service written in step 3.9.
    let created_hosts = services
        .host_service
        .create_many(&hoist.deferred_hosts, entity.clone())
        .await?;
    collect_entity_tags(&created_hosts, &mut all_entity_tags);

    // 5.3. Resolve interface neighbor links in memory. GH #701 moved resolved adjacencies off
    // `interfaces` into `interface_neighbor_interfaces`, which FKs to `interfaces(id)` — so the
    // pairing is computed here (while everything is still keyed by host name / if_index) but
    // written via `InterfaceNeighborService::reconcile_interface_neighbors` only after the
    // interfaces themselves exist (5.5 below). Demo data authors each pairing directly, so this
    // is a full resolution — `Neighbor::Interface`, never `Neighbor::Host` — and there is no raw
    // LLDP/CDP evidence to fabricate into `interface_neighbor_candidates` to justify it.
    let neighbor_links: Vec<(Uuid, Uuid, Uuid, DateTime<Utc>)> = {
        // By title rather than stored name: demo neighbour pairings name hosts the way the UI
        // does, and some demo hosts are nameless and titled by an identifier.
        let host_id_to_name: HashMap<Uuid, String> = all_hosts
            .iter()
            .map(|h| (h.id, h.display_name(&[]).unwrap_or_default()))
            .collect();

        let mut if_entry_lookup: HashMap<(String, Option<i32>), Uuid> = HashMap::new();
        let mut interface_context: HashMap<Uuid, (Uuid, DateTime<Utc>)> = HashMap::new();
        for entry in &demo_data.interfaces {
            if let Some(host_name) = host_id_to_name.get(&entry.base.host_id) {
                if_entry_lookup.insert((host_name.clone(), entry.base.if_index), entry.id);
            }
            interface_context.insert(entry.id, (entry.base.network_id, entry.last_seen_at));
        }

        // Build (network_id, source_interface_id, target_interface_id, scan_time) tuples.
        let mut links = Vec::new();
        for neighbor_update in &demo_data.neighbor_updates {
            let source_key = (
                neighbor_update.source_host_name.clone(),
                Some(neighbor_update.source_if_index),
            );
            let target_key = (
                neighbor_update.target_host_name.clone(),
                Some(neighbor_update.target_if_index),
            );
            if let (Some(&source_id), Some(&target_id)) = (
                if_entry_lookup.get(&source_key),
                if_entry_lookup.get(&target_key),
            ) && let Some(&(network_id, scan_time)) = interface_context.get(&source_id)
            {
                links.push((network_id, source_id, target_id, scan_time));
            }
        }
        links
    };
    let interfaces = demo_data.interfaces;

    // 5.4. ip_addresses must be committed before interfaces: interfaces.ip_address_id
    // FKs into ip_addresses(id), and create_many is not transactional — each chunk
    // autocommits on its own pooled connection. Run concurrently, the child batch can
    // reach the server before the parent rows exist and fail the FK, which is exactly
    // what connection-acquisition skew against a remote database produces. Awaiting
    // here makes the ordering unconditional, at the cost of one serialized round trip.
    services
        .ip_address_service
        .create_many(&all_ip_addresses, entity.clone())
        .await?;

    // 5.5. The remaining three host-children have no interdependency, so they stay
    // concurrent — each takes its own pooled connection. The services result is
    // needed downstream for bindings + entity tags.
    let (_, created_services, _) = tokio::try_join!(
        async {
            services
                .port_service
                .create_many(&all_ports, entity.clone())
                .await
                .map_err(ApiError::from)
        },
        async {
            services
                .service_service
                .create_many(&hoist.deferred_services, entity.clone())
                .await
                .map_err(ApiError::from)
        },
        async {
            services
                .interface_service
                .create_many(&interfaces, entity.clone())
                .await
                .map_err(ApiError::from)
        },
    )?;
    collect_entity_tags(&created_services, &mut all_entity_tags);

    // 5.5b. Write the resolved neighbour rows computed in 5.3, now that the interfaces they
    // reference exist. One `reconcile_interface_neighbors` call per source interface — demo data
    // is not a real discovery run, so `discovery_id` is `None`.
    for (network_id, source_id, target_id, scan_time) in neighbor_links {
        services
            .interface_neighbor_service
            .reconcile_interface_neighbors(
                network_id,
                source_id,
                &[(Neighbor::Interface(target_id), Some(scan_time))],
                scan_time,
                None,
            )
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    }

    // 5.6. Bindings (child entities of services, stored in a separate table).
    // Spans both halves of the hoist: a hoisted runtime service has bindings of its own.
    let all_bindings: Vec<Binding> = created_owner_services
        .iter()
        .chain(created_services.iter())
        .flat_map(|s| {
            s.base
                .bindings
                .iter()
                .cloned()
                .map(|b| b.with_service(s.id, s.base.network_id))
        })
        .collect();
    services
        .binding_service
        .create_many(&all_bindings, entity.clone())
        .await?;

    // 6. Daemons (depends on hosts, networks, subnets)
    services
        .daemon_service
        .storage()
        .create_many(&demo_data.daemons)
        .await?;

    // 7. Daemon API Keys (depends on networks)
    services
        .daemon_api_key_service
        .storage()
        .create_many(&demo_data.api_keys)
        .await?;

    // 8. Discoveries (depends on daemons, networks, subnets)
    services
        .discovery_service
        .storage()
        .create_many(&demo_data.discoveries)
        .await?;

    // 9. Dependencies + members — pre-generated during DemoData::generate().
    // create_many bypasses per-entity service logic, so persist members
    // separately — as one bulk insert across all dependencies, resolving each
    // Bindings member's service_id from the in-memory bindings (avoiding a
    // SELECT per binding) and skipping the per-dependency lock/delete.
    let created_deps = services
        .dependency_service
        .create_many(&demo_data.dependencies, entity.clone())
        .await?;
    {
        use crate::server::dependencies::dependency_members::{
            DependencyMemberRecord, DependencyMemberRecordBase,
        };
        use crate::server::dependencies::r#impl::base::DependencyMembers;
        use std::collections::HashMap;

        let binding_to_service: HashMap<Uuid, Uuid> = all_bindings
            .iter()
            .map(|b| (b.id, b.base.service_id))
            .collect();

        let mut member_records: Vec<DependencyMemberRecord> = Vec::new();
        for dep in &created_deps {
            match &dep.base.members {
                DependencyMembers::Services { service_ids } => {
                    let mut seen = HashSet::new();
                    for (position, service_id) in service_ids
                        .iter()
                        .filter(|id| seen.insert(**id))
                        .enumerate()
                    {
                        member_records.push(DependencyMemberRecord::new(
                            DependencyMemberRecordBase::new(
                                dep.id,
                                *service_id,
                                None,
                                position as i32,
                            ),
                        ));
                    }
                }
                DependencyMembers::Bindings { binding_ids } => {
                    for (position, binding_id) in binding_ids.iter().enumerate() {
                        if let Some(&service_id) = binding_to_service.get(binding_id) {
                            member_records.push(DependencyMemberRecord::new(
                                DependencyMemberRecordBase::new(
                                    dep.id,
                                    service_id,
                                    Some(*binding_id),
                                    position as i32,
                                ),
                            ));
                        }
                    }
                }
            }
        }
        services
            .dependency_service
            .create_members(&member_records)
            .await?;
    }

    // 10. Topologies (depends on networks + the entities created above).
    // The graph is built on request from the persisted entities, so `create`
    // just persists the row + options. Must run before shares (step 11), whose
    // `topology_id` FK references these rows.
    for topology in demo_data.topologies {
        services
            .topology_service
            .create(topology, entity.clone())
            .await?;
    }

    // 10.5. Bulk insert all entity tags (single INSERT for all tagged entities)
    if !all_entity_tags.is_empty() {
        services
            .entity_tag_service
            .create_many(&all_entity_tags)
            .await?;
    }

    // 11. Shares (depends on topologies)
    services
        .share_service
        .storage()
        .create_many(&demo_data.shares)
        .await?;

    // 12. Demo admin user
    let password = hash_password("password123")?;
    let mut demo_admin = User::new(UserBase::new_password(
        EmailAddress::new_unchecked("demo@scanopy.net"),
        true,
        password,
        organization_id,
        UserOrgPermissions::Admin,
        vec![],
        None,
    ));
    demo_admin.base.email_verified = true;
    demo_admin.id = DEMO_USER_ID;
    services
        .user_service
        .create(demo_admin, entity.clone())
        .await?;

    // 13. User API Keys (depends on demo admin user + network access junction table)
    for (api_key, network_ids) in demo_data.user_api_keys {
        services
            .user_api_key_service
            .create_with_networks(api_key, network_ids, entity.clone())
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    }

    // 14. One snapshot per network so the snapshot UI is exercised in demo orgs.
    // Must run last: close-and-clone captures the live entity set, so all demo
    // entities (and their entity-tags + live topology rows) must already exist.
    // Each network's snapshot is scoped to its own network_id and runs in its
    // own transaction, so the networks' snapshots run concurrently.
    let snapshot_futures = created_networks.iter().map(|network| {
        let entity = entity.clone();
        async move {
            let snapshot = Snapshot {
                base: SnapshotBase::new(network.id, chrono::Utc::now(), Some(user_id)),
                ..Default::default()
            };
            let created = services
                .snapshot_service
                .create(snapshot, entity)
                .await
                .map_err(ApiError::from)?;
            services
                .snapshot_service
                .run_close_and_clone(created.base.network_id, created.base.taken_at, created.id)
                .await
                .map_err(|e| ApiError::internal_error(&e.to_string()))?;
            // No snapshot topology row — the graph is built on request from the
            // closed copies stamped above.
            Ok::<(), ApiError>(())
        }
    });
    futures::future::try_join_all(snapshot_futures).await?;

    // 15. "Recently discovered" hosts — created AFTER the snapshot so the
    // snapshot captures the earlier state and the live view visibly differs
    // (these hosts/services appear only in live). Self-contained (no
    // dependencies, neighbors, or interfaces), so the main flatten→create
    // pattern applies, minus interfaces.
    if !demo_data.recent_hosts_with_services.is_empty() {
        let mut recent_hosts = Vec::new();
        let mut recent_ips = Vec::new();
        let mut recent_ports = Vec::new();
        let mut recent_services: Vec<Service> = Vec::new();
        for hws in &demo_data.recent_hosts_with_services {
            let host_id = hws.host.id;
            let network_id = hws.host.base.network_id;
            recent_hosts.push(hws.host.clone());
            recent_ips.extend(hws.ip_addresses.clone());
            recent_ports.extend(
                hws.ports
                    .iter()
                    .cloned()
                    .map(|p| p.with_host(host_id, network_id)),
            );
            recent_services.extend(hws.services.clone());
        }

        // Same hoist as step 3.9, with everything written above counted as available — a recent
        // host may name a hypervisor service from the main batch. Nothing in this batch declares
        // virtualization today, so the split is a no-op; wiring it anyway means adding one
        // cannot fail the foreign key silently.
        let already_written: HashSet<Uuid> = created_owner_services
            .iter()
            .chain(created_services.iter())
            .map(|s| s.id)
            .collect();
        let recent_hoist =
            hoist_virtualization_owners(&[], &recent_hosts, &recent_services, &already_written)?;

        let mut recent_entity_tags: Vec<EntityTag> = Vec::new();
        let created_recent_owner_hosts = services
            .host_service
            .create_many(&recent_hoist.owner_hosts, entity.clone())
            .await?;
        collect_entity_tags(&created_recent_owner_hosts, &mut recent_entity_tags);
        let created_recent_owner_services = services
            .service_service
            .create_many(&recent_hoist.owner_services, entity.clone())
            .await?;
        collect_entity_tags(&created_recent_owner_services, &mut recent_entity_tags);

        let created_recent_hosts = services
            .host_service
            .create_many(&recent_hoist.deferred_hosts, entity.clone())
            .await?;
        collect_entity_tags(&created_recent_hosts, &mut recent_entity_tags);

        let (_, _, created_recent_services) = tokio::try_join!(
            async {
                services
                    .ip_address_service
                    .create_many(&recent_ips, entity.clone())
                    .await
                    .map_err(ApiError::from)
            },
            async {
                services
                    .port_service
                    .create_many(&recent_ports, entity.clone())
                    .await
                    .map_err(ApiError::from)
            },
            async {
                services
                    .service_service
                    .create_many(&recent_hoist.deferred_services, entity.clone())
                    .await
                    .map_err(ApiError::from)
            },
        )?;
        collect_entity_tags(&created_recent_services, &mut recent_entity_tags);

        let recent_bindings: Vec<Binding> = created_recent_owner_services
            .iter()
            .chain(created_recent_services.iter())
            .flat_map(|s| {
                s.base
                    .bindings
                    .iter()
                    .cloned()
                    .map(|b| b.with_service(s.id, s.base.network_id))
            })
            .collect();
        services
            .binding_service
            .create_many(&recent_bindings, entity.clone())
            .await?;

        if !recent_entity_tags.is_empty() {
            services
                .entity_tag_service
                .create_many(&recent_entity_tags)
                .await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{host, service, subnet};

    /// The shape the demo dataset actually has: a bare-metal runtime host, the runtime service on
    /// it, a bridge subnet owned by that service, and a guest host owned by it too.
    fn dataset() -> (Vec<Subnet>, Vec<Host>, Vec<Service>) {
        let network_id = Uuid::new_v4();

        let runtime_host = host(&network_id);
        let guest_host = host(&network_id);
        let runtime_service = service(&network_id, &runtime_host.id);
        let mut guest_service = service(&network_id, &guest_host.id);
        guest_service.base.virtualization_service_id = Some(runtime_service.id);

        let mut bridge = subnet(&network_id);
        bridge.base.virtualization_service_id = Some(runtime_service.id);

        let mut guest_host = guest_host;
        guest_host.base.virtualization_service_id = Some(runtime_service.id);

        (
            vec![bridge],
            vec![runtime_host, guest_host],
            vec![runtime_service, guest_service],
        )
    }

    #[test]
    fn the_owner_and_its_host_are_written_before_everything_that_names_them() {
        let (subnets, hosts, services) = dataset();
        let runtime_service_id = subnets[0].base.virtualization_service_id.unwrap();
        let runtime_host_id = hosts[0].id;

        let hoist =
            hoist_virtualization_owners(&subnets, &hosts, &services, &HashSet::new()).unwrap();

        assert_eq!(
            hoist
                .owner_services
                .iter()
                .map(|s| s.id)
                .collect::<Vec<_>>(),
            vec![runtime_service_id]
        );
        assert_eq!(
            hoist.owner_hosts.iter().map(|h| h.id).collect::<Vec<_>>(),
            vec![runtime_host_id]
        );
        // The guest half waits, and nothing is dropped on the floor.
        assert_eq!(hoist.deferred_hosts.len(), 1);
        assert_eq!(hoist.deferred_services.len(), 1);
        assert!(
            hoist
                .deferred_hosts
                .iter()
                .all(|h| h.base.virtualization_service_id == Some(runtime_service_id))
        );
    }

    #[test]
    fn an_owner_already_in_the_database_is_not_hoisted_again() {
        let (subnets, hosts, services) = dataset();
        let runtime_service_id = subnets[0].base.virtualization_service_id.unwrap();
        let already_written = HashSet::from([runtime_service_id]);

        let hoist =
            hoist_virtualization_owners(&subnets, &hosts, &services, &already_written).unwrap();

        assert!(hoist.owner_services.is_empty());
        assert!(hoist.owner_hosts.is_empty());
        assert_eq!(hoist.deferred_hosts.len(), 2);
        assert_eq!(hoist.deferred_services.len(), 2);
    }

    #[test]
    fn an_owner_the_dataset_never_defines_is_an_error_not_a_null() {
        let (mut subnets, hosts, services) = dataset();
        subnets[0].base.virtualization_service_id = Some(Uuid::new_v4());

        let err = hoist_virtualization_owners(&subnets, &hosts, &services, &HashSet::new())
            .expect_err("an unresolvable owner must not be silently dropped");

        assert!(matches!(err, DemoSeedError::UnresolvableOwner { .. }));
    }

    #[test]
    fn a_second_layer_of_virtualization_is_rejected_rather_than_mis_ordered() {
        let (subnets, hosts, mut services) = dataset();
        // The runtime service is now itself virtualized by the guest service — one hoisted layer
        // can no longer order this batch.
        let guest_service_id = services[1].id;
        services[0].base.virtualization_service_id = Some(guest_service_id);

        let err = hoist_virtualization_owners(&subnets, &hosts, &services, &HashSet::new())
            .expect_err("a nested owner must fail loudly, not reach Postgres");

        assert!(matches!(err, DemoSeedError::NestedOwner { .. }));
    }
}
