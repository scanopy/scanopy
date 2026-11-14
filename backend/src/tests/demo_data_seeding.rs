//! Coverage for the demo seeding write path — `organizations::demo_seed`.
//!
//! Demo seeding deliberately bypasses `discover_host` ("no collisions in fresh org") and writes
//! through bulk `create_many`, so none of the ordering the discovery path grew for GH #650
//! applies to it. When `virtualization_service_id` became a real foreign key on hosts, services
//! and subnets, that left the seeder writing subnets and hosts that name services it has not
//! created yet.
//!
//! These run against a testcontainers Postgres with the migrations applied, so the foreign keys
//! are the real ones. Same vehicle as `host_create_with_children.rs`.

use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::hosts::r#impl::base::Host;
use crate::server::organizations::demo_data::DemoData;
use crate::server::organizations::demo_seed::insert_demo_data;
use crate::server::organizations::handlers::DEMO_USER_ID;
use crate::server::services::r#impl::base::Service;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::Storage;
use crate::server::subnets::r#impl::base::Subnet;

use super::{organization, test_services, user};

/// Every `virtualization_service_id` the dataset declares, as `(row id, owning service id)`,
/// split by the table the row lands in.
#[derive(Default)]
struct DeclaredOwners {
    subnets: Vec<(Uuid, Uuid)>,
    hosts: Vec<(Uuid, Uuid)>,
    services: Vec<(Uuid, Uuid)>,
}

impl DeclaredOwners {
    fn from(demo_data: &DemoData) -> Self {
        let mut declared = Self::default();

        for subnet in &demo_data.subnets {
            if let Some(owner) = subnet.base.virtualization_service_id {
                declared.subnets.push((subnet.id, owner));
            }
        }

        let bundles = demo_data
            .hosts_with_services
            .iter()
            .chain(&demo_data.recent_hosts_with_services);
        for hws in bundles {
            if let Some(owner) = hws.host.base.virtualization_service_id {
                declared.hosts.push((hws.host.id, owner));
            }
            for svc in &hws.services {
                if let Some(owner) = svc.base.virtualization_service_id {
                    declared.services.push((svc.id, owner));
                }
            }
        }

        declared
    }

    fn is_empty(&self) -> bool {
        self.subnets.is_empty() && self.hosts.is_empty() && self.services.is_empty()
    }
}

