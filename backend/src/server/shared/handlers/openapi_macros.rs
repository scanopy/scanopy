//! Macros for generating OpenAPI-annotated CRUD handlers
//!
//! These macros generate thin wrapper handlers with `#[utoipa::path]` annotations
//! that delegate to the generic CRUD handlers. Use them for operations that use
//! the standard generic handlers - write custom handlers for operations that need
//! custom logic.
//!
//! Query param documentation is automatically derived from the filter query type's
//! `IntoParams` implementation. Add doc comments to fields in query structs to
//! customize the OpenAPI parameter descriptions.
//!
//! # Usage
//! ```ignore
//! // In a module block inside handlers.rs:
//! mod generated {
//!     use super::*;
//!     // For entities with site filtering:
//!     crate::crud_get_all_handler!(Daemon);
//!     // Other CRUD operations:
//!     crate::crud_get_by_id_handler!(Port);
//!     crate::crud_create_handler!(Port);
//!     crate::crud_update_handler!(Port);
//!     crate::crud_delete_handler!(Port);
//!     crate::crud_bulk_delete_handler!(Port);
//! }
//!
//! // Then in create_router():
//! OpenApiRouter::new()
//!     .routes(routes!(generated::get_all, generated::create))
//!     .routes(routes!(generated::get_by_id, generated::update, generated::delete))
//!     .routes(routes!(generated::bulk_delete))
//! ```
//!
//! **Note:** These macros use `crate::` paths for utoipa body types instead of `$crate::`
//! because utoipa's proc macro cannot resolve `$crate::` tokens. This means these macros
//! can only be used within this crate, not from external crates.

/// One byte of an entity name in the form spec text uses mid-sentence: each word's first letter
/// is lowercased unless the word is an acronym (its second letter is uppercase). "Daemon API Keys"
/// becomes "daemon API keys", "IP Addresses" becomes "IP addresses", "VLANs" stays.
const fn api_text_byte(name: &[u8], i: usize) -> u8 {
    let byte = name[i];
    let starts_word = i == 0 || name[i - 1] == b' ';
    let acronym = i + 1 < name.len() && name[i + 1].is_ascii_uppercase();
    if starts_word && !acronym {
        byte.to_ascii_lowercase()
    } else {
        byte
    }
}

/// [`api_text`] at compile time, for the CRUD macros. `N` is the name's length.
pub const fn api_text_bytes<const N: usize>(name: &str) -> [u8; N] {
    let bytes = name.as_bytes();
    let mut out = [0u8; N];
    let mut i = 0;
    while i < N {
        out[i] = api_text_byte(bytes, i);
        i += 1;
    }
    out
}

/// An entity name as spec summaries write it mid-sentence. See [`api_text_byte`].
pub fn api_text(name: &str) -> String {
    (0..name.len())
        .map(|i| api_text_byte(name.as_bytes(), i) as char)
        .collect()
}

/// [`api_text`] as a `&'static str` constant, from a constant entity name.
#[macro_export]
macro_rules! api_text_name {
    ($name:expr) => {{
        const __NAME: &str = $name;
        const __BYTES: [u8; __NAME.len()] =
            $crate::server::shared::handlers::openapi_macros::api_text_bytes(__NAME);
        match ::core::str::from_utf8(&__BYTES) {
            Ok(text) => text,
            Err(_) => panic!("entity names are ASCII"),
        }
    }};
}

