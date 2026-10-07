//! The global search returns, from each entity type's service, only rows the caller could list:
//! their organization, their sites, and the types their role may see.

use std::collections::HashSet;

use email_address::EmailAddress;
use uuid::Uuid;

use crate::server::hosts::r#impl::name::{HostName, HostNameSources};
use crate::server::search::scope::{SearchQuery, SearchScope};
use crate::server::search::{
    service::global_search,
    types::{GlobalSearchResponse, SearchHit},
};
use crate::server::shared::entities::{Entity, EntityDiscriminants};
use crate::server::shared::storage::traits::{Storable, Storage};
use crate::server::tags::r#impl::base::{Tag, TagBase};
use crate::server::user_api_keys::r#impl::base::{UserApiKey, UserApiKeyBase};
use crate::server::users::r#impl::permissions::UserOrgPermissions;
use crate::server::vlans::r#impl::base::{Vlan, VlanBase};

use super::{host, organization, service, site, subnet, test_services, user};

fn scope(
    org_id: Uuid,
    site_ids: Vec<Uuid>,
    user_id: Uuid,
    permissions: UserOrgPermissions,
) -> SearchScope {
    SearchScope {
        site_ids,
        organization_id: org_id,
        permissions,
        session_user_id: Some(user_id),
        user_id,
    }
}

fn query(text: &str, tag_ids: Vec<Uuid>) -> SearchQuery {
    SearchQuery::new(text, tag_ids, 5)
}

/// Every returned entity's (type, id).
fn hits(response: &GlobalSearchResponse) -> Vec<(EntityDiscriminants, Uuid)> {
    response
        .groups
        .iter()
        .flat_map(|group| {
            group
                .items
                .iter()
                .map(move |hit| (group.entity_type, Entity::from(hit.clone()).id()))
        })
        .collect()
}

fn ids_of(response: &GlobalSearchResponse, entity_type: EntityDiscriminants) -> Vec<Uuid> {
    hits(response)
        .into_iter()
        .filter(|(t, _)| *t == entity_type)
        .map(|(_, id)| id)
        .collect()
}

