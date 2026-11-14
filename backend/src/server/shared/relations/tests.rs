use super::*;
use crate::server::{
    auth::access::{Assignment, AssignmentScope},
    discovery_warnings::r#impl::base::DiscoveryWarningRecordBase,
    interface_neighbors::r#impl::base::{
        InterfaceNeighborCandidateBase, InterfaceNeighborEvidence, InterfaceNeighborHostBase,
        InterfaceNeighborInterfaceBase,
    },
    interfaces::r#impl::base::InterfaceBase,
    ip_addresses::r#impl::base::IPAddressBase,
    ports::r#impl::base::PortBase,
    services::r#impl::base::ServiceBase,
    shared::{
        storage::{
            relation::Target,
            traits::{Entity, SqlValue, Storable},
        },
        types::entity_field::Hop,
    },
    site_assignments::r#impl::base::SiteAssignmentBase,
    tags::entity_tags::EntityTagBase,
    vlans::r#impl::subnet_vlans::SubnetVlanRecordBase,
};
use uuid::Uuid;

fn id() -> Uuid {
    Uuid::new_v4()
}

fn stored_ids(value: &SqlValue) -> Vec<Uuid> {
    match value {
        SqlValue::Uuid(id) => vec![*id],
        SqlValue::OptionalUuid(id) => id.iter().copied().collect(),
        SqlValue::UuidArray(ids) => ids.clone(),
        SqlValue::OptionVecUuid(ids) | SqlValue::OptionalUuidVec(ids) => {
            ids.clone().unwrap_or_default()
        }
        _ => panic!("not an id column"),
    }
}

/// Each of `T`'s relations reads the id its column stores. Run on a row whose id columns all
/// hold distinct ids, so reading the wrong field fails.
fn reads_match_columns<T: Storable>(row: &T) {
    let (columns, values) = row.to_params().expect("row serializes");
    assert!(
        !T::relations().is_empty(),
        "{} declares no relations",
        T::table_name()
    );
    for relation in T::relations() {
        let index = columns
            .iter()
            .position(|column| *column == relation.column)
            .unwrap_or_else(|| panic!("{} has no {} column", T::table_name(), relation.column));
        assert_eq!(
            relation.ids_of(row),
            stored_ids(&values[index]),
            "{}.{} reads another field",
            T::table_name(),
            relation.column,
        );
    }
}

#[test]
fn every_relation_reads_the_column_it_names() {
    reads_match_columns(&IPAddress::new(IPAddressBase {
        host_id: id(),
        subnet_id: id(),
        ..Default::default()
    }));
    reads_match_columns(&Interface::new(InterfaceBase {
        host_id: id(),
        ip_address_id: Some(id()),
        ..Default::default()
    }));
    reads_match_columns(&Service {
        id: id(),
        base: ServiceBase {
            host_id: id(),
            virtualization_service_id: Some(id()),
            ..Default::default()
        },
        ..Default::default()
    });
    reads_match_columns(&Port::new(PortBase {
        host_id: id(),
        ..Default::default()
    }));
    reads_match_columns(&Binding::new_ip_address(id(), id(), id()));
    reads_match_columns(&Binding::new_port(id(), id(), id(), Some(id())));
    reads_match_columns(&InterfaceNeighborCandidate::new(
        InterfaceNeighborCandidateBase::new(id(), id(), InterfaceNeighborEvidence::default()),
    ));
    reads_match_columns(&InterfaceNeighborInterface::new(
        InterfaceNeighborInterfaceBase {
            site_id: id(),
            interface_id: id(),
            neighbor_interface_id: id(),
            ..Default::default()
        },
    ));
    reads_match_columns(&InterfaceNeighborHost::new(InterfaceNeighborHostBase {
        site_id: id(),
        interface_id: id(),
        neighbor_host_id: id(),
        ..Default::default()
    }));
    reads_match_columns(&SubnetVlanRecord::new(SubnetVlanRecordBase::new(
        id(),
        id(),
    )));
    reads_match_columns(&DaemonInterfacedSubnet {
        daemon_id: id(),
        subnet_id: id(),
    });
    reads_match_columns(&SiteCredential {
        site_id: id(),
        credential_id: id(),
    });
    reads_match_columns(&HostCredential {
        host_id: id(),
        credential_id: id(),
        ip_address_ids: Some(vec![id()]),
    });
    reads_match_columns(&EntityTag::new(EntityTagBase::new(id(), E::Host, id())));
    reads_match_columns(&DependencyMemberRecord::new(
        crate::server::dependencies::dependency_members::DependencyMemberRecordBase {
            dependency_id: id(),
            service_id: id(),
            binding_id: Some(id()),
            position: 0,
        },
    ));
    for principal in [
        Principal::User(id()),
        Principal::ApiKey(id()),
        Principal::Invite(id()),
    ] {
        reads_match_columns(&SiteAssignment {
            id: id(),
            created_at: Default::default(),
            base: SiteAssignmentBase {
                organization_id: id(),
                principal,
                assignment: Assignment {
                    role: Default::default(),
                    scope: AssignmentScope::Site(id()),
                },
            },
        });
    }
    reads_match_columns(&DiscoveryWarningRecord::new(DiscoveryWarningRecordBase {
        discovery_id: id(),
        site_id: id(),
        ..Default::default()
    }));
}

/// The registry lists every reference an entity's fields hold. An owner relation of a registry
/// entity is one of them: some field's first hop names the same entity and key.
fn owners_appear_in_the_registry<T: Entity>() {
    let entity = T::ENTITY_TYPE;
    for relation in T::relations() {
        let Target::Entity(to) = relation.target else {
            continue;
        };
        let listed = entity.fields().iter().any(|field| {
            field.path.first()
                == Some(&Hop::Parent {
                    entity: to,
                    key: relation.column,
                })
        });
        assert!(
            listed,
            "{entity:?} relation {} isn't reached by any registry field",
            relation.column,
        );
    }
}

#[test]
fn every_relation_of_a_registry_entity_is_a_registry_reference() {
    owners_appear_in_the_registry::<IPAddress>();
    owners_appear_in_the_registry::<Interface>();
    owners_appear_in_the_registry::<Service>();
    owners_appear_in_the_registry::<Port>();
    owners_appear_in_the_registry::<Binding>();
}
