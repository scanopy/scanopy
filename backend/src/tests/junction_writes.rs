//! Writes to the junction tables, through the service methods that own them: replace-all sets
//! scoped to one parent, the soft-close diff on SCD2 junctions, dependency member ordering and
//! binding lookup, and the tag writes a tag group makes.

use std::collections::HashSet;
use std::fmt::Display;

use uuid::Uuid;

use super::{
    dependency, host, ip_address, organization, service, site, subnet, test_services, user,
};
use crate::server::{
    auth::{
        access::{Assignment, AssignmentScope, SiteRole},
        middleware::auth::AuthenticatedEntity,
    },
    bindings::r#impl::base::{Binding, BindingBase, BindingType},
    credentials::r#impl::{
        base::{Credential, CredentialBase},
        types::{CredentialAssignment, CredentialHostAssignment},
    },
    daemons::r#impl::base::Daemon,
    dependencies::{
        dependency_members::DependencyMemberRecord,
        r#impl::base::{Dependency, DependencyMembers},
    },
    invites::r#impl::base::Invite,
    shared::{
        attribution::AttributeSource,
        entities::EntityDiscriminants,
        events::types::{DeletionReason, UpdateReason},
        services::{factory::ServiceFactory, traits::CrudService},
        storage::{
            factory::StorageFactory,
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            traits::{Storable, Storage},
        },
        types::api::ApiError,
    },
    site_assignments::r#impl::base::Principal,
    tags::{
        entity_tags::EntityTag,
        groups::base::{TagGroup, TagGroupBase, TagGroupType},
        r#impl::base::{Tag, TagBase},
    },
    user_api_keys::r#impl::base::{UserApiKey, UserApiKeyBase},
    users::r#impl::{base::User, permissions::UserOrgPermissions},
    vlans::r#impl::{
        base::{Vlan, VlanBase, VlanName, VlanNameValue},
        subnet_vlans::SubnetVlanRecord,
    },
};

/// Every row matching `filter`, closed rows included, read without going through the
/// junction's own storage type.
async fn rows<T: Storable + Display>(
    storage: &StorageFactory,
    filter: StorableFilter<T>,
) -> Vec<T> {
    GenericPostgresStorage::<T>::new(storage.pool().clone())
        .get_all(filter)
        .await
        .unwrap()
}

fn sorted(mut ids: Vec<Uuid>) -> Vec<Uuid> {
    ids.sort();
    ids
}

/// An organization with `n` sites.
async fn org_with_sites(storage: &StorageFactory, n: usize) -> (Uuid, Vec<Uuid>) {
    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let mut sites = Vec::new();
    for _ in 0..n {
        let s = site(&org.id);
        storage.sites.create(&s).await.unwrap();
        sites.push(s.id);
    }
    (org.id, sites)
}

async fn member(storage: &StorageFactory, org_id: &Uuid) -> User {
    let mut u = user(org_id);
    u.base.permissions = UserOrgPermissions::Member;
    u.base.email = email_address::EmailAddress::new_unchecked(format!("{}@test.example", u.id));
    storage.users.create(&u).await.unwrap();
    u
}

#[tokio::test]
async fn user_site_access_is_replaced_per_user() {
    let (storage, services, _container) = test_services().await;
    let users = &services.user_service;
    let (org_id, s) = org_with_sites(&storage, 3).await;
    let (a, b) = (
        member(&storage, &org_id).await,
        member(&storage, &org_id).await,
    );

    let set = async |user_id: Uuid, sites: Vec<Uuid>| {
        let role = UserOrgPermissions::Member;
        services
            .access_service
            .set_access(
                Principal::User(user_id),
                role,
                &Assignment::for_person(role, &sites),
                AuthenticatedEntity::System,
            )
            .await
    };
    set(a.id, vec![s[0], s[1]]).await.unwrap();
    set(b.id, vec![s[1]]).await.unwrap();
    set(a.id, vec![s[1], s[2]]).await.unwrap();

    assert_eq!(
        sorted(users.get_site_ids(&a.id).await.unwrap()),
        sorted(vec![s[1], s[2]])
    );
    assert_eq!(users.get_site_ids(&b.id).await.unwrap(), vec![s[1]]);

    let mut both = vec![a.clone(), b.clone()];
    users.hydrate_site_ids_batch(&mut both).await.unwrap();
    assert_eq!(
        sorted(both[0].base.site_ids.clone()),
        sorted(vec![s[1], s[2]])
    );
    assert_eq!(both[1].base.site_ids, vec![s[1]]);

    let with_access: HashSet<Uuid> = users
        .get_users_with_site_access(&s[1], &org_id)
        .await
        .unwrap()
        .into_iter()
        .map(|u| u.id)
        .collect();
    assert_eq!(with_access, HashSet::from([a.id, b.id]));
}