/// The seeder's own guarantee, asserted against real foreign keys.
///
/// Red before the reorder: the DockerBridge subnets name Docker daemon services that step 5.5
/// has not written yet, so step 4 aborts on `subnets_virtualization_service_id_fkey`.
///
/// Green only if every declared relationship survives. A "fix" that satisfies the constraint by
/// setting these ids to `None` seeds cleanly and fails here, which is the point — a demo whose
/// containers are not under their runtime and whose VMs are not under their hypervisor is
/// worthless, and nothing else in the suite would notice.
#[tokio::test]
async fn demo_seed_persists_every_virtualization_relationship() {
    let (storage, services, _container) = test_services().await;

    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let owner = user(&org.id);
    storage.users.create(&owner).await.unwrap();

    // Held rather than regenerated: `DemoData::generate` mints fresh UUIDs every call.
    let demo_data = DemoData::generate(org.id, owner.id, org.base.use_case);
    let declared = DeclaredOwners::from(&demo_data);
    assert!(
        !declared.is_empty(),
        "demo data declares no virtualization at all — the fixture this test guards is gone"
    );

    insert_demo_data(
        &services,
        demo_data,
        org.id,
        owner.id,
        AuthenticatedEntity::System,
    )
    .await
    .expect("demo seeding must satisfy the virtualization_service_id foreign keys");

    // Live rows only: step 14 snapshots each site, and close-and-clone leaves a closed copy of
    // every entity behind. Counting those would double every figure below.
    let subnets: Vec<Subnet> = storage
        .subnets
        .get_all(StorableFilter::new_unfiltered().live())
        .await
        .unwrap();
    let hosts: Vec<Host> = storage
        .hosts
        .get_all(StorableFilter::new_unfiltered().live())
        .await
        .unwrap();
    let persisted_services: Vec<Service> = storage
        .services
        .get_all(StorableFilter::new_unfiltered().live())
        .await
        .unwrap();

    let live_service_ids: HashSet<Uuid> = persisted_services.iter().map(|s| s.id).collect();

    let subnet_owners: HashMap<Uuid, Option<Uuid>> = subnets
        .iter()
        .map(|s| (s.id, s.base.virtualization_service_id))
        .collect();
    let host_owners: HashMap<Uuid, Option<Uuid>> = hosts
        .iter()
        .map(|h| (h.id, h.base.virtualization_service_id))
        .collect();
    let service_owners: HashMap<Uuid, Option<Uuid>> = persisted_services
        .iter()
        .map(|s| (s.id, s.base.virtualization_service_id))
        .collect();

    let cases = [
        ("subnets", &declared.subnets, &subnet_owners),
        ("hosts", &declared.hosts, &host_owners),
        ("services", &declared.services, &service_owners),
    ];

    for (table, declared_rows, persisted) in cases {
        for (row_id, expected_owner) in declared_rows {
            let actual = persisted.get(row_id).copied().unwrap_or_else(|| {
                panic!("{table}: declared row {row_id} was never persisted");
            });
            // Identity, not merely non-NULL: a row repointed at some *other* service would
            // still resolve and would still be wrong.
            assert_eq!(
                actual,
                Some(*expected_owner),
                "{table}: row {row_id} should be virtualized by {expected_owner}"
            );
            assert!(
                live_service_ids.contains(expected_owner),
                "{table}: row {row_id} names service {expected_owner}, which does not exist"
            );
        }

        // The other direction: nothing acquired an owner the dataset never declared, and the
        // per-table totals match. Catches a partial seed as well as a silent downgrade.
        let declared_ids: HashSet<Uuid> = declared_rows.iter().map(|(id, _)| *id).collect();
        let persisted_ids: HashSet<Uuid> = persisted
            .iter()
            .filter(|(_, owner)| owner.is_some())
            .map(|(id, _)| *id)
            .collect();
        assert_eq!(
            persisted_ids, declared_ids,
            "{table}: rows carrying a virtualization_service_id differ from those declared"
        );
    }

    // Nothing anywhere points at a service that is gone — including the closed snapshot copies,
    // which the foreign key covers too.
    for (table, all_rows) in [
        (
            "subnets",
            storage
                .subnets
                .get_all(StorableFilter::<Subnet>::new_unfiltered())
                .await
                .unwrap()
                .iter()
                .filter_map(|s| s.base.virtualization_service_id)
                .collect::<Vec<_>>(),
        ),
        (
            "hosts",
            storage
                .hosts
                .get_all(StorableFilter::<Host>::new_unfiltered())
                .await
                .unwrap()
                .iter()
                .filter_map(|h| h.base.virtualization_service_id)
                .collect::<Vec<_>>(),
        ),
        (
            "services",
            storage
                .services
                .get_all(StorableFilter::<Service>::new_unfiltered())
                .await
                .unwrap()
                .iter()
                .filter_map(|s| s.base.virtualization_service_id)
                .collect::<Vec<_>>(),
        ),
    ] {
        let all_service_ids: HashSet<Uuid> = storage
            .services
            .get_all(StorableFilter::<Service>::new_unfiltered())
            .await
            .unwrap()
            .iter()
            .map(|s| s.id)
            .collect();
        for owner in all_rows {
            assert!(
                all_service_ids.contains(&owner),
                "{table}: a row names service {owner}, which does not exist"
            );
        }
    }

    println!(
        "virtualization relationships persisted — subnets: {}, hosts: {}, services: {}",
        declared.subnets.len(),
        declared.hosts.len(),
        declared.services.len()
    );
}

