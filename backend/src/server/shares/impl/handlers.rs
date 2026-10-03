use crate::server::{
    config::AppState,
    shared::handlers::{query::SharesQuery, traits::CrudHandlers},
    shares::{r#impl::base::Share, service::ShareService},
};

impl CrudHandlers for Share {
    type Service = ShareService;
    type FilterQuery = SharesQuery;
    type OrderField = crate::server::shared::handlers::ordering::NoOrderField;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.share_service
    }
}
