use crate::server::auth::access::{Action, Keying, Resource};
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::auth::middleware::features::{RequireFeature, TaggingFeature};
use crate::server::auth::middleware::permissions::{Authorized, Can, InOrganization, Read, Write};
use crate::server::shared::entities::{Entity as EntityEnum, EntityDiscriminants};
use crate::server::shared::events::log::EventLog;
use crate::server::shared::events::traits::{EntityScope, Event};
use crate::server::shared::events::types::{EntityOperation, UpdateReason};
use crate::server::shared::extractors::Query;
use crate::server::shared::handlers::list::list_page;
use crate::server::shared::handlers::ordering::OrderField;
use crate::server::shared::handlers::query::{
    FilterQueryExtractor, OrderDirection, PaginationParams,
};
use crate::server::shared::handlers::traits::{ListScope, create_handler};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::{Entity, Storable};
use crate::server::shared::types::api::ApiJson;
use crate::server::shared::types::api::{ApiError, ApiErrorResponse, PaginatedApiResponse};
use crate::server::tags::groups::handlers::SYSTEM_SQL;
use crate::server::tags::r#impl::base::Tag;
use crate::server::user_api_keys::r#impl::base::UserApiKey;
use crate::server::{
    config::AppState,
    shared::types::api::{ApiResponse, ApiResult, EmptyApiResponse},
};
use axum::{extract::State, response::Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

// ============================================================================
// Tag Ordering
// ============================================================================

/// The key a tag's group sorts and groups under, in the order the tag groups list uses: system
/// groups first, then by type and lowercased name, then the group's id so two groups never share
/// a key. `<0 or 1>:<type>:<name>:<id>`; NULL for a tag in no group. The UI builds the same key to
/// find a group's total in `group_counts`.
static TAG_GROUP_KEY_SQL: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    format!(
        "(CASE WHEN tag_groups.id IS NULL THEN NULL ELSE \
         (CASE WHEN {system} THEN '0' ELSE '1' END) || ':' || tag_groups.group_type || ':' \
         || lower(tag_groups.name) || ':' || tag_groups.id::text END)",
        system = SYSTEM_SQL.as_str(),
    )
});

/// Fields that tags can be ordered/grouped by.
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, Default, utoipa::ToSchema, strum::EnumIter,
)]
#[serde(rename_all = "snake_case")]
pub enum TagOrderField {
    #[default]
    CreatedAt,
    Name,
    Color,
    UpdatedAt,
    /// The tag's group, in the order the tag groups list uses. Tags in no group sort last.
    TagGroup,
    /// The tag's icon. Tags with none sort last.
    Icon,
}

impl OrderField for TagOrderField {
    fn to_sql(&self) -> &'static str {
        match self {
            Self::CreatedAt => "tags.created_at",
            Self::Name => "tags.name",
            Self::Color => "tags.color",
            Self::UpdatedAt => "tags.updated_at",
            Self::TagGroup => TAG_GROUP_KEY_SQL.as_str(),
            Self::Icon => "tags.icon",
        }
    }

    fn join_sql(&self) -> Option<&'static str> {
        match self {
            Self::TagGroup => Some("LEFT JOIN tag_groups ON tag_groups.id = tags.tag_group_id"),
            Self::CreatedAt | Self::Name | Self::Color | Self::UpdatedAt | Self::Icon => None,
        }
    }

    fn nulls_last(&self) -> bool {
        matches!(self, Self::TagGroup | Self::Icon)
    }

    /// Sorted by tag group, tags follow their groups and then each group's own order: placed
    /// tags by place, then the rest by name.
    fn order_term(&self, dir: &str) -> String {
        match self {
            Self::TagGroup => format!(
                "{} {dir} NULLS LAST, tags.position ASC NULLS LAST, tags.name ASC",
                self.to_sql()
            ),
            _ if self.nulls_last() => format!("{} {dir} NULLS LAST", self.to_sql()),
            _ => format!("{} {dir}", self.to_sql()),
        }
    }
}

