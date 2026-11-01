//! A principal's role and site rows commit together or not at all, and changes to who holds Owner
//! in an organization run one at a time.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use axum::body::Body;
use axum::extract::{FromRequestParts, Path, State};
use axum::http::{Request, StatusCode};
use uuid::Uuid;

use super::{organization, site, test_state, user};
use crate::server::auth::access::{Assignment, AssignmentScope, SiteRole};
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::auth::middleware::permissions::{
    Authorized, Can, Delete, PermissionRequirement, RequireVerified,
};
use crate::server::config::AppState;
use crate::server::shared::events::traits::{EntityEventFilter, Event, Subscriber};
use crate::server::shared::events::types::EntityOperation;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::factory::StorageFactory;
use crate::server::shared::storage::traits::Storage;
use crate::server::shared::types::api::ApiJson;
use crate::server::site_assignments::r#impl::base::Principal;
use crate::server::users::handlers::{bulk_delete_users, delete_user};
use crate::server::users::r#impl::{base::User, permissions::UserOrgPermissions};

type DeleteUsers = RequireVerified<Can<User, Delete>>;

const OWNER: UserOrgPermissions = UserOrgPermissions::Owner;
const MEMBER: UserOrgPermissions = UserOrgPermissions::Member;
const VIEWER: UserOrgPermissions = UserOrgPermissions::Viewer;

/// Long enough for an operation the lock doesn't hold back to finish.
const WAIT: Duration = Duration::from_millis(750);
const RACE_ROUNDS: usize = 25;

/// The id of every Updated event published.
#[derive(Default)]
struct Updates(Mutex<Vec<Uuid>>);

#[async_trait]
impl Subscriber<EntityOperation> for Updates {
    fn filter(&self) -> EntityEventFilter {
        EntityEventFilter::all()
    }

    async fn handle(&self, events: Vec<Event<EntityOperation>>) -> anyhow::Result<()> {
        let mut updates = self.0.lock().unwrap();
        for event in events {
            if matches!(event.operation, EntityOperation::Updated { .. }) {
                updates.push(event.scope.entity_id());
            }
        }
        Ok(())
    }
}

async fn listen(state: &AppState) -> Arc<Updates> {
    let updates = Arc::new(Updates::default());
    state
        .services
        .event_bus
        .register::<EntityOperation>(updates.clone(), "test_access_updates")
        .await;
    updates
}

/// How many Updated events `id` has had, once every subscriber has run.
async fn updates_of(state: &AppState, updates: &Updates, id: Uuid) -> usize {
    state.services.event_bus.flush().await;
    let updates = updates.0.lock().unwrap();
    updates.iter().filter(|updated| **updated == id).count()
}

async fn org_with_site(storage: &StorageFactory) -> (Uuid, Uuid) {
    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let s = site(&org.id);
    storage.sites.create(&s).await.unwrap();
    (org.id, s.id)
}

