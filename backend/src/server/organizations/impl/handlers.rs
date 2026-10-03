use crate::server::{
    config::AppState,
    organizations::{r#impl::base::Organization, service::OrganizationService},
    shared::handlers::{query::NoFilterQuery, traits::CrudHandlers},
};

impl CrudHandlers for Organization {
    type Service = OrganizationService;
    type FilterQuery = NoFilterQuery;
    type OrderField = crate::server::shared::handlers::ordering::NoOrderField;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.organization_service
    }
}