// ============================================================================
// Tag Filter Query
// ============================================================================

/// Query parameters for filtering and ordering tags.
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct TagFilterQuery {
    /// Only tags of one of these colours. Repeat for several.
    pub colors: Option<Vec<String>>,
    /// Only tags in one of these groups. Repeat for several.
    pub tag_group_ids: Option<Vec<Uuid>>,
    /// Only tags with one of these icons. Repeat for several.
    pub icons: Option<Vec<String>>,
    /// Free-text search. Case-insensitive substring match against the name and description.
    pub search: Option<String>,
    /// Primary ordering field (used for grouping). Always sorts ASC to keep groups together.
    pub group_by: Option<TagOrderField>,
    /// Secondary ordering field (sorting within groups or standalone sort).
    pub order_by: Option<TagOrderField>,
    /// Direction for order_by field (group_by always uses ASC).
    pub order_direction: Option<OrderDirection>,
    /// Maximum number of results to return (1-1000, default: 50). Use 0 for no limit.
    #[param(minimum = 0, maximum = 1000)]
    pub limit: Option<u32>,
    /// Number of results to skip. Default: 0.
    #[param(minimum = 0)]
    pub offset: Option<u32>,
    /// As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
    /// instant (snapshot view) instead of live state.
    pub at: Option<chrono::DateTime<chrono::Utc>>,
}

impl FilterQueryExtractor for TagFilterQuery {
    fn apply_to_filter<T: Storable>(
        &self,
        filter: StorableFilter<T>,
        _user_site_ids: &[Uuid],
        _user_organization_id: Uuid,
    ) -> StorableFilter<T> {
        let filter = match &self.colors {
            Some(colors) if !colors.is_empty() => filter.text_column_in("color", colors),
            _ => filter,
        };
        let filter = match &self.tag_group_ids {
            Some(ids) if !ids.is_empty() => filter.expr_text_in(
                &format!("{}.tag_group_id", T::table_name()),
                &ids.iter().map(Uuid::to_string).collect::<Vec<_>>(),
            ),
            _ => filter,
        };
        match &self.icons {
            Some(icons) if !icons.is_empty() => filter.text_column_in("icon", icons),
            _ => filter,
        }
    }

    fn pagination(&self) -> PaginationParams {
        PaginationParams {
            limit: self.limit,
            offset: self.offset,
        }
    }

    fn at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.at
    }
}

// Generated handlers for most CRUD operations
mod generated {
    use super::*;
    crate::crud_get_by_id_handler!(Tag);
    crate::crud_update_handler!(Tag);
    crate::crud_delete_handler!(Tag);
    crate::crud_bulk_delete_handler!(Tag);
    crate::crud_export_csv_handler!(Tag);
    crate::crud_get_field_values_handler!(Tag);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all_tags, create_tag))
        .routes(routes!(
            generated::get_by_id,
            generated::update,
            generated::delete
        ))
        .routes(routes!(generated::bulk_delete))
        .routes(routes!(generated::export_csv))
        .routes(routes!(generated::get_field_values))
        // Entity tag assignment routes
        .routes(routes!(bulk_add_tag))
        .routes(routes!(bulk_remove_tag))
        .routes(routes!(set_entity_tags))
}

