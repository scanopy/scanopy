//! The virtualization tree is computed twice: in SQL, to group and order a paginated host list,
//! and in Rust, for the root and depth each response carries. The UI matches a row to its group
//! by the Rust root against the SQL group key, so the two must agree on every host.

use crate::server::hosts::handlers::{HostFilterQuery, HostOrderField};
use crate::server::hosts::r#impl::name::{HostName, HostNameSources};
use crate::server::shared::handlers::ordering::OrderField;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::Storage;

use super::{host, organization, service, site, test_services};

#[tokio::test]
async fn sql_and_rust_place_every_host_in_the_same_tree() {
    let (storage, services, _container) = test_services().await;

    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let net = site(&org.id);
    storage.sites.create(&net).await.unwrap();

    let named = |name: &str| {
        let mut h = host(&net.id);
        h.base.name = HostName::manual(name.to_string());
        h
    };

    let node = named("pve-01");
    storage.hosts.create(&node).await.unwrap();
    let node_runtime = service(&net.id, &node.id);
    storage.services.create(&node_runtime).await.unwrap();

    let mut vm = named("docker-vm");
    vm.base.virtualization_service_id = Some(node_runtime.id);
    storage.hosts.create(&vm).await.unwrap();
    let vm_runtime = service(&net.id, &vm.id);
    storage.services.create(&vm_runtime).await.unwrap();

    let mut container = named("pihole");
    container.base.virtualization_service_id = Some(vm_runtime.id);
    storage.hosts.create(&container).await.unwrap();

    let mut sibling = named("app-vm");
    sibling.base.virtualization_service_id = Some(node_runtime.id);
    storage.hosts.create(&sibling).await.unwrap();

    let lone = named("printer");
    storage.hosts.create(&lone).await.unwrap();

    let query = HostFilterQuery {
        group_by: Some(HostOrderField::VirtualizationTree),
        ..Default::default()
    };
    let (filter, order_by) =
        query.apply_ordering(StorableFilter::new_from_site_ids(&[net.id]).live());

    let page = services
        .host_service
        .get_all_host_responses_paginated(filter.clone(), &order_by, None, false)
        .await
        .unwrap();

    let names: Vec<&str> = page.items.iter().map(|h| h.name.as_str()).collect();
    assert_eq!(
        names,
        vec!["printer", "pve-01", "app-vm", "docker-vm", "pihole"],
        "no tree first, then the tree parent first with each subtree under its parent"
    );

    let depths: Vec<u32> = page.items.iter().map(|h| h.virtualization_depth).collect();
    assert_eq!(depths, vec![0, 0, 1, 1, 2]);

    // The groups the Rust roots imply, in page order, against the SQL's own groups.
    let mut rust_groups: Vec<(String, u64)> = Vec::new();
    for h in &page.items {
        let key = h
            .virtualization_root_host_id
            .map(|id| id.to_string())
            .unwrap_or_default();
        match rust_groups.last_mut() {
            Some((last, count)) if *last == key => *count += 1,
            _ => rust_groups.push((key, 1)),
        }
    }
    let sql_groups: Vec<(String, u64)> = services
        .host_service
        .count_by_group(filter, HostOrderField::VirtualizationTree.to_sql())
        .await
        .unwrap()
        .into_iter()
        .map(|g| (g.value.unwrap_or_default(), g.count))
        .collect();
    assert_eq!(rust_groups, sql_groups);
    assert_eq!(
        rust_groups,
        vec![(String::new(), 1), (node.id.to_string(), 4)]
    );
}
