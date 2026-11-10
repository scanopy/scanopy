//! The organization's members list: its users and its pending invites in one table, paged,
//! sorted, grouped and filtered on the server.
//!
//! Two tables feed it and most of its columns are derived (a user's status depends on who is
//! looking, their sign-in method on two fields), so it is built in Rust from the rows the caller may
//! see and ordered with [`computed_page`]. An organization's users and invites are few.

use std::sync::Arc;

use axum::{extract::State, response::Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::server::{
    auth::{
        access::Grants,
        middleware::permissions::{Authorized, Can, Read},
    },
    config::AppState,
    invites::r#impl::base::Invite,
    shared::{
        extractors::Query,
        handlers::{
            list::{Computed, ComputedValue, ListOrdering, ListRows, computed_page},
            ordering::OrderField,
            query::{OrderDirection, PaginationParams, deserialize_bool_values},
            traits::CrudHandlers,
        },
        services::traits::CrudService,
        storage::{filter::StorableFilter, traits::Entity as _},
        types::api::{ApiError, ApiResult, PaginatedApiResponse},
    },
    users::r#impl::{base::User, permissions::UserOrgPermissions},
};

/// A member's place in the organization, as the status column shows it.
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq, strum::IntoStaticStr,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum MemberStatus {
    /// An invite nobody has accepted yet.
    Pending,
    /// The signed-in user.
    You,
    /// Any other user.
    User,
}

/// One row of the members list: a user, or an invite still waiting to be accepted.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Member {
    #[schema(title = "User")]
    User {
        /// The user's id.
        id: Uuid,
        data: Box<User>,
        status: MemberStatus,
        /// How the user signs in: `<provider>`, `<provider>+password`, `password` or `none`.
        /// Provider names come from the server's OIDC config.
        auth_method: String,
    },
    #[schema(title = "Invite")]
    Invite {
        /// The invite's id.
        id: Uuid,
        data: Invite,
        status: MemberStatus,
    },
}

/// The password half of an auth method; the provider's name is the other half.
const PASSWORD_METHOD: &str = "password";

impl Member {
    fn user(user: User, viewer_id: Uuid) -> Self {
        let status = if user.id == viewer_id {
            MemberStatus::You
        } else {
            MemberStatus::User
        };
        let auth_method = match (&user.base.oidc_provider, user.base.has_password) {
            (Some(provider), true) => format!("{provider}+{PASSWORD_METHOD}"),
            (Some(provider), false) => provider.clone(),
            (None, true) => PASSWORD_METHOD.to_string(),
            (None, false) => "none".to_string(),
        };
        Self::User {
            id: user.id,
            data: Box::new(user),
            status,
            auth_method,
        }
    }

    fn invite(invite: Invite) -> Self {
        Self::Invite {
            id: invite.id,
            data: invite,
            status: MemberStatus::Pending,
        }
    }

    fn status(&self) -> MemberStatus {
        match self {
            Self::User { status, .. } | Self::Invite { status, .. } => *status,
        }
    }

    fn permissions(&self) -> UserOrgPermissions {
        match self {
            Self::User { data, .. } => data.base.permissions,
            Self::Invite { data, .. } => data.base.permissions,
        }
    }

    /// The email the row reads under: the user's, or the address an invite was sent to.
    fn email(&self) -> Option<String> {
        match self {
            Self::User { data, .. } => Some(data.base.email.to_string()),
            Self::Invite { data, .. } => data.base.send_to.as_ref().map(ToString::to_string),
        }
    }

    fn user_data(&self) -> Option<&User> {
        match self {
            Self::User { data, .. } => Some(data.as_ref()),
            Self::Invite { .. } => None,
        }
    }

    fn invite_data(&self) -> Option<&Invite> {
        match self {
            Self::Invite { data, .. } => Some(data),
            Self::User { .. } => None,
        }
    }
}

/// Fields the members list can be ordered and grouped by. Every one is derived in Rust.
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, Default, utoipa::ToSchema, strum::EnumIter,
)]
#[serde(rename_all = "snake_case")]
pub enum MemberOrderField {
    #[default]
    CreatedAt,
    Email,
    /// Pending invite, the signed-in user, or another user.
    InviteStatus,
    Permissions,
    /// How the user signs in.
    OidcProvider,
    UpdatedAt,
    /// Who created an invite.
    InvitedBy,
    ExpiresAt,
    EmailVerified,
    OidcLinkedAt,
}