/// The two seeded rows that live outside the entity tables: daemon interfaced subnets (a junction
/// heartbeats normally fill) and the demo login's user API key (written last, after the demo user
/// exists). Both are read back through the paths the daemons page and API keys page use.
#[tokio::test]
async fn demo_seed_writes_daemon_subnets_and_the_demo_logins_api_keys() {
    let (storage, services, _container) = test_services().await;

    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let owner = user(&org.id);
    storage.users.create(&owner).await.unwrap();

    let demo_data = DemoData::generate(org.id, owner.id, org.base.use_case);
    let declared_subnets: HashMap<Uuid, HashSet<Uuid>> = demo_data
        .daemon_interfaced_subnets
        .iter()
        .map(|(daemon_id, subnet_ids)| (*daemon_id, subnet_ids.iter().copied().collect()))
        .collect();
    assert!(!declared_subnets.is_empty());

    insert_demo_data(
        &services,
        demo_data,
        org.id,
        owner.id,
        AuthenticatedEntity::System,
    )
    .await
    .expect("demo seeding succeeds");

    for (daemon_id, expected) in &declared_subnets {
        let persisted: HashSet<Uuid> = services
            .daemon_service
            .get_interfaced_subnet_ids(daemon_id)
            .await
            .into_iter()
            .collect();
        assert_eq!(
            &persisted, expected,
            "daemon {daemon_id} interfaced subnets"
        );
    }

    let demo_login_keys = services
        .user_api_key_service
        .get_for_user(&DEMO_USER_ID)
        .await
        .unwrap();
    assert_eq!(
        demo_login_keys.len(),
        2,
        "the demo login should own both seeded user API keys"
    );
    assert!(
        demo_login_keys.iter().all(|key| key
            .base
            .site_ids
            .as_ref()
            .is_none_or(|ids| !ids.is_empty()))
    );
}

/// Everything the resolve modal shows for an unconfirmed host, for every unconfirmed demo host, as
/// the list endpoint returns it: who reported it, the scan that found it, and an identifier to know
/// it by (a chassis ID, or the MAC on its interface).
///
/// Each of these has gone missing once without anything failing: the reporter while
/// `host_responses` skipped the fill, and the scan for the untitled forwarding-table host while the
/// demo's warning builder skipped a host with no title.
#[tokio::test]
async fn demo_unconfirmed_hosts_carry_their_reporter_scan_and_identifier() {
    let (storage, services, _container) = test_services().await;

    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let owner = user(&org.id);
    storage.users.create(&owner).await.unwrap();

    let demo_data = DemoData::generate(org.id, owner.id, org.base.use_case);
    insert_demo_data(
        &services,
        demo_data,
        org.id,
        owner.id,
        AuthenticatedEntity::System,
    )
    .await
    .unwrap();

    let hosts: Vec<Host> = services
        .host_service
        .get_all(
            StorableFilter::<Host>::new_unfiltered()
                .live()
                .unconfirmed(true),
        )
        .await
        .unwrap();
    assert_eq!(hosts.len(), 2, "the demo seeds two unconfirmed hosts");
    let switch = services
        .host_service
        .get_all(StorableFilter::<Host>::new_unfiltered().live())
        .await
        .unwrap()
        .into_iter()
        .find(|h| h.base.name.value().as_str() == "unifi-usw-48")
        .expect("the HQ switch is seeded");

    // With children, as the Hosts list asks for them: the MAC rides on an interface.
    let responses = services
        .host_service
        .host_responses(hosts, None, true)
        .await
        .unwrap();
    for response in responses {
        assert_eq!(
            response.reported_by_host_ids,
            vec![switch.id],
            "unconfirmed host {} should be reported by the HQ switch",
            response.id
        );
        assert!(
            response.found_by_discovery_id.is_some(),
            "unconfirmed host {} should link to the scan that found it",
            response.id
        );
        let identified = response.chassis_id.is_some()
            || response
                .interfaces
                .iter()
                .any(|i| i.base.mac_address.is_some());
        assert!(
            identified,
            "unconfirmed host {} should carry a chassis ID or an interface MAC",
            response.id
        );
    }
}

