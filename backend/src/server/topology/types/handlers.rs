use crate::server::{
    config::AppState,
    shared::handlers::{query::NetworkFilterQuery, traits::CrudHandlers},
    topology::{service::main::TopologyService, types::base::Topology},
};

impl CrudHandlers for Topology {
    type Service = TopologyService;
    type FilterQuery = NetworkFilterQuery;
    type OrderField = crate::server::shared::handlers::ordering::NoOrderField;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.topology_service
    }
}