/// Generates an OpenAPI-annotated get-by-id handler that delegates to `get_by_id_handler::<T>`
#[macro_export]
macro_rules! crud_get_by_id_handler {
    ($entity:ty) => {
        const __GET_BY_ID_TAG: &str = <$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_PLURAL;
        const __GET_BY_ID_OP_ID: &str = const_format::concatcp!("get_", <$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_SINGULAR, "_by_id");
        const __GET_BY_ID_NAME: &str = $crate::api_text_name!(<$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_SINGULAR);
        const __GET_BY_ID_SUMMARY: &str = const_format::concatcp!("Get ", __GET_BY_ID_NAME, " by ID");
        const __GET_BY_ID_DESC_FOUND: &str = const_format::concatcp!(<$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_SINGULAR, " found");
        const __GET_BY_ID_DESC_NOT_FOUND: &str = const_format::concatcp!(<$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_SINGULAR, " not found");
        const __GET_BY_ID_PARAM_DESC: &str = const_format::concatcp!(<$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_SINGULAR, " ID");

        #[utoipa::path(
            get,
            path = "/{id}",
            tag = __GET_BY_ID_TAG,
            operation_id = __GET_BY_ID_OP_ID,
            summary = __GET_BY_ID_SUMMARY,
            params(("id" = uuid::Uuid, Path, description = __GET_BY_ID_PARAM_DESC)),
            responses(
                (status = 200, description = __GET_BY_ID_DESC_FOUND, body = $crate::server::shared::types::api::ApiResponse<$entity>),
                (status = 404, description = __GET_BY_ID_DESC_NOT_FOUND, body = $crate::server::shared::types::api::ApiErrorResponse),
            ),
            security(("user_api_key" = []), ("session" = []))
        )]
        pub async fn get_by_id(
            state: axum::extract::State<std::sync::Arc<$crate::server::config::AppState>>,
            auth: $crate::server::auth::middleware::permissions::Authorized<$crate::server::auth::middleware::permissions::Can<$entity, $crate::server::auth::middleware::permissions::Read>>,
            path: axum::extract::Path<uuid::Uuid>,
        ) -> $crate::server::shared::types::api::ApiResult<
            axum::response::Json<$crate::server::shared::types::api::ApiResponse<$entity>>,
        > {
            $crate::server::shared::handlers::traits::get_by_id_handler::<$entity>(state, auth, path)
                .await
        }
    };
}

/// Generates an OpenAPI-annotated create handler that delegates to `create_handler::<T>`
#[macro_export]
macro_rules! crud_create_handler {
    ($entity:ty) => {
        const __CREATE_TAG: &str = <$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_PLURAL;
        const __CREATE_OP_ID: &str = const_format::concatcp!("create_", <$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_SINGULAR);
        const __CREATE_NAME: &str = $crate::api_text_name!(<$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_SINGULAR);
        const __CREATE_SUMMARY: &str = const_format::concatcp!("Create ", __CREATE_NAME);
        const __CREATE_DESC_CREATED: &str = const_format::concatcp!(<$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_SINGULAR, " created");

        #[utoipa::path(
            post,
            path = "",
            tag = __CREATE_TAG,
            operation_id = __CREATE_OP_ID,
            summary = __CREATE_SUMMARY,
            request_body = $entity,
            responses(
                (status = 200, description = __CREATE_DESC_CREATED, body = $crate::server::shared::types::api::ApiResponse<$entity>),
                (status = 400, description = "Invalid request", body = $crate::server::shared::types::api::ApiErrorResponse),
            ),
            security(("user_api_key" = []), ("session" = []))
        )]
        pub async fn create(
            state: axum::extract::State<std::sync::Arc<$crate::server::config::AppState>>,
            auth: $crate::server::auth::middleware::permissions::Authorized<$crate::server::auth::middleware::permissions::Can<$entity, $crate::server::auth::middleware::permissions::Write>>,
            body: $crate::server::shared::types::api::ApiJson<$entity>,
        ) -> $crate::server::shared::types::api::ApiResult<
            axum::response::Json<$crate::server::shared::types::api::ApiResponse<$entity>>,
        > {
            $crate::server::shared::handlers::traits::create_handler::<$entity>(state, auth, body)
                .await
        }
    };
}

