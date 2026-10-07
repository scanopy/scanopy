//! The global search: every entity type's service asked the same query, side by side.

use std::fmt::Display;

use futures::future::{BoxFuture, FutureExt, try_join_all};
use strum::IntoEnumIterator;

use crate::server::shared::{
    entities::{ChangeTriggersTopologyStaleness, Entity, EntityDiscriminants},
    handlers::traits::CrudHandlers,
    services::{
        factory::ServiceFactory,
        search::{SearchQuery, SearchScope},
        traits::CrudService,
    },
};

use super::types::{GlobalSearchGroup, GlobalSearchResponse};

/// Matches returned per entity type. The palette jumps to a match; browsing belongs to the
/// entity's own list.
pub const GLOBAL_SEARCH_LIMIT: u32 = 5;

/// Every entity type's matches for `query`, grouped by type in registry order.
///
/// Each type is searched by its own service ([`CrudService::search`]), which applies that type's
/// list scope. A type takes part once its storage defines search predicates; until then its
/// service answers with nothing.
pub async fn global_search(
    services: &ServiceFactory,
    scope: &SearchScope,
    query: &SearchQuery,
) -> anyhow::Result<GlobalSearchResponse> {
    if query.is_empty() {
        return Ok(GlobalSearchResponse::default());
    }
    let searches = EntityDiscriminants::iter()
        .map(|entity_type| search_type(services, entity_type, scope, query));
    let groups = try_join_all(searches)
        .await?
        .into_iter()
        .filter(|group| !group.items.is_empty())
        .collect();
    Ok(GlobalSearchResponse { groups })
}

/// One entity type's matches, from that type's service.
fn search_type<'a>(
    services: &'a ServiceFactory,
    entity_type: EntityDiscriminants,
    scope: &'a SearchScope,
    query: &'a SearchQuery,
) -> BoxFuture<'a, anyhow::Result<GlobalSearchGroup>> {
    let s = services;
    let items = match entity_type {
        EntityDiscriminants::Organization => search_in(&*s.organization_service, scope, query),
        EntityDiscriminants::Invite => search_in(&*s.invite_service, scope, query),
        EntityDiscriminants::Share => search_in(&*s.share_service, scope, query),
        EntityDiscriminants::Site => search_in(&*s.site_service, scope, query),
        EntityDiscriminants::DaemonApiKey => search_in(&*s.daemon_api_key_service, scope, query),
        EntityDiscriminants::UserApiKey => search_in(&*s.user_api_key_service, scope, query),
        EntityDiscriminants::User => search_in(&*s.user_service, scope, query),
        EntityDiscriminants::Tag => search_in(&*s.tag_service, scope, query),
        EntityDiscriminants::Discovery => search_in(&*s.discovery_service, scope, query),
        EntityDiscriminants::Daemon => search_in(&*s.daemon_service, scope, query),
        EntityDiscriminants::Host => search_in(&*s.host_service, scope, query),
        EntityDiscriminants::Service => search_in(&*s.service_service, scope, query),
        EntityDiscriminants::Port => search_in(&*s.port_service, scope, query),
        EntityDiscriminants::Binding => search_in(&*s.binding_service, scope, query),
        EntityDiscriminants::IPAddress => search_in(&*s.ip_address_service, scope, query),
        EntityDiscriminants::Interface => search_in(&*s.interface_service, scope, query),
        EntityDiscriminants::Credential => search_in(&*s.credential_service, scope, query),
        EntityDiscriminants::Subnet => search_in(&*s.subnet_service, scope, query),
        EntityDiscriminants::Vlan => search_in(&*s.vlan_service, scope, query),
        EntityDiscriminants::Dependency => search_in(&*s.dependency_service, scope, query),
        EntityDiscriminants::Topology => search_in(&*s.topology_service, scope, query),
        EntityDiscriminants::Snapshot => search_in(&*s.snapshot_service, scope, query),
        EntityDiscriminants::Unknown => async { Ok(Vec::new()) }.boxed(),
    };
    async move {
        Ok(GlobalSearchGroup {
            entity_type,
            items: items.await?,
        })
    }
    .boxed()
}

/// `service`'s matches, in the order `T`'s list sorts by name, as entities.
fn search_in<'a, T, S>(
    service: &'a S,
    scope: &'a SearchScope,
    query: &'a SearchQuery,
) -> BoxFuture<'a, anyhow::Result<Vec<Entity>>>
where
    T: CrudHandlers + Display + ChangeTriggersTopologyStaleness<T> + Default + 'static,
    Entity: From<T>,
    S: CrudService<T> + Sync,
{
    async move {
        let hits = service.search(scope, query, T::search_order()).await?;
        Ok(hits.into_iter().map(Entity::from).collect())
    }
    .boxed()
}
