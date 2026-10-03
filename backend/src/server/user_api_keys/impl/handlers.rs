use crate::server::{
    config::AppState,
    shared::handlers::{query::NoFilterQuery, traits::CrudHandlers},
    user_api_keys::{r#impl::base::UserApiKey, service::UserApiKeyService},
};

impl CrudHandlers for UserApiKey {
    type Service = UserApiKeyService;
    // User API keys are filtered by user_id in the custom get_all handler
    type FilterQuery = NoFilterQuery;
    type OrderField = crate::server::shared::handlers::ordering::NoOrderField;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.user_api_key_service
    }
}
