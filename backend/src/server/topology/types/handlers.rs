use crate::server::{
    config::AppState,
    shared::handlers::{query::SiteFilterQuery, traits::CrudHandlers},
    topology::{service::main::TopologyService, types::base::Topology},
};

impl CrudHandlers for Topology {
    type Service = TopologyService;
    type FilterQuery = SiteFilterQuery;
    type OrderField = crate::server::shared::handlers::ordering::NoOrderField;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.topology_service
    }
}
