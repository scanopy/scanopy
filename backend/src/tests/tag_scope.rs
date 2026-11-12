//! Saving an entity's tags checks only the tags it doesn't already hold against their group's
//! scope. An application tag left on a subnet from before the Application group was limited to
//! hosts and services stays through later saves, and can't be given to another subnet.

use uuid::Uuid;

use super::{organization, site, subnet, test_services};
use crate::server::{
    shared::{entities::EntityDiscriminants, storage::traits::Storage},
    tags::{
        entity_tags::{EntityTag, EntityTagBase},
        groups::base::{TagGroup, TagGroupBase, TagGroupType},
        r#impl::base::{Tag, TagBase},
    },
};

#[tokio::test]
async fn a_held_out_of_scope_tag_survives_a_save_but_cannot_be_added() {
    let (storage, services, _container) = test_services().await;

    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let s = site(&org.id);
    storage.sites.create(&s).await.unwrap();
    let legacy = subnet(&s.id);
    storage.subnets.create(&legacy).await.unwrap();
    let fresh = subnet(&s.id);
    storage.subnets.create(&fresh).await.unwrap();

    let now = crate::server::shared::clock::now();
    let group = TagGroup {
        id: Uuid::new_v4(),
        created_at: now,
        updated_at: now,
        base: TagGroupBase::new(org.id, "Application".into(), TagGroupType::Application),
    };
    storage.tag_groups.create(&group).await.unwrap();
    let app_tag = Tag {
        id: Uuid::new_v4(),
        base: TagBase {
            name: "Monitoring".to_string(),
            organization_id: org.id,
            tag_group_id: Some(group.id),
            ..Default::default()
        },
        ..Default::default()
    };
    storage.tags.create(&app_tag).await.unwrap();

    // Written straight to storage, as the v0.18.0 migration left it.
    services
        .entity_tag_service
        .create_many(&[EntityTag::new(EntityTagBase::new(
            legacy.id,
            EntityDiscriminants::Subnet,
            app_tag.id,
        ))])
        .await
        .unwrap();

    services
        .entity_tag_service
        .set_tags(
            legacy.id,
            EntityDiscriminants::Subnet,
            vec![app_tag.id],
            org.id,
        )
        .await
        .expect("a tag the subnet already holds must not block saving it");

    let refused = services
        .entity_tag_service
        .set_tags(
            fresh.id,
            EntityDiscriminants::Subnet,
            vec![app_tag.id],
            org.id,
        )
        .await;
    assert!(
        refused.is_err(),
        "an application tag must not be newly applied to a subnet"
    );
}
