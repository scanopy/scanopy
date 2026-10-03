use crate::server::{
    config::AppState,
    invites::{r#impl::base::Invite, service::InviteService},
    shared::handlers::{query::NoFilterQuery, traits::CrudHandlers},
};

impl CrudHandlers for Invite {
    type Service = InviteService;
    type FilterQuery = NoFilterQuery;
    type OrderField = crate::server::shared::handlers::ordering::NoOrderField;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.invite_service
    }
}