/// Generates an OpenAPI-annotated update handler that delegates to `update_handler::<T>`
#[macro_export]
macro_rules! crud_update_handler {
    ($entity:ty) => {
        const __UPDATE_TAG: &str = <$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_PLURAL;
        const __UPDATE_OP_ID: &str = const_format::concatcp!("update_", <$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_SINGULAR);
        const __UPDATE_NAME: &str = $crate::api_text_name!(<$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_SINGULAR);
        const __UPDATE_SUMMARY: &str = const_format::concatcp!("Update ", __UPDATE_NAME);
        const __UPDATE_DESC_UPDATED: &str = const_format::concatcp!(<$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_SINGULAR, " updated");
        const __UPDATE_DESC_NOT_FOUND: &str = const_format::concatcp!(<$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_SINGULAR, " not found");
        const __UPDATE_PARAM_DESC: &str = const_format::concatcp!(<$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_SINGULAR, " ID");

        #[utoipa::path(
            put,
            path = "/{id}",
            tag = __UPDATE_TAG,
            operation_id = __UPDATE_OP_ID,
            summary = __UPDATE_SUMMARY,
            params(("id" = uuid::Uuid, Path, description = __UPDATE_PARAM_DESC)),
            request_body = $entity,
            responses(
                (status = 200, description = __UPDATE_DESC_UPDATED, body = $crate::server::shared::types::api::ApiResponse<$entity>),
                (status = 404, description = __UPDATE_DESC_NOT_FOUND, body = $crate::server::shared::types::api::ApiErrorResponse),
            ),
            security(("user_api_key" = []), ("session" = []))
        )]
        pub async fn update(
            state: axum::extract::State<std::sync::Arc<$crate::server::config::AppState>>,
            auth: $crate::server::auth::middleware::permissions::Authorized<$crate::server::auth::middleware::permissions::Can<$entity, $crate::server::auth::middleware::permissions::Write>>,
            path: axum::extract::Path<uuid::Uuid>,
            body: $crate::server::shared::types::api::ApiJson<$entity>,
        ) -> $crate::server::shared::types::api::ApiResult<
            axum::response::Json<$crate::server::shared::types::api::ApiResponse<$entity>>,
        > {
            $crate::server::shared::handlers::traits::update_handler::<$entity>(
                state, auth, path, body,
            )
            .await
        }
    };
}

/// Generates an OpenAPI-annotated delete handler that delegates to `delete_handler::<T>`
#[macro_export]
macro_rules! crud_delete_handler {
    ($entity:ty) => {
        const __DELETE_TAG: &str = <$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_PLURAL;
        const __DELETE_OP_ID: &str = const_format::concatcp!("delete_", <$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_SINGULAR);
        const __DELETE_NAME: &str = $crate::api_text_name!(<$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_SINGULAR);
        const __DELETE_SUMMARY: &str = const_format::concatcp!("Delete ", __DELETE_NAME);
        const __DELETE_DESC_DELETED: &str = const_format::concatcp!(<$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_SINGULAR, " deleted");
        const __DELETE_DESC_NOT_FOUND: &str = const_format::concatcp!(<$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_SINGULAR, " not found");
        const __DELETE_PARAM_DESC: &str = const_format::concatcp!(<$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_SINGULAR, " ID");

        #[utoipa::path(
            delete,
            path = "/{id}",
            tag = __DELETE_TAG,
            operation_id = __DELETE_OP_ID,
            summary = __DELETE_SUMMARY,
            params(("id" = uuid::Uuid, Path, description = __DELETE_PARAM_DESC)),
            responses(
                (status = 200, description = __DELETE_DESC_DELETED, body = $crate::server::shared::types::api::EmptyApiResponse),
                (status = 404, description = __DELETE_DESC_NOT_FOUND, body = $crate::server::shared::types::api::ApiErrorResponse),
            ),
            security(("user_api_key" = []), ("session" = []))
        )]
        pub async fn delete(
            state: axum::extract::State<std::sync::Arc<$crate::server::config::AppState>>,
            auth: $crate::server::auth::middleware::permissions::Authorized<$crate::server::auth::middleware::permissions::Can<$entity, $crate::server::auth::middleware::permissions::Delete>>,
            path: axum::extract::Path<uuid::Uuid>,
        ) -> $crate::server::shared::types::api::ApiResult<
            axum::response::Json<$crate::server::shared::types::api::ApiResponse<()>>,
        > {
            $crate::server::shared::handlers::traits::delete_handler::<$entity>(state, auth, path)
                .await
        }
    };
}