/// List tags
///
/// Returns all tags in the authenticated user's organization.
/// Supports pagination via `limit` and `offset` query parameters,
/// and ordering via `group_by`, `order_by`, and `order_direction`.
#[utoipa::path(
    get,
    operation_id = "list_tags",
    path = "",
    tag = Tag::ENTITY_NAME_PLURAL,
    params(TagFilterQuery),
    responses(
        (status = 200, description = "List of tags", body = PaginatedApiResponse<Tag>),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_all_tags(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Tag, Read>>,
    Query(query): Query<TagFilterQuery>,
) -> ApiResult<Json<PaginatedApiResponse<Tag>>> {
    // Tags are the organization's: `StorableFilter::new_for_access` scopes them to it.
    let scope = ListScope::of(&auth)?;
    Ok(Json(list_page::<Tag>(&state, &scope, &query).await?))
}

/// Create tag
///
/// Creates a tag scoped to your organization. Tag names must be unique within the organization.
///
/// ### Validation
///
/// - Name must be 1-100 characters (empty names are rejected)
/// - Name must be unique within your organization
#[utoipa::path(
    post,
    path = "",
    tag = Tag::ENTITY_NAME_PLURAL,
    request_body = Tag,
    responses(
        (status = 200, description = "Tag created successfully", body = ApiResponse<Tag>),
        (status = 400, description = "Validation error: name empty or too long", body = ApiErrorResponse),
        (status = 409, description = "Tag name already exists in this organization", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn create_tag(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Tag, Write>>,
    ApiJson(tag): ApiJson<Tag>,
) -> ApiResult<Json<ApiResponse<Tag>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;
    // A deleted tag is closed, not removed, and gives up its name, as `idx_tags_org_name_live` does.
    let name_filter = StorableFilter::<Tag>::new_from_org_id(&organization_id)
        .name(tag.base.name.clone())
        .live();

    if state.services.tag_service.exists(name_filter).await? {
        return Err(ApiError::conflict(&format!(
            "Tag names must be unique; a tag named \"{}\" already exists",
            tag.base.name
        )));
    }

    create_handler::<Tag>(State(state), auth, ApiJson(tag)).await
}

/// Tagging an entity takes `Tag` Write and Read on that entity, so the entity type in the request
/// decides which other resource the caller needs.
fn require_write_on(
    auth: &Authorized<InOrganization>,
    entity_type: EntityDiscriminants,
) -> Result<(), ApiError> {
    if auth.entity.can(Resource::Tag, Action::Write)
        && auth.entity.can(entity_type.resource(), Action::Read)
    {
        Ok(())
    } else {
        Err(ApiError::permission_denied())
    }
}

/// Resolve site_id and organization_id for an entity by looking it up via its service.
async fn resolve_scope<T>(
    service: &(impl CrudService<T> + Sync),
    id: &Uuid,
) -> (Option<Uuid>, Option<Uuid>)
where
    T: Entity
        + Into<EntityEnum>
        + std::fmt::Display
        + crate::server::shared::entities::ChangeTriggersTopologyStaleness<T>,
{
    service
        .get_by_id(id)
        .await
        .ok()
        .flatten()
        .map(|e| (e.site_id(), e.organization_id()))
        .unwrap_or((None, None))
}

/// Resolve site_id and organization_id for any entity type.
async fn resolve_entity_scope(
    state: &AppState,
    entity_id: &Uuid,
    entity_type: EntityDiscriminants,
) -> (Option<Uuid>, Option<Uuid>) {
    let s = &state.services;
    match entity_type {
        EntityDiscriminants::Host => resolve_scope(s.host_service.as_ref(), entity_id).await,
        EntityDiscriminants::Service => resolve_scope(s.service_service.as_ref(), entity_id).await,
        EntityDiscriminants::Subnet => resolve_scope(s.subnet_service.as_ref(), entity_id).await,
        EntityDiscriminants::Dependency => {
            resolve_scope(s.dependency_service.as_ref(), entity_id).await
        }
        EntityDiscriminants::Site => resolve_scope(s.site_service.as_ref(), entity_id).await,
        EntityDiscriminants::Discovery => {
            resolve_scope(s.discovery_service.as_ref(), entity_id).await
        }
        EntityDiscriminants::Daemon => resolve_scope(s.daemon_service.as_ref(), entity_id).await,
        EntityDiscriminants::DaemonApiKey => {
            resolve_scope(s.daemon_api_key_service.as_ref(), entity_id).await
        }
        EntityDiscriminants::UserApiKey => {
            resolve_scope(s.user_api_key_service.as_ref(), entity_id).await
        }
        EntityDiscriminants::Tag => resolve_scope(s.tag_service.as_ref(), entity_id).await,
        EntityDiscriminants::TagGroup => {
            resolve_scope(s.tag_group_service.as_ref(), entity_id).await
        }
        EntityDiscriminants::Organization => {
            resolve_scope(s.organization_service.as_ref(), entity_id).await
        }
        EntityDiscriminants::User => resolve_scope(s.user_service.as_ref(), entity_id).await,
        EntityDiscriminants::Invite => resolve_scope(s.invite_service.as_ref(), entity_id).await,
        EntityDiscriminants::Share => resolve_scope(s.share_service.as_ref(), entity_id).await,
        EntityDiscriminants::Topology => {
            resolve_scope(s.topology_service.as_ref(), entity_id).await
        }
        EntityDiscriminants::Port => resolve_scope(s.port_service.as_ref(), entity_id).await,
        EntityDiscriminants::Binding => resolve_scope(s.binding_service.as_ref(), entity_id).await,
        EntityDiscriminants::IPAddress => {
            resolve_scope(s.ip_address_service.as_ref(), entity_id).await
        }
        EntityDiscriminants::Interface => {
            resolve_scope(s.interface_service.as_ref(), entity_id).await
        }
        EntityDiscriminants::Credential => {
            resolve_scope(s.credential_service.as_ref(), entity_id).await
        }
        EntityDiscriminants::Vlan => resolve_scope(s.vlan_service.as_ref(), entity_id).await,
        // Snapshots aren't user-taggable, but the match must be exhaustive.
        EntityDiscriminants::Snapshot => (None, None),
    }
}

/// Validate that every `entity_id` belongs to the caller's tenant before we
/// write tag-junction rows for it. Tag-org is validated separately by the
/// service (`validate_tag_full`); this guards the *target entities*, which are
/// otherwise caller-supplied ids written straight to the junction. Resolves each
/// id's `(site_id, organization_id)` and requires Read on the entity: on its site
/// for a site-keyed one, in the caller's organization otherwise. An id that can't
/// be resolved (missing / foreign) is rejected with the same error as a
/// cross-tenant id (no existence oracle).
///
/// A user's API keys are theirs alone, whatever their role.
async fn validate_entities_in_tenant(
    state: &AppState,
    auth: &Authorized<InOrganization>,
    entity_ids: &[Uuid],
    entity_type: EntityDiscriminants,
) -> Result<(), ApiError> {
    let resource = entity_type.resource();
    if resource == Resource::UserApiKey {
        for entity_id in entity_ids {
            let key = state
                .services
                .user_api_key_service
                .get_by_id(entity_id)
                .await?;
            if key.map(|key| key.base.user_id) != auth.user_id() {
                return Err(ApiError::entity_access_denied::<UserApiKey>(*entity_id));
            }
        }
    }
    let org_id = auth.require_organization_id()?;
    let site_ids = auth.entity.site_ids_for(resource, Action::Read);
    for entity_id in entity_ids {
        let (site, org) = resolve_entity_scope(state, entity_id, entity_type).await;
        let readable = match resource.keying() {
            Keying::Site => site.is_some_and(|site| site_ids.contains(&site)),
            Keying::Organization => org == Some(org_id),
        };
        if !readable {
            return Err(ApiError::forbidden(
                "Cannot modify tags on an entity outside your organization",
            ));
        }
    }
    Ok(())
}

/// Emit EntityEvent::Updated for each entity whose tags changed.
/// This triggers subscribers (like topology) to refresh their snapshots.
///
/// When `trigger_stale` is true, the topology subscriber marks the site's
/// topology stale — used when an application tag is added or removed, since
/// that changes the Application-perspective container structure.
async fn emit_tag_change_events(
    state: &AppState,
    auth: &AuthenticatedEntity,
    entity_ids: &[Uuid],
    entity_type: EntityDiscriminants,
    _trigger_stale: bool,
) {
    let default_entity: EntityEnum = entity_type.into();

    for entity_id in entity_ids {
        let (site_id, organization_id) = resolve_entity_scope(state, entity_id, entity_type).await;

        if let Some(scope) = EntityScope::from_ids(
            *entity_id,
            default_entity.clone(),
            site_id,
            organization_id.or(auth.organization_id()),
        ) {
            state
                .services
                .event_bus
                // A refresh signal that carries a default entity in place of the real one, so
                // it has nothing to log.
                .publish(
                    Event::new(
                        scope,
                        EntityOperation::Updated {
                            reason: UpdateReason::EntityTagsChanged,
                        },
                        auth.clone(),
                    )
                    .with_log(EventLog::hidden()),
                )
                .await;
        }
    }
}

/// Request body for bulk tag operations
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BulkTagRequest {
    /// The entity type (e.g., Host, Service, Subnet)
    pub entity_type: EntityDiscriminants,
    /// The IDs of entities to modify
    pub entity_ids: Vec<Uuid>,
    /// The tag ID to add or remove
    pub tag_id: Uuid,
}

/// Response for bulk tag operations
#[derive(Debug, Serialize, ToSchema)]
pub struct BulkTagResponse {
    /// Number of entities affected
    pub affected_count: usize,
}

/// Request body for setting all tags on an entity
#[derive(Debug, Deserialize, ToSchema)]
pub struct SetTagsRequest {
    /// The entity type (e.g., Host, Service, Subnet)
    pub entity_type: EntityDiscriminants,
    /// The entity ID
    pub entity_id: Uuid,
    /// The new list of tag IDs
    pub tag_ids: Vec<Uuid>,
}

/// Bulk add tag to entities
///
/// Adds a single tag to multiple entities of the same type. This is useful for batch tagging operations.
///
/// ### Validation
///
/// - Entity type must be taggable (Host, Service, Subnet, Vlan, Dependency, Site, Discovery, Daemon, DaemonApiKey, UserApiKey, Credential)
/// - Tag must exist and belong to your organization
/// - Entities that already have the tag are silently skipped
#[utoipa::path(
    post,
    path = "/assign/bulk-add",
    tag = Tag::ENTITY_NAME_PLURAL,
    request_body = BulkTagRequest,
    responses(
        (status = 200, description = "Tag added successfully", body = ApiResponse<BulkTagResponse>),
        (status = 400, description = "Invalid entity type or tag", body = ApiErrorResponse),
        (status = 404, description = "Tag not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
pub async fn bulk_add_tag(
    State(state): State<Arc<AppState>>,
    auth: Authorized<InOrganization>,
    _tagging: RequireFeature<TaggingFeature>,
    ApiJson(request): ApiJson<BulkTagRequest>,
) -> ApiResult<Json<ApiResponse<BulkTagResponse>>> {
    require_write_on(&auth, request.entity_type)?;

    // Validate entity type is taggable
    if !request.entity_type.is_taggable() {
        return Err(ApiError::bad_request(&format!(
            "Entity type {:?} does not support tagging",
            request.entity_type
        )));
    }

    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;

    validate_entities_in_tenant(&state, &auth, &request.entity_ids, request.entity_type).await?;

    let affected_count = state
        .services
        .entity_tag_service
        .bulk_add_tag(
            &request.entity_ids,
            request.entity_type,
            request.tag_id,
            organization_id,
        )
        .await?;

    if affected_count > 0 {
        // Topology staleness model is gone — fire tag-change events
        // unconditionally; the topology subscriber broadcasts a live-update
        // ping for the affected site(s) on receipt.
        emit_tag_change_events(
            &state,
            &auth.entity,
            &request.entity_ids,
            request.entity_type,
            false,
        )
        .await;
    }

    Ok(Json(ApiResponse::success(BulkTagResponse {
        affected_count,
    })))
}

/// Bulk remove tag from entities
///
/// Removes a single tag from multiple entities of the same type.
///
/// ### Validation
///
/// - Entity type must be taggable (Host, Service, Subnet, Vlan, Dependency, Site, Discovery, Daemon, DaemonApiKey, UserApiKey, Credential)
/// - Entities that don't have the tag are silently skipped
#[utoipa::path(
    post,
    path = "/assign/bulk-remove",
    tag = Tag::ENTITY_NAME_PLURAL,
    request_body = BulkTagRequest,
    responses(
        (status = 200, description = "Tag removed successfully", body = ApiResponse<BulkTagResponse>),
        (status = 400, description = "Invalid entity type", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
pub async fn bulk_remove_tag(
    State(state): State<Arc<AppState>>,
    auth: Authorized<InOrganization>,
    _tagging: RequireFeature<TaggingFeature>,
    ApiJson(request): ApiJson<BulkTagRequest>,
) -> ApiResult<Json<ApiResponse<BulkTagResponse>>> {
    require_write_on(&auth, request.entity_type)?;

    // Validate entity type is taggable
    if !request.entity_type.is_taggable() {
        return Err(ApiError::bad_request(&format!(
            "Entity type {:?} does not support tagging",
            request.entity_type
        )));
    }

    // Assert auth context has an organization — bulk-remove is org-scoped
    // even though we no longer thread the id through to the staleness check.
    let _organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;

    validate_entities_in_tenant(&state, &auth, &request.entity_ids, request.entity_type).await?;

    let affected_count = state
        .services
        .entity_tag_service
        .bulk_remove_tag(&request.entity_ids, request.entity_type, request.tag_id)
        .await?;

    if affected_count > 0 {
        emit_tag_change_events(
            &state,
            &auth.entity,
            &request.entity_ids,
            request.entity_type,
            false,
        )
        .await;
    }

    Ok(Json(ApiResponse::success(BulkTagResponse {
        affected_count,
    })))
}

/// Set entity tags
///
/// Replaces all tags on an entity with the provided list.
///
/// ### Validation
///
/// - Entity type must be taggable (Host, Service, Subnet, Vlan, Dependency, Site, Discovery, Daemon, DaemonApiKey, UserApiKey, Credential)
/// - All tags must exist and belong to your organization
#[utoipa::path(
    put,
    path = "/assign",
    tag = Tag::ENTITY_NAME_PLURAL,
    request_body = SetTagsRequest,
    responses(
        (status = 200, description = "Tags set successfully", body = EmptyApiResponse),
        (status = 400, description = "Invalid entity type or tag", body = ApiErrorResponse),
        (status = 404, description = "Tag not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
pub async fn set_entity_tags(
    State(state): State<Arc<AppState>>,
    auth: Authorized<InOrganization>,
    _tagging: RequireFeature<TaggingFeature>,
    ApiJson(request): ApiJson<SetTagsRequest>,
) -> ApiResult<Json<ApiResponse<()>>> {
    require_write_on(&auth, request.entity_type)?;

    // Validate entity type is taggable
    if !request.entity_type.is_taggable() {
        return Err(ApiError::bad_request(&format!(
            "Entity type {:?} does not support tagging",
            request.entity_type
        )));
    }

    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;

    validate_entities_in_tenant(
        &state,
        &auth,
        std::slice::from_ref(&request.entity_id),
        request.entity_type,
    )
    .await?;

    state
        .services
        .entity_tag_service
        .set_tags(
            request.entity_id,
            request.entity_type,
            request.tag_ids,
            organization_id,
        )
        .await?;

    // Topology staleness model is gone — emit tag-change events
    // unconditionally; the topology subscriber broadcasts a live-update
    // ping for the affected site(s) on receipt.
    emit_tag_change_events(
        &state,
        &auth.entity,
        &[request.entity_id],
        request.entity_type,
        false,
    )
    .await;

    Ok(Json(ApiResponse::success(())))
}
