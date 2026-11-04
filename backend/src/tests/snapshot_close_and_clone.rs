//! Taking a snapshot closes a copy of every live row on the site, stamps the copy with the
//! snapshot, points the copy's foreign keys at the closed parents, and moves the live rows'
//! `valid_from` to the snapshot time.

use std::fmt::Display;

use chrono::Duration;
use uuid::Uuid;

use super::{
    dependency, host, ip_address, organization, port, service, site, subnet, test_services,
};
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    bindings::r#impl::base::{Binding, BindingBase, BindingType},
    dependencies::{
        dependency_members::DependencyMemberRecord,
        r#impl::base::{Dependency, DependencyMembers},
    },
    hosts::r#impl::base::Host,
    interface_neighbors::r#impl::base::{InterfaceNeighborInterface, Neighbor},
    interfaces::r#impl::base::{Interface, InterfaceBase},
    ip_addresses::r#impl::base::IPAddress,
    ports::r#impl::base::Port,
    services::r#impl::base::Service,
    shared::{
        attribution::AttributeSource,
        entities::EntityDiscriminants,
        services::traits::CrudService,
        storage::{
            factory::StorageFactory,
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            snapshot::Snapshotable,
            traits::{Storable, Storage},
        },
    },
    snapshots::types::base::Snapshot,
    subnets::r#impl::base::Subnet,
    tags::{
        entity_tags::EntityTag,
        r#impl::base::{Tag, TagBase},
    },
    vlans::r#impl::{
        base::{Vlan, VlanBase, VlanName, VlanNameValue},
        subnet_vlans::SubnetVlanRecord,
    },
};

/// The rows of `T` stamped with `snapshot_id`.
async fn closed<T: Storable + Display>(storage: &StorageFactory, snapshot_id: Uuid) -> Vec<T> {
    GenericPostgresStorage::<T>::new(storage.pool().clone())
        .get_all(StorableFilter::<T>::new().snapshot_id(&snapshot_id))
        .await
        .unwrap()
}

/// The one row of `T` stamped with `snapshot_id`.
async fn one_closed<T: Storable + Display>(storage: &StorageFactory, snapshot_id: Uuid) -> T {
    let mut rows = closed::<T>(storage, snapshot_id).await;
    assert_eq!(rows.len(), 1, "one closed {} row", T::table_name());
    rows.remove(0)
}

