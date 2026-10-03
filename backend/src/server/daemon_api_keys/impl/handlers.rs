use crate::server::{
    config::AppState,
    daemon_api_keys::{r#impl::base::DaemonApiKey, service::DaemonApiKeyService},
    shared::handlers::{query::NetworkFilterQuery, traits::CrudHandlers},
};

impl CrudHandlers for DaemonApiKey {
    type Service = DaemonApiKeyService;
    type FilterQuery = NetworkFilterQuery;
    type OrderField = crate::server::shared::handlers::ordering::NoOrderField;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.daemon_api_key_service
    }
}
