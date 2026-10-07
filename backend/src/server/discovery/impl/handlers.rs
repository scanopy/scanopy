use crate::server::{
    config::AppState,
    discovery::handlers::DiscoveryFilterQuery,
    discovery::{r#impl::base::Discovery, service::DiscoveryService},
    shared::handlers::traits::CrudHandlers,
};

impl CrudHandlers for Discovery {
    type Service = DiscoveryService;
    type FilterQuery = DiscoveryFilterQuery;
    type OrderField = crate::server::discovery::handlers::DiscoveryOrderField;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.discovery_service
    }

    fn search_order() -> Option<Self::OrderField> {
        Some(Self::OrderField::Name)
    }
}
