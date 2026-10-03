use crate::server::{
    config::AppState,
    services::{handlers::ServiceFilterQuery, r#impl::base::Service, service::ServiceService},
    shared::handlers::traits::CrudHandlers,
};

impl CrudHandlers for Service {
    type Service = ServiceService;
    type FilterQuery = ServiceFilterQuery;
    type OrderField = crate::server::services::handlers::ServiceOrderField;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.service_service
    }
}
