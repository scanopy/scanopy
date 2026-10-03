use crate::server::{
    config::AppState,
    dependencies::{
        handlers::DependencyFilterQuery, r#impl::base::Dependency, service::DependencyService,
    },
    shared::handlers::traits::CrudHandlers,
};

impl CrudHandlers for Dependency {
    type Service = DependencyService;
    type FilterQuery = DependencyFilterQuery;
    type OrderField = crate::server::dependencies::handlers::DependencyOrderField;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.dependency_service
    }
}