#[tokio::test]
async fn user_api_key_site_access_is_replaced_per_key() {
    let (storage, services, _container) = test_services().await;
    let keys = &services.user_api_key_service;
    let access = &services.access_service;
    let (org_id, s) = org_with_sites(&storage, 3).await;
    let owner = member(&storage, &org_id).await;

    let key = |name: &str| {
        UserApiKey::new(UserApiKeyBase {
            name: name.to_string(),
            user_id: owner.id,
            organization_id: org_id,
            key: Uuid::new_v4().to_string(),
            is_enabled: true,
            ..Default::default()
        })
    };
    let first = access
        .create_api_key(
            key("ci"),
            Some(vec![s[0], s[1]]),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();
    let second = access
        .create_api_key(key("backup"), Some(vec![s[2]]), AuthenticatedEntity::System)
        .await
        .unwrap();
    let role = first.base.permissions;
    access
        .set_access(
            Principal::ApiKey(first.id),
            role,
            &Assignment::for_api_key(role, Some(&[s[1], s[2]])),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    assert_eq!(
        sorted(keys.get_site_ids(&first.id).await.unwrap().unwrap()),
        sorted(vec![s[1], s[2]])
    );
    let mut both = vec![first.clone(), second.clone()];
    keys.hydrate_site_ids(&mut both).await.unwrap();
    assert_eq!(
        sorted(both[0].base.site_ids.clone().unwrap()),
        sorted(vec![s[1], s[2]])
    );
    assert_eq!(both[1].base.site_ids, Some(vec![s[2]]));
}

fn scoped(role: SiteRole, scope: AssignmentScope) -> Assignment {
    Assignment { role, scope }
}

/// The rows of a principal, whatever their order.
async fn held(services: &ServiceFactory, principal: Principal) -> Vec<String> {
    let rows = services.site_assignment_service.get_for(principal).await;
    let mut held: Vec<String> = rows.unwrap().iter().map(|a| format!("{a:?}")).collect();
    held.sort();
    held
}

/// Mixed roles and All sites come back as written, for each thing that can hold them, and a second
/// write replaces the first.
#[tokio::test]
async fn set_access_round_trips_for_users_keys_and_invites() {
    let (storage, services, _container) = test_services().await;
    let access = &services.access_service;
    let (org_id, s) = org_with_sites(&storage, 2).await;
    let person = member(&storage, &org_id).await;
    let key = access
        .create_api_key(
            UserApiKey::new(UserApiKeyBase {
                name: "mixed".to_string(),
                user_id: person.id,
                organization_id: org_id,
                key: Uuid::new_v4().to_string(),
                is_enabled: true,
                ..Default::default()
            }),
            Some(vec![]),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();
    let invite = access
        .create_invite(
            Invite::with_expiration(
                org_id,
                "http://localhost".to_string(),
                person.id,
                24,
                UserOrgPermissions::Viewer,
                vec![],
                None,
            ),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    let role = UserOrgPermissions::Member;
    let mixed = [
        scoped(SiteRole::Viewer, AssignmentScope::Site(s[0])),
        scoped(SiteRole::Admin, AssignmentScope::Site(s[1])),
    ];
    let everywhere = [scoped(SiteRole::Member, AssignmentScope::AllSites)];
    let sorted_debug = |assignments: &[Assignment]| {
        let mut rows: Vec<String> = assignments.iter().map(|a| format!("{a:?}")).collect();
        rows.sort();
        rows
    };
    for principal in [
        Principal::User(person.id),
        Principal::ApiKey(key.id),
        Principal::Invite(invite.id),
    ] {
        for assignments in [&mixed[..], &everywhere[..]] {
            access
                .set_access(principal, role, assignments, AuthenticatedEntity::System)
                .await
                .unwrap();
            assert_eq!(held(&services, principal).await, sorted_debug(assignments));
        }
    }

    let stored = services
        .user_api_key_service
        .get_by_id(&key.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.base.permissions, role);
    let stored = services.invite_service.get_by_id(&invite.id).await;
    assert_eq!(stored.unwrap().unwrap().base.permissions, role);
}

#[tokio::test]
async fn an_owner_always_holds_every_site_and_the_last_owner_stays() {
    let (storage, services, _container) = test_services().await;
    let access = &services.access_service;
    let (org_id, s) = org_with_sites(&storage, 1).await;
    let every_site = vec![format!(
        "{:?}",
        scoped(SiteRole::Admin, AssignmentScope::AllSites)
    )];
    let one_site = [scoped(SiteRole::Member, AssignmentScope::Site(s[0]))];
    let system = || AuthenticatedEntity::System;

    let mut first = user(&org_id);
    first.base.permissions = UserOrgPermissions::Owner;
    first.base.email = "first@test.example".parse().unwrap();
    storage.users.create(&first).await.unwrap();
    let first = Principal::User(first.id);

    let owner = UserOrgPermissions::Owner;
    access
        .set_access(first, owner, &one_site, system())
        .await
        .unwrap();
    assert_eq!(held(&services, first).await, every_site);

    let member_role = UserOrgPermissions::Member;
    let refused = access
        .set_access(first, member_role, &one_site, system())
        .await
        .unwrap_err();
    let refused = ApiError::from(refused);
    assert_eq!(refused.status, axum::http::StatusCode::CONFLICT);
    assert_eq!(
        held(&services, first).await,
        every_site,
        "a refused demotion changes nothing"
    );

    let second = member(&storage, &org_id).await;
    let second = Principal::User(second.id);
    access
        .set_access(second, owner, &[], system())
        .await
        .unwrap();
    access
        .set_access(first, member_role, &one_site, system())
        .await
        .unwrap();
    assert_eq!(held(&services, second).await, every_site);
}

async fn credential(storage: &StorageFactory, org_id: Uuid) -> Uuid {
    let c = Credential::new(CredentialBase {
        organization_id: org_id,
        ..Default::default()
    });
    storage.credentials.create(&c).await.unwrap();
    c.id
}

#[tokio::test]
async fn site_credentials_are_replaced_from_either_end() {
    let (storage, services, _container) = test_services().await;
    let creds = &services.credential_service;
    let (org_id, s) = org_with_sites(&storage, 3).await;
    let (c1, c2) = (
        credential(&storage, org_id).await,
        credential(&storage, org_id).await,
    );

    creds.set_site_credentials(&s[0], &[c1, c2]).await.unwrap();
    creds.set_site_credentials(&s[1], &[c1]).await.unwrap();
    // Replacing c1's sites leaves c2's row on s[0] alone.
    creds
        .set_credential_sites(&c1, &[s[1], s[2]])
        .await
        .unwrap();

    assert_eq!(
        creds.get_credential_ids_for_site(&s[0]).await.unwrap(),
        vec![c2]
    );
    assert_eq!(
        sorted(creds.get_site_ids_for_credential(&c1).await.unwrap()),
        sorted(vec![s[1], s[2]])
    );
    let by_site = creds.get_credential_ids_for_sites(&s).await.unwrap();
    assert_eq!(by_site.get(&s[2]), Some(&vec![c1]));
    let by_credential = creds.get_site_ids_for_credentials(&[c1, c2]).await.unwrap();
    assert_eq!(by_credential.get(&c2), Some(&vec![s[0]]));

    // Replacing a site's set leaves the credential's other sites alone.
    creds.set_site_credentials(&s[1], &[c2]).await.unwrap();
    assert_eq!(
        creds.get_site_ids_for_credential(&c1).await.unwrap(),
        vec![s[2]]
    );
    assert_eq!(
        sorted(creds.get_site_ids_for_credential(&c2).await.unwrap()),
        sorted(vec![s[0], s[1]])
    );
}

#[tokio::test]
async fn host_credentials_are_replaced_from_either_end() {
    let (storage, services, _container) = test_services().await;
    let creds = &services.credential_service;
    let (org_id, s) = org_with_sites(&storage, 1).await;
    let (h1, h2) = (host(&s[0]), host(&s[0]));
    storage.hosts.create(&h1).await.unwrap();
    storage.hosts.create(&h2).await.unwrap();
    let sn = subnet(&s[0]);
    storage.subnets.create(&sn).await.unwrap();
    let mut ip = ip_address(&s[0], &sn.id);
    ip.base.host_id = h1.id;
    storage.ip_addresses.create(&ip).await.unwrap();
    let (c1, c2) = (
        credential(&storage, org_id).await,
        credential(&storage, org_id).await,
    );

    let limited = CredentialAssignment {
        credential_id: c1,
        ip_address_ids: Some(vec![ip.id]),
    };
    let whole = |credential_id| CredentialAssignment {
        credential_id,
        ip_address_ids: None,
    };
    creds
        .set_host_credentials(&h1.id, &[limited.clone(), whole(c2)])
        .await
        .unwrap();
    creds
        .set_host_credentials(&h2.id, &[whole(c1)])
        .await
        .unwrap();

    let on_h1 = creds
        .get_credential_assignments_for_host(&h1.id)
        .await
        .unwrap();
    let limited_back = on_h1.iter().find(|a| a.credential_id == c1).unwrap();
    assert_eq!(limited_back.ip_address_ids, Some(vec![ip.id]));
    assert_eq!(on_h1.len(), 2);

    // Replacing c1's hosts leaves c2's assignment on h1 alone.
    creds
        .set_credential_host_assignments(
            &c1,
            &[CredentialHostAssignment {
                host_id: h2.id,
                ip_address_ids: None,
            }],
        )
        .await
        .unwrap();
    let by_host = creds
        .get_credential_assignments_for_hosts(&[h1.id, h2.id])
        .await
        .unwrap();
    let ids = |host_id: &Uuid| -> Vec<Uuid> {
        sorted(
            by_host
                .get(host_id)
                .map(|a| a.iter().map(|x| x.credential_id).collect())
                .unwrap_or_default(),
        )
    };
    assert_eq!(ids(&h1.id), vec![c2]);
    assert_eq!(ids(&h2.id), vec![c1]);
    let for_c1 = creds
        .get_host_assignments_for_credential(&c1)
        .await
        .unwrap();
    assert_eq!(
        for_c1.iter().map(|a| a.host_id).collect::<Vec<_>>(),
        vec![h2.id]
    );
    let for_both = creds
        .get_host_assignments_for_credentials(&[c1, c2])
        .await
        .unwrap();
    assert_eq!(for_both.get(&c2).map(|a| a[0].host_id), Some(h1.id));
}

#[tokio::test]
async fn daemon_interfaced_subnets_are_replaced_per_daemon() {
    let (storage, services, _container) = test_services().await;
    let daemons = &services.daemon_service;
    let (org_id, s) = org_with_sites(&storage, 1).await;
    let owner = member(&storage, &org_id).await;
    let h = host(&s[0]);
    storage.hosts.create(&h).await.unwrap();
    let daemon = || -> Daemon {
        let mut d = super::daemon(&s[0], &h.id);
        d.base.user_id = owner.id;
        d
    };
    let (d1, d2) = (daemon(), daemon());
    storage.daemons.create(&d1).await.unwrap();
    storage.daemons.create(&d2).await.unwrap();
    let subnets: Vec<Uuid> = {
        let mut ids = Vec::new();
        for _ in 0..3 {
            let sn = subnet(&s[0]);
            storage.subnets.create(&sn).await.unwrap();
            ids.push(sn.id);
        }
        ids
    };

    daemons
        .set_interfaced_subnet_ids(&d1.id, &subnets[..2])
        .await
        .unwrap();
    daemons
        .set_interfaced_subnet_ids(&d2.id, &subnets[2..])
        .await
        .unwrap();
    daemons
        .set_interfaced_subnet_ids(&d1.id, &subnets[1..])
        .await
        .unwrap();

    assert_eq!(
        daemons.get_interfaced_subnet_ids(&d1.id).await,
        sorted(subnets[1..].to_vec())
    );
    let batch = daemons
        .get_interfaced_subnet_ids_batch(&[d1.id, d2.id])
        .await;
    assert_eq!(batch.get(&d2.id), Some(&vec![subnets[2]]));
}

/// A status report that carries no interfaced subnets leaves the junction as the last session
/// classified it. Only a report that carries a list rewrites it, an empty one included.
#[tokio::test]
async fn a_status_without_interfaced_subnets_leaves_the_junction_alone() {
    let (storage, services, _container) = test_services().await;
    let daemons = &services.daemon_service;
    let (org_id, s) = org_with_sites(&storage, 1).await;
    let owner = member(&storage, &org_id).await;
    let h = host(&s[0]);
    storage.hosts.create(&h).await.unwrap();
    let mut d = super::daemon(&s[0], &h.id);
    d.base.user_id = owner.id;
    storage.daemons.create(&d).await.unwrap();
    let sn = subnet(&s[0]);
    storage.subnets.create(&sn).await.unwrap();
    daemons
        .set_interfaced_subnet_ids(&d.id, &[sn.id])
        .await
        .unwrap();
    let status = |interfaced_subnets| crate::daemon::runtime::state::DaemonStatus {
        url: None,
        name: "daemon".to_string(),
        mode: d.base.mode,
        version: Some(semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap()),
        capabilities: Default::default(),
        interfaced_subnets,
        interfaces_fingerprint: None,
        ready_for_work: true,
        os: None,
    };

    daemons
        .process_status(d.id, status(None), AuthenticatedEntity::System)
        .await
        .unwrap();
    assert_eq!(daemons.get_interfaced_subnet_ids(&d.id).await, vec![sn.id]);

    daemons
        .process_status(d.id, status(Some(Vec::new())), AuthenticatedEntity::System)
        .await
        .unwrap();
    assert!(daemons.get_interfaced_subnet_ids(&d.id).await.is_empty());
}

/// A site with one host holding two services, each bound to its own IP address.
struct Bound {
    site_id: Uuid,
    services: [Uuid; 2],
    bindings: [Uuid; 2],
}

async fn bound(storage: &StorageFactory) -> Bound {
    let (_, s) = org_with_sites(storage, 1).await;
    let h = host(&s[0]);
    storage.hosts.create(&h).await.unwrap();
    let sn = subnet(&s[0]);
    storage.subnets.create(&sn).await.unwrap();
    let mut services = [Uuid::nil(); 2];
    let mut bindings = [Uuid::nil(); 2];
    for i in 0..2 {
        let mut ip = ip_address(&s[0], &sn.id);
        ip.base.host_id = h.id;
        ip.base.position = i as i32;
        ip.base.ip_address =
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, 10 + i as u8));
        storage.ip_addresses.create(&ip).await.unwrap();
        let svc = service(&s[0], &h.id);
        storage.services.create(&svc).await.unwrap();
        let b = Binding::new(BindingBase::new(
            svc.id,
            s[0],
            BindingType::IPAddress {
                ip_address_id: ip.id,
            },
        ));
        storage.bindings.create(&b).await.unwrap();
        services[i] = svc.id;
        bindings[i] = b.id;
    }
    Bound {
        site_id: s[0],
        services,
        bindings,
    }
}

async fn members_of(storage: &StorageFactory, dependency_id: Uuid) -> Vec<DependencyMemberRecord> {
    let mut members = rows(
        storage,
        StorableFilter::<DependencyMemberRecord>::new_from_dependency_ids(&[dependency_id]),
    )
    .await;
    members.sort_by_key(|m| m.base.position);
    members
}

#[tokio::test]
async fn dependency_members_keep_order_and_resolve_bindings() {
    let (storage, services, _container) = test_services().await;
    let deps = &services.dependency_service;
    let lab = bound(&storage).await;

    // Services: duplicates dropped, order kept as position.
    let mut by_service = dependency(&lab.site_id);
    by_service.base.members = DependencyMembers::Services {
        service_ids: vec![lab.services[1], lab.services[0], lab.services[1]],
    };
    let by_service = deps
        .create(by_service, AuthenticatedEntity::System)
        .await
        .unwrap();
    let stored = members_of(&storage, by_service.id).await;
    assert_eq!(
        stored
            .iter()
            .map(|m| (m.base.service_id, m.base.binding_id, m.base.position))
            .collect::<Vec<_>>(),
        vec![(lab.services[1], None, 0), (lab.services[0], None, 1)]
    );

    // Bindings: each row records its binding's service.
    let mut by_binding = dependency(&lab.site_id);
    by_binding.base.members = DependencyMembers::Bindings {
        binding_ids: vec![lab.bindings[1], lab.bindings[0]],
    };
    let mut by_binding = deps
        .create(by_binding, AuthenticatedEntity::System)
        .await
        .unwrap();
    let stored = members_of(&storage, by_binding.id).await;
    assert_eq!(
        stored
            .iter()
            .map(|m| (m.base.service_id, m.base.binding_id))
            .collect::<Vec<_>>(),
        vec![
            (lab.services[1], Some(lab.bindings[1])),
            (lab.services[0], Some(lab.bindings[0])),
        ]
    );

    // A binding gone from storage keeps the service its row recorded.
    storage.bindings.delete(&lab.bindings[0]).await.unwrap();
    by_binding.base.members = DependencyMembers::Bindings {
        binding_ids: vec![lab.bindings[0]],
    };
    deps.update(
        &mut by_binding,
        AuthenticatedEntity::System,
        UpdateReason::EntityRequested,
    )
    .await
    .unwrap();
    let stored = members_of(&storage, by_binding.id).await;
    assert_eq!(
        stored
            .iter()
            .map(|m| (m.base.service_id, m.base.binding_id))
            .collect::<Vec<_>>(),
        vec![(lab.services[0], Some(lab.bindings[0]))]
    );

    // A binding never seen is an error.
    by_binding.base.members = DependencyMembers::Bindings {
        binding_ids: vec![Uuid::new_v4()],
    };
    assert!(
        deps.update(
            &mut by_binding,
            AuthenticatedEntity::System,
            UpdateReason::EntityRequested
        )
        .await
        .is_err()
    );

    // Members read back in position order.
    let read: Dependency = deps.get_by_id(&by_service.id).await.unwrap().unwrap();
    assert_eq!(
        read.base.members,
        DependencyMembers::Services {
            service_ids: vec![lab.services[1], lab.services[0]],
        }
    );

    deps.delete(
        &by_service.id,
        AuthenticatedEntity::System,
        DeletionReason::EntityRequested,
    )
    .await
    .unwrap();
    assert!(members_of(&storage, by_service.id).await.is_empty());
}

#[tokio::test]
async fn entity_tags_diff_by_soft_close() {
    let (storage, services, _container) = test_services().await;
    let entity_tags = &services.entity_tag_service;
    let (org_id, s) = org_with_sites(&storage, 1).await;
    let sn = subnet(&s[0]);
    storage.subnets.create(&sn).await.unwrap();
    let mut tags = Vec::new();
    for name in ["a", "b", "c"] {
        let t = Tag::new(TagBase {
            name: name.to_string(),
            organization_id: org_id,
            ..Default::default()
        });
        storage.tags.create(&t).await.unwrap();
        tags.push(t.id);
    }
    let subnet_rows = || {
        StorableFilter::<EntityTag>::new_from_uuid_column("entity_id", &sn.id)
            .entity_type(&EntityDiscriminants::Subnet)
    };

    entity_tags
        .set_tags(
            sn.id,
            EntityDiscriminants::Subnet,
            vec![tags[0], tags[1]],
            org_id,
        )
        .await
        .unwrap();
    let kept_before = rows(&storage, subnet_rows().live())
        .await
        .into_iter()
        .find(|r| r.base.tag_id == tags[1])
        .unwrap();

    entity_tags
        .set_tags(
            sn.id,
            EntityDiscriminants::Subnet,
            vec![tags[1], tags[2]],
            org_id,
        )
        .await
        .unwrap();
    let all = rows(&storage, subnet_rows()).await;
    let live: Vec<&EntityTag> = all.iter().filter(|r| r.valid_to.is_none()).collect();
    let closed: Vec<&EntityTag> = all.iter().filter(|r| r.valid_to.is_some()).collect();
    assert_eq!(
        sorted(live.iter().map(|r| r.base.tag_id).collect()),
        sorted(vec![tags[1], tags[2]])
    );
    assert_eq!(
        closed.iter().map(|r| r.base.tag_id).collect::<Vec<_>>(),
        vec![tags[0]]
    );
    assert!(
        live.iter().any(|r| r.id == kept_before.id),
        "an unchanged tag keeps its row"
    );

    // A row for another entity type with the same id is not this subnet's.
    entity_tags
        .set_tags(sn.id, EntityDiscriminants::Host, vec![tags[0]], org_id)
        .await
        .unwrap();
    entity_tags
        .remove_all_for_entity(sn.id, EntityDiscriminants::Subnet)
        .await
        .unwrap();
    assert!(rows(&storage, subnet_rows().live()).await.is_empty());
    assert_eq!(
        rows(&storage, subnet_rows()).await.len(),
        3,
        "removed tags are closed, not deleted"
    );
    let host_rows = rows(
        &storage,
        StorableFilter::<EntityTag>::new_from_uuid_column("entity_id", &sn.id)
            .entity_type(&EntityDiscriminants::Host)
            .live(),
    )
    .await;
    assert_eq!(host_rows.len(), 1);
}

#[tokio::test]
async fn subnet_vlan_links_diff_by_soft_close() {
    let (storage, services, _container) = test_services().await;
    let (org_id, s) = org_with_sites(&storage, 1).await;
    let sn = subnet(&s[0]);
    storage.subnets.create(&sn).await.unwrap();
    let mut vlans = Vec::new();
    for number in [10, 20, 30] {
        let v = Vlan::new(VlanBase {
            vlan_number: number,
            name: VlanName::new(
                VlanNameValue(format!("vlan {number}")),
                AttributeSource::Manual,
            ),
            site_id: s[0],
            organization_id: org_id,
            ..Default::default()
        });
        storage.vlans.create(&v).await.unwrap();
        vlans.push(v.id);
    }
    let links = || StorableFilter::<SubnetVlanRecord>::new_from_uuid_column("subnet_id", &sn.id);

    save_subnet_vlans(&services, &sn.id, &[vlans[0], vlans[1]]).await;
    let kept_before = rows(&storage, links().live())
        .await
        .into_iter()
        .find(|r| r.vlan_id() == vlans[1])
        .unwrap();
    save_subnet_vlans(&services, &sn.id, &[vlans[1], vlans[2]]).await;

    let all = rows(&storage, links()).await;
    let live: Vec<&SubnetVlanRecord> = all.iter().filter(|r| r.valid_to.is_none()).collect();
    let closed: Vec<&SubnetVlanRecord> = all.iter().filter(|r| r.valid_to.is_some()).collect();
    assert_eq!(
        sorted(live.iter().map(|r| r.vlan_id()).collect()),
        sorted(vec![vlans[1], vlans[2]])
    );
    assert_eq!(
        closed.iter().map(|r| r.vlan_id()).collect::<Vec<_>>(),
        vec![vlans[0]]
    );
    assert!(
        live.iter().any(|r| r.id == kept_before.id),
        "an unchanged link keeps its row"
    );

    // Read back by VLAN, live links only.
    let by_vlan = read_vlan_subnets(&services, &vlans).await;
    assert_eq!(by_vlan.get(&vlans[0]), None);
    assert_eq!(by_vlan.get(&vlans[2]), Some(&vec![sn.id]));
}

/// The subnet-VLAN write `HostService` makes at the end of a scan.
async fn save_subnet_vlans(services: &ServiceFactory, subnet_id: &Uuid, vlan_ids: &[Uuid]) {
    services
        .vlan_service
        .set_subnet_vlans(subnet_id, vlan_ids)
        .await
        .unwrap();
}

/// The read `VlanService` hydrates each VLAN's subnets from.
async fn read_vlan_subnets(
    services: &ServiceFactory,
    vlan_ids: &[Uuid],
) -> std::collections::HashMap<Uuid, Vec<Uuid>> {
    services
        .vlan_service
        .subnet_ids_for_vlans(vlan_ids)
        .await
        .unwrap()
}

/// A group's tags as (name, position), by name.
async fn in_group(storage: &StorageFactory, group_id: Uuid) -> Vec<(String, Option<i32>)> {
    let mut tags = rows(
        storage,
        StorableFilter::<Tag>::new_from_uuid_column("tag_group_id", &group_id),
    )
    .await;
    tags.sort_by_key(|t| t.base.name.clone());
    tags.into_iter()
        .map(|t| (t.base.name, t.base.position))
        .collect()
}

#[tokio::test]
async fn tag_groups_write_their_tags() {
    let (storage, services, _container) = test_services().await;
    let groups = &services.tag_group_service;
    let (org_id, _) = org_with_sites(&storage, 0).await;

    let all = groups
        .seed_groups(org_id, &[], AuthenticatedEntity::System)
        .await
        .unwrap();
    let visibility = all
        .iter()
        .find(|g| g.base.group_type == TagGroupType::Visibility)
        .expect("seed_groups creates the Visibility group");
    let hidden = rows(
        &storage,
        StorableFilter::<Tag>::new_from_uuid_column("tag_group_id", &visibility.id),
    )
    .await;
    assert_eq!(
        hidden.len(),
        1,
        "a new Visibility group starts with one tag"
    );

    let mut base = TagGroupBase::new(org_id, "Racks".to_string(), TagGroupType::Custom);
    base.ordered = true;
    let mut group = groups
        .create(TagGroup::new(base), AuthenticatedEntity::System)
        .await
        .unwrap();
    let mut tag_ids = Vec::new();
    for (name, position) in [("charlie", 1), ("alpha", 2), ("bravo", 3)] {
        let t = Tag::new(TagBase {
            name: name.to_string(),
            organization_id: org_id,
            tag_group_id: Some(group.id),
            position: Some(position),
            ..Default::default()
        });
        storage.tags.create(&t).await.unwrap();
        tag_ids.push(t.id);
    }
    groups
        .reorder_tags(&group, &[tag_ids[2], tag_ids[0], tag_ids[1]])
        .await
        .unwrap();
    assert_eq!(
        in_group(&storage, group.id).await,
        vec![
            ("alpha".to_string(), Some(3)),
            ("bravo".to_string(), Some(1)),
            ("charlie".to_string(), Some(2)),
        ]
    );
    assert!(groups.reorder_tags(&group, &tag_ids[..2]).await.is_err());

    group.base.ordered = false;
    let mut group = groups
        .update(
            &mut group,
            AuthenticatedEntity::System,
            UpdateReason::EntityRequested,
        )
        .await
        .unwrap();
    assert!(
        in_group(&storage, group.id)
            .await
            .iter()
            .all(|(_, p)| p.is_none())
    );

    group.base.ordered = true;
    let group = groups
        .update(
            &mut group,
            AuthenticatedEntity::System,
            UpdateReason::EntityRequested,
        )
        .await
        .unwrap();
    assert_eq!(
        in_group(&storage, group.id).await,
        vec![
            ("alpha".to_string(), Some(1)),
            ("bravo".to_string(), Some(2)),
            ("charlie".to_string(), Some(3)),
        ]
    );

    groups
        .delete(
            &group.id,
            AuthenticatedEntity::System,
            DeletionReason::EntityRequested,
        )
        .await
        .unwrap();
    let ungrouped = rows(
        &storage,
        StorableFilter::<Tag>::new_from_entity_ids(&tag_ids),
    )
    .await;
    assert!(
        ungrouped
            .iter()
            .all(|t| t.base.tag_group_id.is_none() && t.base.position.is_none())
    );
}
