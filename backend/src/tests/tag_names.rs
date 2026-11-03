//! A tag's name is unique among the organization's live tags. A deleted tag is closed, not
//! removed, and its name can be given to a new tag (GH #841).

use std::sync::Arc;

use axum::body::Body;
use axum::extract::{FromRequestParts, State};
use axum::http::Request;
use uuid::Uuid;

use super::{organization, test_state, user};
use crate::server::auth::access::Assignment;
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::auth::middleware::permissions::{Authorized, Can, Write};
use crate::server::config::AppState;
use crate::server::shared::events::types::DeletionReason;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::traits::Storage;
use crate::server::shared::types::api::ApiJson;
use crate::server::site_assignments::r#impl::base::Principal;
use crate::server::tags::handlers::create_tag;
use crate::server::tags::r#impl::base::{Tag, TagBase};
use crate::server::users::r#impl::{base::User, permissions::UserOrgPermissions};

async fn owner(state: &AppState, org_id: Uuid) -> User {
    let role = UserOrgPermissions::Owner;
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
            &Assignment::for_person(role, &[]),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();
    u
}

async fn as_caller(state: &Arc<AppState>, caller: &User) -> Authorized<Can<Tag, Write>> {
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
    Authorized::from_request_parts(&mut parts, state)
        .await
        .ok()
        .expect("the caller passes the permission check")
}

fn named(org_id: Uuid, name: &str) -> Tag {
    Tag {
        id: Uuid::new_v4(),
        base: TagBase {
            name: name.to_string(),
            organization_id: org_id,
            ..Default::default()
        },
        ..Default::default()
    }
}

async fn create(state: &Arc<AppState>, caller: &User, tag: Tag) -> Result<Tag, String> {
    let auth = as_caller(state, caller).await;
    create_tag(State(state.clone()), auth, ApiJson(tag))
        .await
        .map(|response| response.0.data().cloned().expect("the created tag"))
        .map_err(|e| format!("{e:?}"))
}

#[tokio::test]
async fn a_deleted_tags_name_can_be_used_again() {
    let (storage, state, _container) = test_state().await;
    let state = Arc::new(state);
    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let caller = owner(&state, org.id).await;

    let media = create(&state, &caller, named(org.id, "Media"))
        .await
        .unwrap();
    assert!(
        create(&state, &caller, named(org.id, "Media"))
            .await
            .is_err(),
        "a live tag holds its name"
    );

    state
        .services
        .tag_service
        .delete(
            &media.id,
            AuthenticatedEntity::System,
            DeletionReason::EntityRequested,
        )
        .await
        .unwrap();

    let again = create(&state, &caller, named(org.id, "Media"))
        .await
        .expect("a deleted tag no longer holds its name");
    assert_ne!(again.id, media.id);
}