impl OrderField for MemberOrderField {
    /// Never read: the list is ordered in memory.
    fn to_sql(&self) -> &'static str {
        "id"
    }
}

/// Query parameters for filtering and ordering the members list.
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct MemberFilterQuery {
    /// Only members in one of these statuses. Repeat for several.
    pub statuses: Option<Vec<MemberStatus>>,
    /// Only members with one of these roles. Repeat for several.
    pub permissions: Option<Vec<UserOrgPermissions>>,
    /// Only users assigned one of these sites. Repeat for several.
    pub site_ids: Option<Vec<Uuid>>,
    /// `true` also returns admins and owners, who reach every site. Set on its own it returns only
    /// those.
    pub all_sites: Option<bool>,
    /// Only users signing in one of these ways (`<provider>`, `<provider>+password`, `password`,
    /// `none`). Repeat for several.
    pub auth_methods: Option<Vec<String>>,
    /// Only invites created by one of these users. Repeat for several.
    pub invited_by: Option<Vec<Uuid>>,
    /// Only users whose email is verified (`true`), unverified (`false`), or both (repeat it).
    /// Sent empty, matches none.
    #[serde(default, deserialize_with = "deserialize_bool_values")]
    pub email_verified: Option<Vec<bool>>,
    /// Free-text search. Case-insensitive substring match against the email.
    pub search: Option<String>,
    /// Primary ordering field (used for grouping). Always sorts ASC to keep groups together.
    pub group_by: Option<MemberOrderField>,
    /// Secondary ordering field (sorting within groups or standalone sort).
    pub order_by: Option<MemberOrderField>,
    /// Direction for order_by field (group_by always uses ASC).
    pub order_direction: Option<OrderDirection>,
    /// Maximum number of results to return (1-1000, default: 50). Use 0 for no limit.
    #[param(minimum = 0, maximum = 1000)]
    pub limit: Option<u32>,
    /// Number of results to skip. Default: 0.
    #[param(minimum = 0)]
    pub offset: Option<u32>,
}

impl MemberFilterQuery {
    fn pagination(&self) -> PaginationParams {
        PaginationParams {
            limit: self.limit,
            offset: self.offset,
        }
    }

    /// The site filter: users assigned a requested site, and with `all_sites`, the admins and
    /// owners who reach every one. Invites match no site.
    fn matches_sites(&self, member: &Member) -> bool {
        let requested = self.site_ids.as_deref().unwrap_or_default();
        let every = self.all_sites.unwrap_or(false);
        if requested.is_empty() && !every {
            return true;
        }
        let Some(user) = member.user_data() else {
            return false;
        };
        if matches!(
            user.base.permissions,
            UserOrgPermissions::Owner | UserOrgPermissions::Admin
        ) {
            return every;
        }
        user.base.site_ids.iter().any(|id| requested.contains(id))
    }
}

/// Whether a filter passes: unset or empty passes everything, otherwise some value must match.
fn any_of<V>(values: &Option<Vec<V>>, test: impl Fn(&V) -> bool) -> bool {
    values
        .as_ref()
        .is_none_or(|values| values.is_empty() || values.iter().any(test))
}

/// The members list's rows: users and invites, ordered and filtered in code.
pub struct MemberList;

impl ListRows for MemberList {
    type Item = Member;
    type Field = MemberOrderField;
    type Query = MemberFilterQuery;

    fn ordering(query: &MemberFilterQuery) -> ListOrdering<MemberOrderField> {
        ListOrdering {
            group_by: query.group_by,
            order_by: query.order_by,
            direction: query.order_direction,
            tag_group: Default::default(),
        }
    }

    /// Whether `member` passes every filter.
    fn keep(query: &MemberFilterQuery, member: &Member) -> bool {
        let search = query
            .search
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_lowercase);