/// The Plant site's estate seeds with every link written, and its L2 view draws the tiers joined:
/// every access switch to a distribution switch, every distribution switch to a core.
#[tokio::test]
async fn plant_site_seeds_and_its_l2_view_links_the_tiers() {
    use crate::server::interface_neighbors::r#impl::base::{
        InterfaceNeighborHost, InterfaceNeighborInterface,
    };
    use crate::server::organizations::demo_data::large_estate::PLANT_SITE_NAME;
    use crate::server::topology::types::edges::EdgeType;
    use crate::server::topology::types::views::TopologyView;

    let (storage, services, _container) = test_services().await;
    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let owner = user(&org.id);
    storage.users.create(&owner).await.unwrap();

    let demo_data = DemoData::generate(org.id, owner.id, org.base.use_case);
    let plant = demo_data
        .sites
        .iter()
        .find(|s| s.base.name == PLANT_SITE_NAME)
        .expect("the demo seeds the Plant site")
        .id;
    // Tier by the description each switch is generated with.
    let tier: HashMap<Uuid, String> = demo_data
        .hosts_with_services
        .iter()
        .filter(|h| h.host.base.site_id == plant)
        .filter_map(|h| Some((h.host.id, h.host.base.description.clone()?)))
        .collect();
    let plant_host_names: HashSet<String> = demo_data
        .hosts_with_services
        .iter()
        .filter(|h| h.host.base.site_id == plant)
        .map(|h| h.host.base.name.to_string())
        .collect();
    let port_links = demo_data
        .neighbor_updates
        .iter()
        .filter(|n| plant_host_names.contains(&n.source_host_name))
        .count();
    let device_links = demo_data.host_neighbor_updates.len();

    insert_demo_data(
        &services,
        demo_data,
        org.id,
        owner.id,
        AuthenticatedEntity::System,
    )
    .await
    .expect("the Plant estate seeds with no foreign-key errors");

    macro_rules! site_live {
        () => {
            StorableFilter::new_from_site_ids(&[plant]).live()
        };
    }
    let interface_rows: Vec<InterfaceNeighborInterface> = storage
        .interface_neighbor_interfaces
        .get_all(site_live!())
        .await
        .unwrap();
    let host_rows: Vec<InterfaceNeighborHost> = storage
        .interface_neighbor_hosts
        .get_all(site_live!())
        .await
        .unwrap();
    assert_eq!(
        interface_rows.len(),
        port_links,
        "a port-to-port link was dropped"
    );
    assert_eq!(
        host_rows.len(),
        device_links,
        "a device-level link was dropped"
    );

    let interface_host: HashMap<Uuid, Uuid> = storage
        .interfaces
        .get_all(site_live!())
        .await
        .unwrap()
        .into_iter()
        .map(|i| (i.id, i.base.host_id))
        .collect();
    let data = services
        .topology_service
        .get_topology_render_data(plant, None, &[TopologyView::L2Physical])
        .await
        .unwrap();
    let mut adjacent: HashMap<Uuid, HashSet<Uuid>> = HashMap::new();
    for edge in &data.edges[&TopologyView::L2Physical] {
        let pair = match edge.edge_type {
            EdgeType::PhysicalLink {
                source_entity_id,
                target_entity_id,
                ..
            } => (
                interface_host[&source_entity_id],
                interface_host[&target_entity_id],
            ),
            EdgeType::NeighborLink {
                source_host_id,
                target_host_id,
                ..
            } => (source_host_id, target_host_id),
            _ => continue,
        };
        adjacent.entry(pair.0).or_default().insert(pair.1);
        adjacent.entry(pair.1).or_default().insert(pair.0);
    }

    let in_tier = |name: &str| -> Vec<Uuid> {
        tier.iter()
            .filter(|(_, d)| d.as_str() == name)
            .map(|(id, _)| *id)
            .collect()
    };
    let linked_to = |host: Uuid, upper: &str| {
        adjacent.get(&host).is_some_and(|peers| {
            peers
                .iter()
                .any(|p| tier.get(p).map(String::as_str) == Some(upper))
        })
    };
    let access = in_tier("Access switch");
    let distribution = in_tier("Distribution switch");
    assert!(!access.is_empty() && !distribution.is_empty());
    for host in access {
        assert!(
            linked_to(host, "Distribution switch"),
            "access switch {host} has no uplink drawn"
        );
    }
    for host in distribution {
        assert!(
            linked_to(host, "Core switch"),
            "distribution switch {host} has no core link drawn"
        );
    }
}