/// Generates an OpenAPI-annotated bulk delete handler that delegates to `bulk_delete_handler::<T>`
#[macro_export]
macro_rules! crud_bulk_delete_handler {
    ($entity:ty) => {
        const __BULK_DELETE_TAG: &str = <$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_PLURAL;
        const __BULK_DELETE_OP_ID: &str = const_format::concatcp!("bulk_delete_", <$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_PLURAL);
        const __BULK_DELETE_NAME: &str = $crate::api_text_name!(<$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_PLURAL);
        const __BULK_DELETE_SUMMARY: &str = const_format::concatcp!("Bulk delete ", __BULK_DELETE_NAME);
        const __BULK_DELETE_DESC_DELETED: &str = const_format::concatcp!(<$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_PLURAL, " deleted");
        const __BULK_DELETE_BODY_DESC: &str = const_format::concatcp!("Array of ", <$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_SINGULAR, " IDs to delete");

        #[utoipa::path(
            post,
            path = "/bulk-delete",
            tag = __BULK_DELETE_TAG,
            operation_id = __BULK_DELETE_OP_ID,
            summary = __BULK_DELETE_SUMMARY,
            request_body(content = Vec<uuid::Uuid>, description = __BULK_DELETE_BODY_DESC),
            responses(
                (status = 200, description = __BULK_DELETE_DESC_DELETED, body = $crate::server::shared::types::api::ApiResponse<$crate::server::shared::handlers::traits::BulkDeleteResponse>),
            ),
            security(("user_api_key" = []), ("session" = []))
        )]
        pub async fn bulk_delete(
            state: axum::extract::State<std::sync::Arc<$crate::server::config::AppState>>,
            auth: $crate::server::auth::middleware::permissions::Authorized<$crate::server::auth::middleware::permissions::Can<$entity, $crate::server::auth::middleware::permissions::Delete>>,
            body: $crate::server::shared::types::api::ApiJson<Vec<uuid::Uuid>>,
        ) -> $crate::server::shared::types::api::ApiResult<
            axum::response::Json<
                $crate::server::shared::types::api::ApiResponse<
                    $crate::server::shared::handlers::traits::BulkDeleteResponse,
                >,
            >,
        > {
            $crate::server::shared::handlers::traits::bulk_delete_handler::<$entity>(
                state, auth, body,
            )
            .await
        }
    };
}

/// Generates an OpenAPI-annotated CSV export handler that delegates to `export_csv_handler::<T>`
///
/// # Example
/// ```ignore
/// crud_export_csv_handler!(Host);
/// // For an entity whose export only the UI calls:
/// crud_export_csv_handler!(UserApiKey, internal);
/// ```
#[macro_export]
macro_rules! crud_export_csv_handler {
    (@impl $entity:ty, [$($tag:ident),+]) => {
        // Type alias for filter query
        type __ExportCsvFilterQuery = <$entity as $crate::server::shared::handlers::traits::CrudHandlers>::FilterQuery;

        const __EXPORT_CSV_TAG: &str = <$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_PLURAL;
        const __EXPORT_CSV_OP_ID: &str = const_format::concatcp!("export_", <$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_PLURAL, "_csv");
        const __EXPORT_CSV_NAME: &str = $crate::api_text_name!(<$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_PLURAL);
        const __EXPORT_CSV_SUMMARY: &str = const_format::concatcp!("Export ", __EXPORT_CSV_NAME, " to CSV");
        const __EXPORT_CSV_DESC: &str = const_format::concatcp!("Export all ", __EXPORT_CSV_NAME, " matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.");
        const __EXPORT_CSV_RESP_DESC: &str = const_format::concatcp!("CSV file containing ", <$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_PLURAL);

        #[utoipa::path(
            get,
            path = "/export/csv",
            tags = [$($tag),+],
            operation_id = __EXPORT_CSV_OP_ID,
            summary = __EXPORT_CSV_SUMMARY,
            description = __EXPORT_CSV_DESC,
            params(__ExportCsvFilterQuery),
            responses(
                (status = 200, description = __EXPORT_CSV_RESP_DESC, content_type = "text/csv", body = String),
            ),
            security(("user_api_key" = []), ("session" = []))
        )]
        pub async fn export_csv(
            state: axum::extract::State<std::sync::Arc<$crate::server::config::AppState>>,
            auth: $crate::server::auth::middleware::permissions::Authorized<$crate::server::auth::middleware::permissions::Can<$entity, $crate::server::auth::middleware::permissions::Read>>,
            query: $crate::server::shared::extractors::Query<__ExportCsvFilterQuery>,
        ) -> $crate::server::shared::types::api::ApiResult<impl axum::response::IntoResponse> {
            $crate::server::shared::handlers::csv::export_csv_handler::<$entity>(state, auth, query)
                .await
        }
    };
    ($entity:ty) => {
        $crate::crud_export_csv_handler!(@impl $entity, [__EXPORT_CSV_TAG]);
    };
    ($entity:ty, internal) => {
        const __EXPORT_CSV_INTERNAL: &str = $crate::server::openapi::tags::INTERNAL;
        $crate::crud_export_csv_handler!(@impl $entity, [__EXPORT_CSV_TAG, __EXPORT_CSV_INTERNAL]);
    };
}

