use crate::server::{
    config::AppState,
    hosts::{handlers::HostFilterQuery, r#impl::base::Host, service::HostService},
    shared::handlers::traits::CrudHandlers,
};

impl CrudHandlers for Host {
    type Service = HostService;
    type FilterQuery = HostFilterQuery;
    type OrderField = crate::server::hosts::handlers::HostOrderField;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.host_service
    }

    fn search_order() -> Option<Self::OrderField> {
        Some(Self::OrderField::Name)
    }
}
