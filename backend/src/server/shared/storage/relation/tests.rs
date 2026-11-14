use uuid::Uuid;

use crate::server::shared::{entities::EntityDiscriminants as E, relations as R};

/// An `AnyEntity` relation's set is one entity's rows: an id shared by two entity types names
/// two sets.
#[test]
fn an_any_entity_relation_scopes_its_set_by_entity_type() {
    let id = Uuid::new_v4();
    let host = R::ENTITY_TAG_ENTITY.at_entity(E::Host, id);
    let service = R::ENTITY_TAG_ENTITY.at_entity(E::Service, id);

    assert_ne!(host.lock_key(), service.lock_key());
    assert!(host.filter().to_where_clause().contains("entity_type"));
}

#[test]
#[should_panic(expected = "call at_entity")]
fn an_any_entity_relation_refuses_a_key_without_its_entity_type() {
    R::ENTITY_TAG_ENTITY.at(Uuid::new_v4());
}

#[test]
#[should_panic(expected = "call at")]
fn a_single_entity_relation_refuses_a_second_entity_type() {
    R::ENTITY_TAG_TAG.at_entity(E::Host, Uuid::new_v4());
}

/// A junction row's two ends key two different sets: replacing a credential's sites leaves
/// another credential's rows on the same site alone.
#[test]
fn each_end_of_a_junction_keys_its_own_set() {
    let id = Uuid::new_v4();
    let by_site = R::SITE_CREDENTIAL_SITE.at(id);
    let by_credential = R::SITE_CREDENTIAL_CREDENTIAL.at(id);

    assert_ne!(by_site.lock_key(), by_credential.lock_key());
    assert!(by_site.filter().to_where_clause().contains("site_id"));
    assert!(
        by_credential
            .filter()
            .to_where_clause()
            .contains("credential_id")
    );
}