/// Generates an OpenAPI-annotated get_all handler with query params derived from the filter query type.
/// The filter query type must derive `IntoParams` for utoipa to extract the param documentation.
///
/// # Example
/// ```ignore
/// // For entities with site filtering:
/// crud_get_all_handler!(Daemon);
/// // With custom response type:
/// crud_get_all_handler!(Group, GroupResponse);
/// ```
#[macro_export]
macro_rules! crud_get_all_handler {
    ($entity:ty, $response:ty) => {
        // Type alias for filter query
        type __GetAllFilterQuery = <$entity as $crate::server::shared::handlers::traits::CrudHandlers>::FilterQuery;
        // Type alias for response - used in function signature
        type __PaginatedResponse = $crate::server::shared::types::api::PaginatedApiResponse<$response>;
        // Consts from trait - utoipa accepts const identifiers
        const __GET_ALL_TAG: &str = <$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_PLURAL;
        const __GET_ALL_OP_ID: &str = const_format::concatcp!("list_", <$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_PLURAL);
        const __GET_ALL_NAME: &str = $crate::api_text_name!(<$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_PLURAL);
        const __GET_ALL_SUMMARY: &str = const_format::concatcp!("List ", __GET_ALL_NAME);
        const __GET_ALL_RESP_DESC: &str = const_format::concatcp!("List of ", <$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_PLURAL);

        #[utoipa::path(
            get,
            path = "",
            tag = __GET_ALL_TAG,
            operation_id = __GET_ALL_OP_ID,
            summary = __GET_ALL_SUMMARY,
            params(__GetAllFilterQuery),
            responses(
                // Use inline() to force utoipa 5.0 to generate unique schema for each generic instantiation
                (status = 200, description = __GET_ALL_RESP_DESC, body = inline(__PaginatedResponse)),
            ),
            security(("user_api_key" = []), ("session" = []))
        )]
        pub async fn get_all(
            state: axum::extract::State<std::sync::Arc<$crate::server::config::AppState>>,
            auth: $crate::server::auth::middleware::permissions::Authorized<$crate::server::auth::middleware::permissions::Can<$entity, $crate::server::auth::middleware::permissions::Read>>,
            query: $crate::server::shared::extractors::Query<__GetAllFilterQuery>,
        ) -> $crate::server::shared::types::api::ApiResult<
            axum::response::Json<__PaginatedResponse>,
        > {
            $crate::server::shared::handlers::traits::get_all_handler::<$entity>(state, auth, query)
                .await
        }
    };
    // Version where response = entity
    ($entity:ty) => {
        $crate::crud_get_all_handler!($entity, $entity);
    };
}

/// Generates an OpenAPI-annotated list handler that delegates to `list_page::<T>`: one page of
/// the entity's list, ordered, grouped and counted on the server. The route, tag and operation id
/// are those `crud_get_all_handler!` gives, so moving an entity onto it keeps its API.
#[macro_export]
macro_rules! crud_list_page_handler {
    ($entity:ty) => {
        type __ListFilterQuery = <$entity as $crate::server::shared::handlers::traits::CrudHandlers>::FilterQuery;
        type __ListPageResponse = $crate::server::shared::types::api::PaginatedApiResponse<
            <$entity as $crate::server::shared::handlers::list::ListRows>::Item,
        >;
        const __LIST_TAG: &str = <$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_PLURAL;
        const __LIST_OP_ID: &str = const_format::concatcp!("list_", <$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_PLURAL);
        const __LIST_NAME: &str = $crate::api_text_name!(<$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_PLURAL);
        const __LIST_SUMMARY: &str = const_format::concatcp!("List ", __LIST_NAME);
        const __LIST_RESP_DESC: &str = const_format::concatcp!("List of ", <$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_PLURAL);

        #[utoipa::path(
            get,
            path = "",
            tag = __LIST_TAG,
            operation_id = __LIST_OP_ID,
            summary = __LIST_SUMMARY,
            params(__ListFilterQuery),
            responses(
                (status = 200, description = __LIST_RESP_DESC, body = inline(__ListPageResponse)),
            ),
            security(("user_api_key" = []), ("session" = []))
        )]
        pub async fn get_all(
            axum::extract::State(state): axum::extract::State<std::sync::Arc<$crate::server::config::AppState>>,
            auth: $crate::server::auth::middleware::permissions::Authorized<$crate::server::auth::middleware::permissions::Can<$entity, $crate::server::auth::middleware::permissions::Read>>,
            $crate::server::shared::extractors::Query(query): $crate::server::shared::extractors::Query<__ListFilterQuery>,
        ) -> $crate::server::shared::types::api::ApiResult<axum::response::Json<__ListPageResponse>> {
            let scope = $crate::server::shared::handlers::traits::ListScope::of(&auth)?;
            Ok(axum::response::Json(
                $crate::server::shared::handlers::list::list_page::<$entity>(&state, &scope, &query)
                    .await?,
            ))
        }
    };
}