        search.is_none_or(|s| {
            member
                .email()
                .is_some_and(|e| e.to_lowercase().contains(&s))
        }) && any_of(&query.statuses, |s| *s == member.status())
            && any_of(&query.permissions, |p| *p == member.permissions())
            && query.matches_sites(member)
            && any_of(&query.auth_methods, |m| match member {
                Member::User { auth_method, .. } => auth_method == m,
                Member::Invite { .. } => false,
            })
            && any_of(&query.invited_by, |id| {
                member
                    .invite_data()
                    .is_some_and(|i| i.base.created_by == *id)
            })
            && query.email_verified.as_ref().is_none_or(|values| {
                member
                    .user_data()
                    .is_some_and(|u| values.contains(&u.base.email_verified))
            })
    }

    /// The value a member sorts and groups by on `field`.
    fn computed(field: &MemberOrderField, member: &Member) -> Option<Computed> {
        let text = |value: Option<String>| Computed {
            value: value.clone().map(ComputedValue::Text),
            group: value,
        };
        let time = |value: Option<DateTime<Utc>>| Computed {
            value: value.map(|t| ComputedValue::Number(t.timestamp_millis() as f64)),
            group: value.map(|t| t.to_rfc3339()),
        };
        Some(match field {
            MemberOrderField::Email => text(member.email().map(|e| e.to_lowercase())),
            MemberOrderField::InviteStatus => text(Some(<&str>::from(member.status()).to_string())),
            MemberOrderField::Permissions => text(Some(member.permissions().to_string())),
            MemberOrderField::OidcProvider => text(match member {
                Member::User { auth_method, .. } => Some(auth_method.clone()),
                Member::Invite { .. } => None,
            }),
            MemberOrderField::InvitedBy => {
                text(member.invite_data().map(|i| i.base.created_by.to_string()))
            }
            MemberOrderField::EmailVerified => text(
                member
                    .user_data()
                    .map(|u| u.base.email_verified.to_string()),
            ),
            MemberOrderField::CreatedAt => time(Some(match member {
                Member::User { data, .. } => data.created_at,
                Member::Invite { data, .. } => data.created_at,
            })),
            MemberOrderField::UpdatedAt => time(Some(match member {
                Member::User { data, .. } => data.updated_at,
                Member::Invite { data, .. } => data.updated_at,
            })),
            MemberOrderField::ExpiresAt => time(member.invite_data().map(|i| i.base.expires_at)),
            MemberOrderField::OidcLinkedAt => {
                time(member.user_data().and_then(|u| u.base.oidc_linked_at))
            }
        })
    }
}

/// List members
///
/// The organization's users and its pending invites in one list, each limited to the ones the
/// caller may see: every user and invite for an owner, otherwise the caller themselves, members
/// below their role, and invites they created.
#[utoipa::path(
    get,
    path = "/members",
    tag = User::ENTITY_NAME_PLURAL,
    operation_id = "list_members",
    params(MemberFilterQuery),
    responses(
        (status = 200, description = "Users and pending invites", body = PaginatedApiResponse<Member>),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
pub async fn list_members(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<User, Read>>,
    Query(query): Query<MemberFilterQuery>,
) -> ApiResult<Json<PaginatedApiResponse<Member>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let viewer_id = auth.user_id().ok_or_else(ApiError::user_required)?;
    let viewer_grants = auth
        .entity
        .organization_grants()
        .ok_or_else(ApiError::user_required)?;
    Ok(Json(
        members_page(&state, organization_id, viewer_id, &viewer_grants, &query).await?,
    ))
}

/// One page of the members list for a viewer with these organization grants.
pub async fn members_page(
    state: &AppState,
    organization_id: Uuid,
    viewer_id: Uuid,
    viewer_grants: &Grants,
    query: &MemberFilterQuery,
) -> ApiResult<PaginatedApiResponse<Member>> {
    let service = User::get_service(state);
    let mut users = service
        .get_all(StorableFilter::<User>::new_from_org_id(&organization_id))
        .await?;
    service.hydrate_site_ids_batch(&mut users).await?;
    let users = users
        .into_iter()
        .filter(|u| u.is_listed_for(viewer_grants, viewer_id))
        .map(|u| Member::user(u, viewer_id));
    let invites = state
        .services
        .invite_service
        .list_active_invites(&organization_id)
        .await
        .into_iter()
        .filter(|i| i.is_listed_for(viewer_grants, viewer_id))
        .map(Member::invite);

    Ok(computed_page::<MemberList>(
        users.chain(invites).collect(),
        query,
        query.pagination(),
    ))
}
