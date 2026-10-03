use crate::server::{
    config::AppState,
    daemons::{handlers::DaemonFilterQuery, r#impl::base::Daemon, service::DaemonService},
    shared::handlers::traits::CrudHandlers,
};

impl CrudHandlers for Daemon {
    type Service = DaemonService;
    type FilterQuery = DaemonFilterQuery;
    type OrderField = crate::server::daemons::handlers::DaemonOrderField;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.daemon_service
    }
}
