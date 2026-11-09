//! Bulk insertion of a generated demo dataset.
//!
//! Split out of `handlers.rs` because it is the one part of the demo flow with real ordering
//! constraints — foreign keys between the entity types decide the sequence of writes — and
//! because taking a `&ServiceFactory` rather than an `AppState` makes it directly drivable from
//! `src/tests/demo_data_seeding.rs` against a testcontainers Postgres with the migrations applied.

use crate::server::auth::access::Assignment;
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::auth::service::hash_password;
use crate::server::bindings::r#impl::base::Binding;
use crate::server::hosts::r#impl::base::Host;
use crate::server::interface_neighbors::r#impl::base::Neighbor;
use crate::server::interfaces::r#impl::base::Interface;
use crate::server::organizations::demo_data::{DemoData, backdate_rows};
use crate::server::organizations::r#impl::base::UseCase;
use crate::server::services::r#impl::base::Service;
use crate::server::shared::services::factory::ServiceFactory;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::snapshot::Snapshotable;
use crate::server::shared::storage::traits::{Entity, Storable, Storage};
use crate::server::shared::types::api::{ApiError, ApiResult};
use crate::server::site_assignments::r#impl::base::Principal;
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

/// A neighbour link resolved from the dataset's host names, written once interfaces exist.
struct NeighborLink {
    site_id: Uuid,
    interface_id: Uuid,
    neighbor: Neighbor,
    /// When the row becomes valid: the source interface's `valid_from`.
    valid_from: DateTime<Utc>,
    /// When the link was last heard: the source interface's `last_seen_at`.
    seen_at: DateTime<Utc>,
}

mod owners;

use owners::{hoist_virtualization_owners, write_owner_layers};


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

/// [`backdate_rows`] for the rows the seeder builds itself on the snapshotted sites.
fn backdated<T: Snapshotable>(mut rows: Vec<T>, at: DateTime<Utc>) -> Vec<T> {
    backdate_rows(&mut rows, at);
    rows
}