/// The hand-built sites are snapshotted a month back, Plant is not, and every closed copy the
/// snapshot left opens before it closes.
#[tokio::test]
async fn hand_built_sites_snapshot_is_a_month_old_and_its_rows_cover_real_intervals() {
    use crate::server::shared::storage::snapshot::Snapshotable;
    use crate::server::snapshots::types::base::Snapshot;

    let (storage, services, _container) = test_services().await;
    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let owner = user(&org.id);
    storage.users.create(&owner).await.unwrap();

    let demo_data = DemoData::generate(org.id, owner.id, org.base.use_case);
    let snapshot_site_ids: HashSet<Uuid> = demo_data.snapshot_site_ids.iter().copied().collect();
    insert_demo_data(
        &services,
        demo_data,
        org.id,
        owner.id,
        AuthenticatedEntity::System,
    )
    .await
    .unwrap();

    let snapshots: Vec<Snapshot> = storage
        .snapshots
        .get_all(StorableFilter::new_unfiltered())
        .await
        .unwrap();
    let snapshotted: HashSet<Uuid> = snapshots.iter().map(|s| s.base.site_id).collect();
    assert_eq!(snapshotted, snapshot_site_ids);

    let now = crate::server::shared::clock::now();
    for snapshot in &snapshots {
        let age = now - snapshot.base.taken_at;
        assert!(
            (age - chrono::Duration::days(30)).num_minutes().abs() < 60,
            "snapshot is {} days old",
            age.num_days()
        );

        fn assert_intervals<T: Snapshotable>(rows: &[T], table: &str) {
            for row in rows {
                let valid_to = row.valid_to().expect("a snapshot row is closed");
                assert!(
                    row.valid_from() <= valid_to,
                    "a closed {table} row ends before it starts"
                );
            }
        }
        macro_rules! filter {
            () => {
                StorableFilter::new_unfiltered().snapshot_id(&snapshot.id)
            };
        }
        let hosts = storage.hosts.get_all(filter!()).await.unwrap();
        assert!(!hosts.is_empty(), "the snapshot holds the site's hosts");
        assert_intervals(&hosts, "hosts");
        assert_intervals(
            &storage.ip_addresses.get_all(filter!()).await.unwrap(),
            "ip_addresses",
        );
        assert_intervals(&storage.ports.get_all(filter!()).await.unwrap(), "ports");
        assert_intervals(
            &storage.services.get_all(filter!()).await.unwrap(),
            "services",
        );
        assert_intervals(
            &storage.bindings.get_all(filter!()).await.unwrap(),
            "bindings",
        );
        assert_intervals(
            &storage.interfaces.get_all(filter!()).await.unwrap(),
            "interfaces",
        );
        assert_intervals(
            &storage.subnets.get_all(filter!()).await.unwrap(),
            "subnets",
        );
        assert_intervals(&storage.vlans.get_all(filter!()).await.unwrap(), "vlans");
        assert_intervals(
            &storage.dependencies.get_all(filter!()).await.unwrap(),
            "dependencies",
        );
        assert_intervals(
            &storage.entity_tags.get_all(filter!()).await.unwrap(),
            "entity_tags",
        );
        assert_intervals(
            &storage.dependency_members.get_all(filter!()).await.unwrap(),
            "dependency_members",
        );
        assert_intervals(
            &storage.subnet_vlan.get_all(filter!()).await.unwrap(),
            "subnet_vlans",
        );
        assert_intervals(
            &storage
                .interface_neighbor_interfaces
                .get_all(filter!())
                .await
                .unwrap(),
            "interface_neighbor_interfaces",
        );
    }
}