#[tokio::test]
async fn returns_matches_of_several_types_within_the_callers_sites_and_org() {
    let (storage, services, _container) = test_services().await;

    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let mut visible = site(&org.id);
    visible.base.name = "Lisbon office".to_string();
    storage.sites.create(&visible).await.unwrap();
    let mut hidden = site(&org.id);
    hidden.base.name = "Lisbon warehouse".to_string();
    storage.sites.create(&hidden).await.unwrap();

    let other_org = organization();
    storage.organizations.create(&other_org).await.unwrap();
    let foreign = site(&other_org.id);
    storage.sites.create(&foreign).await.unwrap();

    // The same name on every site: only the visible site's copies may come back.
    let mut visible_ids = Vec::new();
    for (site_id, org_id) in [
        (visible.id, org.id),
        (hidden.id, org.id),
        (foreign.id, other_org.id),
    ] {
        let mut h = host(&site_id);
        h.base.name = HostName::manual("lisbon-core".to_string());
        storage.hosts.create(&h).await.unwrap();
        let mut sn = subnet(&site_id);
        sn.base.name = "Lisbon LAN".to_string();
        storage.subnets.create(&sn).await.unwrap();
        let vlan = Vlan::new(VlanBase {
            name: "Lisbon voice".to_string(),
            site_id,
            organization_id: org_id,
            ..Default::default()
        });
        storage.vlans.create(&vlan).await.unwrap();
        if site_id == visible.id {
            visible_ids.extend([
                (EntityDiscriminants::Host, h.id),
                (EntityDiscriminants::Subnet, sn.id),
                (EntityDiscriminants::Vlan, vlan.id),
            ]);
        }
    }
    visible_ids.push((EntityDiscriminants::Site, visible.id));

    let owner = user(&org.id);
    let caller = scope(
        org.id,
        vec![visible.id],
        owner.id,
        UserOrgPermissions::Owner,
    );
    let response = global_search(&services, &caller, &query("lisbon", vec![]))
        .await
        .unwrap();

    // Hits come in their list's shape: a host carries the title the UI shows.
    let host_titles: Vec<Option<String>> = response
        .groups
        .iter()
        .flat_map(|group| &group.items)
        .filter_map(|hit| match hit {
            SearchHit::Host(host) => Some(host.display_name.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(host_titles, vec![Some("lisbon-core".to_string())]);

    assert_eq!(
        hits(&response).into_iter().collect::<HashSet<_>>(),
        visible_ids.into_iter().collect::<HashSet<_>>(),
        "one host, subnet, VLAN and site from the caller's site; nothing from the hidden site or the other org"
    );
}

#[tokio::test]
async fn tags_narrow_every_type_to_entities_carrying_all_of_them() {
    let (storage, services, _container) = test_services().await;

    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let net = site(&org.id);
    storage.sites.create(&net).await.unwrap();

    let tag = |name: &str| {
        Tag::new(TagBase {
            name: name.to_string(),
            organization_id: org.id,
            ..Default::default()
        })
    };
    let prod = tag("production");
    let pci = tag("pci");
    storage.tags.create(&prod).await.unwrap();
    storage.tags.create(&pci).await.unwrap();

    let untagged = host(&net.id);
    let prod_host = host(&net.id);
    let both_host = host(&net.id);
    for h in [&untagged, &prod_host, &both_host] {
        storage.hosts.create(h).await.unwrap();
    }
    let s = service(&net.id, &untagged.id);
    storage.services.create(&s).await.unwrap();
    let tag_entity = |entity_id: Uuid, entity_type: EntityDiscriminants, tag_id: Uuid| {
        let storage = &storage;
        async move {
            storage
                .entity_tags
                .add(entity_id, entity_type, tag_id)
                .await
                .unwrap();
        }
    };
    tag_entity(prod_host.id, EntityDiscriminants::Host, prod.id).await;
    tag_entity(both_host.id, EntityDiscriminants::Host, prod.id).await;
    tag_entity(both_host.id, EntityDiscriminants::Host, pci.id).await;
    tag_entity(s.id, EntityDiscriminants::Service, prod.id).await;

    let owner = user(&org.id);
    let caller = scope(org.id, vec![net.id], owner.id, UserOrgPermissions::Owner);

    let one_tag = global_search(&services, &caller, &query("", vec![prod.id]))
        .await
        .unwrap();
    let mut hosts = ids_of(&one_tag, EntityDiscriminants::Host);
    hosts.sort();
    let mut expected = vec![prod_host.id, both_host.id];
    expected.sort();
    assert_eq!(hosts, expected, "a tag alone finds the hosts carrying it");
    assert_eq!(
        ids_of(&one_tag, EntityDiscriminants::Service),
        vec![s.id],
        "and every other type carrying it"
    );
    assert!(
        ids_of(&one_tag, EntityDiscriminants::Site).is_empty(),
        "an untagged entity is never returned for a tag"
    );

    let two_tags = global_search(&services, &caller, &query("", vec![prod.id, pci.id]))
        .await
        .unwrap();
    assert_eq!(
        hits(&two_tags),
        vec![(EntityDiscriminants::Host, both_host.id)],
        "two tags return only entities carrying both"
    );

    let tag_and_text = global_search(&services, &caller, &query("Test Service", vec![prod.id]))
        .await
        .unwrap();
    assert_eq!(
        hits(&tag_and_text),
        vec![(EntityDiscriminants::Service, s.id)],
        "text narrows the tagged entities"
    );
}

#[tokio::test]
async fn users_and_user_api_keys_follow_their_own_list_rules() {
    let (storage, services, _container) = test_services().await;

    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let net = site(&org.id);
    storage.sites.create(&net).await.unwrap();

    let mut admin = user(&org.id);
    admin.base.email = EmailAddress::new_unchecked("ana@lisbon.example");
    admin.base.permissions = UserOrgPermissions::Admin;
    storage.users.create(&admin).await.unwrap();
    let mut viewer = user(&org.id);
    viewer.base.email = EmailAddress::new_unchecked("rui@lisbon.example");
    viewer.base.permissions = UserOrgPermissions::Viewer;
    storage.users.create(&viewer).await.unwrap();

    let key = |owner: Uuid| {
        UserApiKey::new(UserApiKeyBase {
            name: "lisbon automation".to_string(),
            user_id: owner,
            organization_id: org.id,
            key: Uuid::new_v4().to_string(),
            ..Default::default()
        })
    };
    let admin_key = key(admin.id);
    let viewer_key = key(viewer.id);
    storage.user_api_keys.create(&admin_key).await.unwrap();
    storage.user_api_keys.create(&viewer_key).await.unwrap();

    let as_viewer = global_search(
        &services,
        &scope(org.id, vec![net.id], viewer.id, UserOrgPermissions::Viewer),
        &query("lisbon", vec![]),
    )
    .await
    .unwrap();
    assert!(
        ids_of(&as_viewer, EntityDiscriminants::User).is_empty(),
        "a viewer can't list users, so finds none"
    );
    assert_eq!(
        ids_of(&as_viewer, EntityDiscriminants::UserApiKey),
        vec![viewer_key.id],
        "only the caller's own API key"
    );

    let as_admin = global_search(
        &services,
        &scope(org.id, vec![net.id], admin.id, UserOrgPermissions::Admin),
        &query("lisbon", vec![]),
    )
    .await
    .unwrap();
    let mut users = ids_of(&as_admin, EntityDiscriminants::User);
    users.sort();
    let mut expected = vec![admin.id, viewer.id];
    expected.sort();
    assert_eq!(
        users, expected,
        "an admin finds themselves and the users below them"
    );
    assert_eq!(
        ids_of(&as_admin, EntityDiscriminants::UserApiKey),
        vec![admin_key.id]
    );

    let mut through_api_key = scope(org.id, vec![net.id], admin.id, UserOrgPermissions::Admin);
    through_api_key.session_user_id = None;
    let as_api_key = global_search(&services, &through_api_key, &query("lisbon", vec![]))
        .await
        .unwrap();
    assert!(
        ids_of(&as_api_key, EntityDiscriminants::UserApiKey).is_empty(),
        "an API key caller can't list user API keys, so finds none"
    );
}
