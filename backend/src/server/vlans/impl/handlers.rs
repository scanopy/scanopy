use crate::server::{
    config::AppState,
    shared::handlers::traits::CrudHandlers,
    vlans::{handlers::VlanFilterQuery, r#impl::base::Vlan, service::VlanService},
};

impl CrudHandlers for Vlan {
    type Service = VlanService;
    type FilterQuery = VlanFilterQuery;
    type OrderField = crate::server::vlans::handlers::VlanOrderField;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.vlan_service
    }

    fn search_order() -> Option<Self::OrderField> {
        Some(Self::OrderField::VlanNumber)
    }
}