async fn person(state: &AppState, org_id: Uuid, role: UserOrgPermissions, sites: &[Uuid]) -> User {
    let mut u = user(&org_id);
    u.base.email = email_address::EmailAddress::new_unchecked(format!("{}@test.example", u.id));
    u.base.email_verified = true;
    u.base.permissions = role;
    let u = state
        .services
        .user_service
        .create(u, AuthenticatedEntity::System)
        .await
        .unwrap();
    state
        .services
        .access_service
        .set_access(
            Principal::User(u.id),
            role,
            &Assignment::for_person(role, sites),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();
    u
}

async fn stored(state: &AppState, id: Uuid) -> User {
    let user = state.services.user_service.get_by_id(&id).await.unwrap();
    user.expect("the user exists")
}

async fn rows(state: &AppState, principal: Principal) -> Vec<String> {
    let held = state.services.site_assignment_service.get_for(principal);
    let held = held.await.unwrap();
    let mut held: Vec<String> = held.iter().map(|a| format!("{a:?}")).collect();
    held.sort();
    held
}

fn on_missing_site() -> [Assignment; 1] {
    [Assignment {
        role: SiteRole::Member,
        scope: AssignmentScope::Site(Uuid::new_v4()),
    }]
}

/// The extractor's result for a request `caller` makes.
async fn authorized<P: PermissionRequirement>(
    state: &Arc<AppState>,
    caller: &User,
) -> Authorized<P> {
    let access = state.services.access_service.user_access(caller).await;
    let entity = AuthenticatedEntity::User {
        user_id: caller.id,
        organization_id: caller.base.organization_id,
        permissions: caller.base.permissions,
        access: access.unwrap(),
        email: caller.base.email.clone(),
        email_verified: true,
        terms_accepted_at: None,
    };
    let mut request = Request::new(Body::empty());
    request.extensions_mut().insert(entity);
    let (mut parts, _) = request.into_parts();
    Authorized::<P>::from_request_parts(&mut parts, state)
        .await
        .ok()
        .expect("the caller passes the permission check")
}

async fn owners(state: &AppState, org_id: Uuid) -> usize {
    let owners = state.services.user_service.get_organization_owners(&org_id);
    owners.await.unwrap().len()
}

#[tokio::test]
async fn a_failed_set_access_leaves_role_rows_and_events_unchanged() {
    let (storage, state, _container) = test_state().await;
    let (org_id, site_id) = org_with_site(&storage).await;
    let target = person(&state, org_id, MEMBER, &[site_id]).await;
    let principal = Principal::User(target.id);
    let rows_before = rows(&state, principal).await;
    let updates = listen(&state).await;
    let access = &state.services.access_service;

    let failed = access
        .set_access(
            principal,
            VIEWER,
            &on_missing_site(),
            AuthenticatedEntity::System,
        )
        .await;

    assert!(failed.is_err(), "the row names a site that doesn't exist");
    assert_eq!(stored(&state, target.id).await.base.permissions, MEMBER);
    assert_eq!(rows(&state, principal).await, rows_before);
    assert_eq!(updates_of(&state, &updates, target.id).await, 0);

    access
        .set_access(
            principal,
            VIEWER,
            &Assignment::for_person(VIEWER, &[site_id]),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();
    assert_eq!(
        updates_of(&state, &updates, target.id).await,
        1,
        "the role change that commits is announced once"
    );
}

#[tokio::test]
async fn a_failed_join_leaves_the_user_where_they_were() {
    let (storage, state, _container) = test_state().await;
    let (org_id, site_id) = org_with_site(&storage).await;
    let (new_org_id, new_site_id) = org_with_site(&storage).await;
    let target = person(&state, org_id, MEMBER, &[site_id]).await;
    let principal = Principal::User(target.id);
    let rows_before = rows(&state, principal).await;
    let updates = listen(&state).await;

    let failed = state
        .services
        .access_service
        .join_organization(
            target.id,
            new_org_id,
            VIEWER,
            &on_missing_site(),
            AuthenticatedEntity::System,
        )
        .await;

    assert!(failed.is_err(), "the row names a site that doesn't exist");
    let after = stored(&state, target.id).await;
    assert_eq!(after.base.organization_id, org_id);
    assert_eq!(after.base.permissions, MEMBER);
    assert_eq!(rows(&state, principal).await, rows_before);
    assert_eq!(updates_of(&state, &updates, target.id).await, 0);

    let new_rows = Assignment::for_person(VIEWER, &[new_site_id]);
    state
        .services
        .access_service
        .join_organization(
            target.id,
            new_org_id,
            VIEWER,
            &new_rows,
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();
    let after = stored(&state, target.id).await;
    assert_eq!(after.base.organization_id, new_org_id);
    assert_eq!(after.base.permissions, VIEWER);
    assert_ne!(rows(&state, principal).await, rows_before);
    assert_eq!(updates_of(&state, &updates, target.id).await, 1);
}

#[tokio::test]
async fn owner_changes_wait_for_the_owners_lock() {
    let (storage, state, _container) = test_state().await;
    let state = Arc::new(state);
    let (org_id, _) = org_with_site(&storage).await;
    let caller = person(&state, org_id, OWNER, &[]).await;
    let (deleted, bulk_deleted, demoted) = (
        person(&state, org_id, OWNER, &[]).await,
        person(&state, org_id, OWNER, &[]).await,
        person(&state, org_id, OWNER, &[]).await,
    );
    let users = &state.services.user_service;

    let guard = users.lock_organization_owners(org_id).await.unwrap();
    let delete = tokio::spawn(delete_user(
        State(state.clone()),
        authorized::<DeleteUsers>(&state, &caller).await,
        Path(deleted.id),
    ));
    let bulk_delete = tokio::spawn(bulk_delete_users(
        State(state.clone()),
        authorized::<DeleteUsers>(&state, &caller).await,
        ApiJson(vec![bulk_deleted.id]),
    ));
    let demote = {
        let state = state.clone();
        let id = demoted.id;
        tokio::spawn(async move {
            state
                .services
                .access_service
                .set_access(
                    Principal::User(id),
                    MEMBER,
                    &[],
                    AuthenticatedEntity::System,
                )
                .await
        })
    };

    tokio::time::sleep(WAIT).await;
    assert!(!delete.is_finished(), "a delete waits for the lock");
    assert!(
        !bulk_delete.is_finished(),
        "a bulk delete waits for the lock"
    );
    assert!(!demote.is_finished(), "a demotion waits for the lock");
    assert_eq!(owners(&state, org_id).await, 4, "nothing has changed yet");

    guard.release().await.unwrap();
    assert!(delete.await.unwrap().is_ok());
    assert!(bulk_delete.await.unwrap().is_ok());
    demote.await.unwrap().unwrap();
    assert_eq!(owners(&state, org_id).await, 1);
    assert_eq!(stored(&state, caller.id).await.base.permissions, OWNER);
}

/// Each round runs in its own organization, whose two Owners go at once.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn one_of_two_owners_always_remains() {
    let (storage, state, _container) = test_state().await;
    let state = Arc::new(state);

    for round in 0..RACE_ROUNDS {
        let (org_id, _) = org_with_site(&storage).await;
        let a = person(&state, org_id, OWNER, &[]).await;
        let b = person(&state, org_id, OWNER, &[]).await;
        let (auth_a, auth_b) = (
            authorized::<DeleteUsers>(&state, &a).await,
            authorized::<DeleteUsers>(&state, &b).await,
        );
        let (first, second) = tokio::join!(
            tokio::spawn(delete_user(State(state.clone()), auth_a, Path(b.id))),
            tokio::spawn(delete_user(State(state.clone()), auth_b, Path(a.id))),
        );
        let refused = [first.unwrap(), second.unwrap()]
            .into_iter()
            .filter_map(Result::err)
            .collect::<Vec<_>>();
        assert_eq!(owners(&state, org_id).await, 1, "delete, round {round}");
        assert_eq!(refused.len(), 1, "delete, round {round}");
        assert_eq!(refused[0].status, StatusCode::CONFLICT);

        let (org_id, _) = org_with_site(&storage).await;
        let a = person(&state, org_id, OWNER, &[]).await;
        let b = person(&state, org_id, OWNER, &[]).await;
        let (auth_a, auth_b) = (
            authorized::<DeleteUsers>(&state, &a).await,
            authorized::<DeleteUsers>(&state, &b).await,
        );
        let (first, second) = tokio::join!(
            tokio::spawn(bulk_delete_users(
                State(state.clone()),
                auth_a,
                ApiJson(vec![b.id])
            )),
            tokio::spawn(bulk_delete_users(
                State(state.clone()),
                auth_b,
                ApiJson(vec![a.id])
            )),
        );
        first.unwrap().ok();
        second.unwrap().ok();
        assert_eq!(owners(&state, org_id).await, 1, "bulk, round {round}");

        let (org_id, _) = org_with_site(&storage).await;
        let a = person(&state, org_id, OWNER, &[]).await;
        let b = person(&state, org_id, OWNER, &[]).await;
        let demote = |id: Uuid| {
            let state = state.clone();
            tokio::spawn(async move {
                state
                    .services
                    .access_service
                    .set_access(
                        Principal::User(id),
                        MEMBER,
                        &[],
                        AuthenticatedEntity::System,
                    )
                    .await
            })
        };
        let (first, second) = tokio::join!(demote(a.id), demote(b.id));
        let demoted = [first.unwrap(), second.unwrap()]
            .iter()
            .filter(|result| result.is_ok())
            .count();
        assert_eq!(owners(&state, org_id).await, 1, "demote, round {round}");
        assert_eq!(demoted, 1, "demote, round {round}");
    }
}
