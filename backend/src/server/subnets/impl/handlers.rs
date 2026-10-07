use crate::server::{
    config::AppState,
    shared::handlers::traits::CrudHandlers,
    subnets::{handlers::SubnetFilterQuery, r#impl::base::Subnet, service::SubnetService},
};

impl CrudHandlers for Subnet {
    type Service = SubnetService;
    type FilterQuery = SubnetFilterQuery;
    type OrderField = crate::server::subnets::handlers::SubnetOrderField;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.subnet_service
    }

    fn search_order() -> Option<Self::OrderField> {
        Some(Self::OrderField::Name)
    }
}