/// Generates an OpenAPI-annotated field-values handler that delegates to
/// `get_field_values_handler::<T>`: every distinct value of one of the entity's order fields,
/// with its row count, under the same filters the list takes.
///
/// # Example
/// ```ignore
/// crud_get_field_values_handler!(Host);
/// ```
#[macro_export]
macro_rules! crud_get_field_values_handler {
    ($entity:ty) => {
        type __FieldValuesFilterQuery = <$entity as $crate::server::shared::handlers::traits::CrudHandlers>::FilterQuery;
        type __FieldValuesOrderField = <$entity as $crate::server::shared::handlers::traits::CrudHandlers>::OrderField;
        type __FieldValuesResponse = $crate::server::shared::types::api::ApiResponse<Vec<$crate::server::shared::types::api::GroupCount>>;

        const __FIELD_VALUES_TAG: &str = <$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_PLURAL;
        const __FIELD_VALUES_OP_ID: &str = const_format::concatcp!("get_", <$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_SINGULAR, "_field_values");
        const __FIELD_VALUES_NAME: &str = $crate::api_text_name!(<$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_SINGULAR);
        const __FIELD_VALUES_INTERNAL: &str = $crate::server::openapi::tags::INTERNAL;
        const __FIELD_VALUES_SUMMARY: &str = const_format::concatcp!("List ", __FIELD_VALUES_NAME, " field values");
        const __FIELD_VALUES_DESC: &str = const_format::concatcp!("Every distinct value of one ", <$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_SINGULAR, " field across the ", <$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_PLURAL, " the caller can list, with how many hold each. Takes the list's filters; ignores pagination, search and ordering.");
        const __FIELD_VALUES_RESP_DESC: &str = const_format::concatcp!("Distinct values and their ", <$entity as $crate::server::shared::storage::traits::Entity>::ENTITY_NAME_SINGULAR, " counts");

        #[utoipa::path(
            get,
            path = "/field-values/{field}",
            tags = [__FIELD_VALUES_TAG, __FIELD_VALUES_INTERNAL],
            operation_id = __FIELD_VALUES_OP_ID,
            summary = __FIELD_VALUES_SUMMARY,
            description = __FIELD_VALUES_DESC,
            params(
                ("field" = __FieldValuesOrderField, Path, description = "The field whose values to count"),
                __FieldValuesFilterQuery
            ),
            responses(
                (status = 200, description = __FIELD_VALUES_RESP_DESC, body = $crate::server::shared::types::api::ApiResponse<Vec<$crate::server::shared::types::api::GroupCount>>),
                (status = 400, description = "Unknown field", body = $crate::server::shared::types::api::ApiErrorResponse),
            ),
            security(("user_api_key" = []), ("session" = []))
        )]
        pub async fn get_field_values(
            state: axum::extract::State<std::sync::Arc<$crate::server::config::AppState>>,
            auth: $crate::server::auth::middleware::permissions::Authorized<$crate::server::auth::middleware::permissions::Can<$entity, $crate::server::auth::middleware::permissions::Read>>,
            path: axum::extract::Path<__FieldValuesOrderField>,
            query: $crate::server::shared::extractors::Query<__FieldValuesFilterQuery>,
        ) -> $crate::server::shared::types::api::ApiResult<
            axum::response::Json<__FieldValuesResponse>,
        > {
            $crate::server::shared::handlers::traits::get_field_values_handler::<$entity>(
                state, auth, path, query,
            )
            .await
        }
    };
}
