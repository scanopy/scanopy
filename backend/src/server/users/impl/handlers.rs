use crate::server::{
    config::AppState,
    shared::handlers::{query::NoFilterQuery, traits::CrudHandlers},
    users::{r#impl::base::User, service::UserService},
};

impl CrudHandlers for User {
    type Service = UserService;
    type FilterQuery = NoFilterQuery;
    type OrderField = crate::server::shared::handlers::ordering::NoOrderField;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.user_service
    }
}