#[tokio::test]
async fn a_snapshot_closes_every_site_row_onto_closed_parents() {
    let (storage, services, _container) = test_services().await;

    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let s = site(&org.id);
    storage.sites.create(&s).await.unwrap();

    let h = host(&s.id);
    storage.hosts.create(&h).await.unwrap();
    let sn = subnet(&s.id);
    storage.subnets.create(&sn).await.unwrap();
    let mut ip = ip_address(&s.id, &sn.id);
    ip.base.host_id = h.id;
    storage.ip_addresses.create(&ip).await.unwrap();
    let p = port(&s.id, &h.id);
    storage.ports.create(&p).await.unwrap();
    let svc = service(&s.id, &h.id);
    storage.services.create(&svc).await.unwrap();
    let b = Binding::new(BindingBase::new(
        svc.id,
        s.id,
        BindingType::Port {
            port_id: p.id,
            ip_address_id: Some(ip.id),
        },
    ));
    storage.bindings.create(&b).await.unwrap();

    let mut interfaces = Vec::new();
    for if_index in [1, 2] {
        let i = Interface::new(InterfaceBase {
            host_id: h.id,
            site_id: s.id,
            if_index: Some(if_index),
            ..Default::default()
        });
        storage.interfaces.create(&i).await.unwrap();
        interfaces.push(i.id);
    }
    services
        .interface_neighbor_service
        .reconcile_interface_neighbors(
            s.id,
            interfaces[0],
            &[(Neighbor::Interface(interfaces[1]), None)],
            crate::server::shared::clock::now(),
            None,
        )
        .await
        .unwrap();

    let vlan = Vlan::new(VlanBase {
        vlan_number: 10,
        name: VlanName::new(VlanNameValue("users".to_string()), AttributeSource::Manual),
        site_id: s.id,
        organization_id: org.id,
        ..Default::default()
    });
    storage.vlans.create(&vlan).await.unwrap();
    services
        .vlan_service
        .set_subnet_vlans(&sn.id, &[vlan.id])
        .await
        .unwrap();

    let mut dep = dependency(&s.id);
    dep.base.members = DependencyMembers::Bindings {
        binding_ids: vec![b.id],
    };
    let dep: Dependency = services
        .dependency_service
        .create(dep, AuthenticatedEntity::System)
        .await
        .unwrap();

    let tag = Tag::new(TagBase {
        name: "core".to_string(),
        organization_id: org.id,
        ..Default::default()
    });
    storage.tags.create(&tag).await.unwrap();
    services
        .entity_tag_service
        .set_tags(h.id, EntityDiscriminants::Host, vec![tag.id], org.id)
        .await
        .unwrap();

    let taken_at = crate::server::shared::clock::now() + Duration::seconds(1);
    let snapshot = Snapshot::new(s.id, taken_at, None);
    storage.snapshots.create(&snapshot).await.unwrap();
    services
        .snapshot_service
        .run_close_and_clone(s.id, taken_at, snapshot.id)
        .await
        .unwrap();

    let host_copy: Host = one_closed(&storage, snapshot.id).await;
    let subnet_copy: Subnet = one_closed(&storage, snapshot.id).await;
    let vlan_copy: Vlan = one_closed(&storage, snapshot.id).await;
    let ip_copy: IPAddress = one_closed(&storage, snapshot.id).await;
    let port_copy: Port = one_closed(&storage, snapshot.id).await;
    let service_copy: Service = one_closed(&storage, snapshot.id).await;
    let binding_copy: Binding = one_closed(&storage, snapshot.id).await;
    let dependency_copy: Dependency = one_closed(&storage, snapshot.id).await;
    let member_copy: DependencyMemberRecord = one_closed(&storage, snapshot.id).await;
    let link_copy: SubnetVlanRecord = one_closed(&storage, snapshot.id).await;
    let tag_copy: EntityTag = one_closed(&storage, snapshot.id).await;
    let neighbor_copy: InterfaceNeighborInterface = one_closed(&storage, snapshot.id).await;
    let interface_copies: Vec<Interface> = closed(&storage, snapshot.id).await;
    assert_eq!(interface_copies.len(), 2);

    // Every copy is a new, closed row.
    assert_ne!(host_copy.id, h.id);
    assert_ne!(dependency_copy.id, dep.id);
    assert_eq!(host_copy.valid_to(), Some(taken_at));
    assert_eq!(member_copy.valid_to, Some(taken_at));

    // Copies point at the closed parents.
    assert_eq!(ip_copy.base.host_id, host_copy.id);
    assert_eq!(ip_copy.base.subnet_id, subnet_copy.id);
    assert_eq!(port_copy.base.host_id, host_copy.id);
    assert_eq!(service_copy.base.host_id, host_copy.id);
    assert_eq!(binding_copy.base.service_id, service_copy.id);
    assert_eq!(member_copy.base.dependency_id, dependency_copy.id);
    assert_eq!(member_copy.base.service_id, service_copy.id);
    assert_eq!(member_copy.base.binding_id, Some(binding_copy.id));
    assert_eq!(link_copy.subnet_id(), subnet_copy.id);
    assert_eq!(link_copy.vlan_id(), vlan_copy.id);
    assert_eq!(tag_copy.base.entity_id, host_copy.id);
    let interface_copy_ids: Vec<Uuid> = interface_copies.iter().map(|i| i.id).collect();
    assert!(
        interface_copies
            .iter()
            .all(|i| i.base.host_id == host_copy.id)
    );
    assert!(interface_copy_ids.contains(&neighbor_copy.base.interface_id));
    assert!(interface_copy_ids.contains(&neighbor_copy.base.neighbor_interface_id));

    // Live rows stay, starting at the snapshot.
    let live_host = storage.hosts.get_by_id(&h.id).await.unwrap().unwrap();
    assert_eq!(live_host.valid_from(), taken_at);
    assert_eq!(live_host.valid_to(), None);
}