/// Generate and insert a full demo dataset into an organization.
///
/// Assumes the organization has just been reset — there are no collisions, so this uses bulk
/// `create_many` throughout instead of the per-entity create/upsert paths.
pub(crate) async fn seed_demo_data(
    services: &ServiceFactory,
    organization_id: Uuid,
    user_id: Uuid,
    use_case: UseCase,
    entity: AuthenticatedEntity,
) -> ApiResult<()> {
    let demo_data = DemoData::generate(organization_id, user_id, use_case);
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
    let snapshot_taken_at = demo_data.snapshot_taken_at;
    let snapshot_site_ids = demo_data.snapshot_site_ids.clone();

    // Insert entities in dependency order using bulk inserts.
    // Since we just reset the org, there are no collisions — we use service-level
    // create_many (publishes one event per scope per entity type) instead of
    // per-entity create() for speed. Entities without event subscribers use
    // storage().create_many() directly.

    // 1. Tag groups, then the tags in them
    services
        .tag_group_service
        .storage()
        .create_many(&demo_data.tag_groups)
        .await?;
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

    // 3. Sites (depends on organization, tags)
    let created_sites = services
        .site_service
        .storage()
        .create_many(&demo_data.sites)
        .await?;
    collect_entity_tags(&created_sites, &mut all_entity_tags);

    // 3.5. Site-credential associations — one bulk insert across all
    // sites (no per-site lock/delete; the org was just reset).
    let site_cred_pairs: Vec<(Uuid, Uuid)> = demo_data
        .site_credential_assignments
        .iter()
        .flat_map(|a| {
            let site_id = a.site_id;
            a.credential_ids
                .iter()
                .map(move |&cred_id| (site_id, cred_id))
        })
        .collect();
    services
        .credential_service
        .create_site_credentials(&site_cred_pairs)
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
        let site_id = hws.host.base.site_id;
        all_hosts.push(hws.host.clone());
        all_ip_addresses.extend(hws.ip_addresses.clone());
        all_ports.extend(
            hws.ports
                .iter()
                .cloned()
                .map(|p| p.with_host(host_id, site_id)),
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
    let (_, created_owner_services) =
        write_owner_layers(services, &hoist, &entity, &mut all_entity_tags).await?;

    // 4. Subnets (depends on sites, and on the owner services written above)
    let created_subnets = services
        .subnet_service
        .create_many(&demo_data.subnets, entity.clone())
        .await?;
    collect_entity_tags(&created_subnets, &mut all_entity_tags);

    // 4.5. VLANs (depends on sites)
    let created_vlans = services
        .vlan_service
        .storage()
        .create_many(&demo_data.vlans)
        .await?;
    collect_entity_tags(&created_vlans, &mut all_entity_tags);

    // 4.6. Subnet↔VLAN junction rows (depend on subnets + VLANs). One bulk
    // insert, no per-subnet lock (fresh org). Derived to mirror the discovery
    // reconciler, so demo subnets show their VLANs like a real deployment.
    services
        .vlan_service
        .create_subnet_links(&demo_data.subnet_vlan_records)
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
    let neighbor_links: Vec<NeighborLink> = {
        // By title rather than stored name: demo neighbour pairings name hosts the way the UI
        // does, and some demo hosts are nameless and titled by an identifier.
        let host_id_to_name: HashMap<Uuid, String> = all_hosts
            .iter()
            .map(|h| (h.id, h.display_name(&[]).unwrap_or_default()))
            .collect();

        let host_name_to_id: HashMap<&str, Uuid> = host_id_to_name
            .iter()
            .map(|(&id, name)| (name.as_str(), id))
            .collect();

        let mut interface_lookup: HashMap<(String, Option<i32>), Uuid> = HashMap::new();
        let mut interface_context: HashMap<Uuid, &Interface> = HashMap::new();
        for entry in &demo_data.interfaces {
            if let Some(host_name) = host_id_to_name.get(&entry.base.host_id) {
                interface_lookup.insert((host_name.clone(), entry.base.if_index), entry.id);
            }
            interface_context.insert(entry.id, entry);
        }

        // A link row is valid from when its source interface is: on a snapshotted site that is
        // the snapshot time or earlier, so the snapshot holds the link too.
        let link = |source_id: Uuid, neighbor: Neighbor| {
            interface_context.get(&source_id).map(|iface| NeighborLink {
                site_id: iface.base.site_id,
                interface_id: source_id,
                neighbor,
                valid_from: iface.valid_from,
                seen_at: iface.last_seen_at,
            })
        };

        let mut links = Vec::new();
        for neighbor_update in &demo_data.neighbor_updates {
            let source_key = (
                neighbor_update.source_host_name.clone(),
                Some(neighbor_update.source_if_index),
            );
            let target_key = (
                neighbor_update.target_host_name.clone(),
                neighbor_update.target_if_index,
            );
            if let (Some(&source_id), Some(&target_id)) = (
                interface_lookup.get(&source_key),
                interface_lookup.get(&target_key),
            ) {
                links.extend(link(source_id, Neighbor::Interface(target_id)));
            }
        }
        for neighbor_update in &demo_data.host_neighbor_updates {
            let source_key = (
                neighbor_update.source_host_name.clone(),
                Some(neighbor_update.source_if_index),
            );
            if let (Some(&source_id), Some(&target_host_id)) = (
                interface_lookup.get(&source_key),
                host_name_to_id.get(neighbor_update.target_host_name.as_str()),
            ) {
                links.extend(link(source_id, Neighbor::Host(target_host_id)));
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
    for link in neighbor_links {
        services
            .interface_neighbor_service
            .reconcile_interface_neighbors(
                link.site_id,
                link.interface_id,
                &[(link.neighbor, Some(link.seen_at))],
                link.valid_from,
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
                .map(|b| b.with_service(s.id, s.base.site_id))
        })
        .collect();
    services
        .binding_service
        .create_many(&all_bindings, entity.clone())
        .await?;

    // 5.7. Host-credential assignments (depend on hosts + credentials). `create_many` writes the
    // host row only; `credential_assignments` lives in the `host_credentials` junction, which
    // nothing above writes. Without this the demo's host-pinned credentials (pfSense's SNMP
    // override, the Docker proxy, SSH inventory, Wake-on-LAN) were silently dropped.
    for host in all_hosts
        .iter()
        .filter(|h| !h.base.credential_assignments.is_empty())
    {
        services
            .credential_service
            .set_host_credentials(&host.id, &host.base.credential_assignments)
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    }

    // 6. Daemons (depends on hosts, sites, subnets). Inserted without their api key, which does
    // not exist yet: each key names its daemon and each daemon its key, so the daemon side of
    // the binding is written in 7.1, as provisioning does.
    let unbound_daemons: Vec<_> = demo_data
        .daemons
        .iter()
        .cloned()
        .map(|mut daemon| {
            daemon.base.api_key_id = None;
            daemon
        })
        .collect();
    services
        .daemon_service
        .storage()
        .create_many(&unbound_daemons)
        .await?;

    // 6.1. Daemon interfaced subnets (depend on daemons + subnets). Same service method the
    // heartbeat writes through.
    for (daemon_id, subnet_ids) in &demo_data.daemon_interfaced_subnets {
        services
            .daemon_service
            .set_interfaced_subnet_ids(daemon_id, subnet_ids)
            .await?;
    }

    // 7. Daemon API Keys (depends on sites and daemons)
    services
        .daemon_api_key_service
        .storage()
        .create_many(&demo_data.api_keys)
        .await?;

    // 7.1. Bind each daemon to its key.
    services
        .daemon_service
        .storage()
        .update_many(&demo_data.daemons)
        .await?;

    // 8. Discoveries (depends on daemons, sites, subnets)
    services
        .discovery_service
        .storage()
        .create_many(&demo_data.discoveries)
        .await?;
    // The historical runs' warnings are rows of their own, written beside each run.
    for discovery in &demo_data.discoveries {
        services
            .discovery_service
            .record_warnings(discovery)
            .await?;
    }

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
            .create_members(&backdated(member_records, snapshot_taken_at))
            .await?;
    }

    // 10. Topologies (depends on sites + the entities created above).
    // The graph is built on request from the persisted entities, so `create`
    // just persists the row + options. Must run before shares (step 11), whose
    // `topology_id` FK references these rows.
    for topology in demo_data.topologies {
        services
            .topology_service
            .create(topology, entity.clone())
            .await?;
    }

    // 10.5. Bulk insert all entity tags (single INSERT for all tagged entities). Only the
    // snapshotted sites' entities carry tags, so the tag rows are backdated with them.
    let all_entity_tags = backdated(all_entity_tags, snapshot_taken_at);
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
    services
        .access_service
        .set_access(
            Principal::User(DEMO_USER_ID),
            UserOrgPermissions::Admin,
            &Assignment::for_person(UserOrgPermissions::Admin, &[]),
            entity.clone(),
        )
        .await?;

    // 13. User API Keys (depends on demo admin user, whose access caps theirs)
    for (api_key, site_ids) in demo_data.user_api_keys {
        services
            .access_service
            .create_api_key(api_key, site_ids, entity.clone())
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    }

    // 14. One snapshot per hand-built site, dated a month back, so the snapshot UI is exercised
    // in demo orgs. Must run last: close-and-clone captures the live entity set, so all demo
    // entities (and their entity-tags + live topology rows) must already exist.
    // Each site's snapshot is scoped to its own site_id and runs in its
    // own transaction, so the sites' snapshots run concurrently.
    let snapshot_futures = created_sites
        .iter()
        .filter(|site| snapshot_site_ids.contains(&site.id))
        .map(|site| {
            let entity = entity.clone();
            async move {
                let snapshot = Snapshot {
                    base: SnapshotBase::new(site.id, snapshot_taken_at, Some(user_id)),
                    ..Default::default()
                };
                let created = services
                    .snapshot_service
                    .create(snapshot, entity)
                    .await
                    .map_err(ApiError::from)?;
                services
                    .snapshot_service
                    .run_close_and_clone(created.base.site_id, created.base.taken_at, created.id)
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
            let site_id = hws.host.base.site_id;
            recent_hosts.push(hws.host.clone());
            recent_ips.extend(hws.ip_addresses.clone());
            recent_ports.extend(
                hws.ports
                    .iter()
                    .cloned()
                    .map(|p| p.with_host(host_id, site_id)),
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
        let (_, created_recent_owner_services) =
            write_owner_layers(services, &recent_hoist, &entity, &mut recent_entity_tags).await?;

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
                    .map(|b| b.with_service(s.id, s.base.site_id))
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
