//! The global search: one query, answered by every entity type's service side by side.
//!
//! Each type's filter is the caller's scope narrowed by the query ([`SearchQuery::narrow`], which
//! applies the type's search predicates and tags). It is then read through the service method the
//! type's list endpoint uses, so a match comes back in the shape that list returns.

use std::fmt::Display;

use futures::future::{BoxFuture, FutureExt, try_join_all};
use strum::IntoEnumIterator;
use uuid::Uuid;

use crate::server::{
    daemons::r#impl::{api::DaemonResponse, base::Daemon, version::DaemonVersionPolicy},
    hosts::r#impl::base::Host,
    shared::{
        entities::{ChangeTriggersTopologyStaleness, Entity, EntityDiscriminants},
        handlers::{ordering::apply_ordering, traits::CrudHandlers},
        services::{factory::ServiceFactory, traits::CrudService},
        storage::{filter::StorableFilter, traits::PaginatedResult},
    },
    subnets::r#impl::base::Subnet,
    user_api_keys::r#impl::base::UserApiKey,
    users::r#impl::{base::User, permissions::UserOrgPermissions},
};

use super::{
    scope::{SearchQuery, SearchScope},
    types::{GlobalSearchGroup, GlobalSearchResponse, SearchHit},
};

/// Matches on the first page of each entity type, enough to show every type at once.
pub const GLOBAL_SEARCH_FIRST_PAGE: u32 = 5;
/// The most matches one request returns for one type.
pub const GLOBAL_SEARCH_MAX_PAGE: u32 = 50;

type Page = PaginatedResult<SearchHit>;

/// Matches for `query`, grouped by type in registry order: a page of every type, or of `only`.
pub async fn global_search(
    services: &ServiceFactory,
    scope: &SearchScope,
    query: &SearchQuery,
    only: Option<EntityDiscriminants>,
) -> anyhow::Result<GlobalSearchResponse> {
    if query.is_empty() {
        return Ok(GlobalSearchResponse::default());
    }
    let searches = EntityDiscriminants::iter()
        .filter(|entity_type| only.is_none_or(|only| only == *entity_type))
        .map(|entity_type| search_type(services, entity_type, scope, query));
    let groups = try_join_all(searches)
        .await?
        .into_iter()
        .filter(|group| group.total_count > 0)
        .collect();
    Ok(GlobalSearchResponse { groups })
}

/// One page of an entity type's matches, with how many there are in all. A type takes part once its storage defines search predicates;
/// until then [`SearchQuery::narrow`] leaves it nothing to read.
fn search_type<'a>(
    services: &'a ServiceFactory,
    entity_type: EntityDiscriminants,
    scope: &'a SearchScope,
    query: &'a SearchQuery,
) -> BoxFuture<'a, anyhow::Result<GlobalSearchGroup>> {
    use EntityDiscriminants as E;
    let s = services;
    let items = match entity_type {
        // The caller's own organization is their scope, not something to find.
        E::Organization => async { Ok(empty()) }.boxed(),
        E::Invite => listed(&*s.invite_service, scope, query),
        E::Share => listed(&*s.share_service, scope, query),
        E::Site => listed(&*s.site_service, scope, query),
        E::DaemonApiKey => listed(&*s.daemon_api_key_service, scope, query),
        E::UserApiKey => user_api_keys(s, scope, query).boxed(),
        E::User => users(s, scope, query).boxed(),
        E::Tag => listed(&*s.tag_service, scope, query),
        E::Discovery => listed(&*s.discovery_service, scope, query),
        E::Daemon => daemons(s, scope, query).boxed(),
        E::Host => hosts(s, scope, query).boxed(),
        E::Service => listed(&*s.service_service, scope, query),
        E::Port => listed(&*s.port_service, scope, query),
        E::Binding => listed(&*s.binding_service, scope, query),
        E::IPAddress => listed(&*s.ip_address_service, scope, query),
        E::Interface => listed(&*s.interface_service, scope, query),
        E::Credential => listed(&*s.credential_service, scope, query),
        E::Subnet => subnets(s, scope, query).boxed(),
        E::Vlan => listed(&*s.vlan_service, scope, query),
        E::Dependency => listed(&*s.dependency_service, scope, query),
        E::Topology => listed(&*s.topology_service, scope, query),
        E::Snapshot => listed(&*s.snapshot_service, scope, query),
    };
    async move {
        let page = items.await?;
        Ok(GlobalSearchGroup {
            entity_type,
            total_count: page.total_count,
            items: page.items,
        })
    }
    .boxed()
}

/// `T`'s filter for this caller and query, paged and with the ORDER BY its list sorts by name,
/// or `None` when `T` can't match.
fn search_filter<T>(
    base: StorableFilter<T>,
    query: &SearchQuery,
) -> Option<(StorableFilter<T>, String)>
where
    T: CrudHandlers + Display + ChangeTriggersTopologyStaleness<T> + Default,
    Entity: From<T>,
{
    let filter = query.narrow(base)?;
    let default_order = format!("{}.created_at ASC", T::table_name());
    let (filter, order) = apply_ordering(None, T::search_order(), None, filter, &default_order);
    Some((filter.limit(query.limit).offset(query.offset), order))
}

/// Matches of an entity whose list is the generic one: scoped to the caller's sites or
/// organization and read with `get_paginated_ordered`.
fn listed<'a, T, S>(
    service: &'a S,
    scope: &'a SearchScope,
    query: &'a SearchQuery,
) -> BoxFuture<'a, anyhow::Result<Page>>
where
    T: CrudHandlers + Display + ChangeTriggersTopologyStaleness<T> + Default + 'static,
    Entity: From<T>,
    S: CrudService<T> + Sync,
{
    async move {
        let base = StorableFilter::<T>::new_for_access(&scope.site_ids, &scope.organization_id);
        let Some((filter, order)) = search_filter(base, query) else {
            return Ok(empty());
        };
        let found = service.get_paginated_ordered(filter, &order).await?;
        Ok(Page {
            items: found
                .items
                .into_iter()
                .map(|item| SearchHit::try_from(Entity::from(item)))
                .collect::<anyhow::Result<_>>()?,
            total_count: found.total_count,
        })
    }
    .boxed()
}

fn empty() -> Page {
    Page {
        items: Vec::new(),
        total_count: 0,
    }
}

/// Hosts as the host list returns them: with their addresses, which their titles can fall back
/// to, and without the other children.
async fn hosts(
    s: &ServiceFactory,
    scope: &SearchScope,
    query: &SearchQuery,
) -> anyhow::Result<Page> {
    let base = StorableFilter::<Host>::new_from_site_ids(&scope.site_ids);
    let Some((filter, order)) = search_filter(base, query) else {
        return Ok(empty());
    };
    let found = s
        .host_service
        .get_all_host_responses_paginated(filter, &order, None, false)
        .await?;
    Ok(Page {
        items: found.items.into_iter().map(SearchHit::Host).collect(),
        total_count: found.total_count,
    })
}

/// Daemons as the daemon list returns them: with their version status and interfaced subnets.
async fn daemons(
    s: &ServiceFactory,
    scope: &SearchScope,
    query: &SearchQuery,
) -> anyhow::Result<Page> {
    let base = StorableFilter::<Daemon>::new_from_site_ids(&scope.site_ids);
    let Some((filter, order)) = search_filter(base, query) else {
        return Ok(empty());
    };
    let found = s
        .daemon_service
        .get_paginated_ordered(filter, &order)
        .await?;
    let ids: Vec<Uuid> = found.items.iter().map(|d| d.id).collect();
    let subnet_ids = s.daemon_service.get_interfaced_subnet_ids_batch(&ids).await;
    let policy = DaemonVersionPolicy::default();
    Ok(Page {
        items: found
            .items
            .into_iter()
            .map(|d| {
                SearchHit::Daemon(DaemonResponse {
                    id: d.id,
                    created_at: d.created_at,
                    updated_at: d.updated_at,
                    version_status: policy.evaluate(d.base.version.as_ref()),
                    interfaced_subnet_ids: subnet_ids.get(&d.id).cloned().unwrap_or_default(),
                    base: d.base,
                })
            })
            .collect(),
        total_count: found.total_count,
    })
}

/// Subnets as the subnet list returns them: with their usage.
async fn subnets(
    s: &ServiceFactory,
    scope: &SearchScope,
    query: &SearchQuery,
) -> anyhow::Result<Page> {
    let base = StorableFilter::<Subnet>::new_from_site_ids(&scope.site_ids);
    let Some((filter, order)) = search_filter(base, query) else {
        return Ok(empty());
    };
    let found = s
        .subnet_service
        .get_paginated_ordered(filter, &order)
        .await?;
    let subnets = s.subnet_service.with_usage(found.items, None).await?;
    Ok(Page {
        items: subnets.into_iter().map(SearchHit::Subnet).collect(),
        total_count: found.total_count,
    })
}

/// Users as the users list shows them: to admins and owners only, and only the users that list
/// shows the caller. Filtered after the read, as the list is, so the page and the total count
/// what is shown.
async fn users(
    s: &ServiceFactory,
    scope: &SearchScope,
    query: &SearchQuery,
) -> anyhow::Result<Page> {
    if scope.permissions < UserOrgPermissions::Admin {
        return Ok(empty());
    }
    let base = StorableFilter::<User>::new_from_org_id(&scope.organization_id);
    let Some(filter) = query.narrow(base) else {
        return Ok(empty());
    };
    let mut found = s.user_service.get_all(filter).await?;
    found.sort_by(|a, b| a.base.email.as_str().cmp(b.base.email.as_str()));
    let listed: Vec<User> = found
        .into_iter()
        .filter(|user| user.is_listed_for(scope.permissions, scope.user_id))
        .collect();
    Ok(Page {
        total_count: listed.len() as u64,
        items: listed
            .into_iter()
            .skip(query.offset as usize)
            .take(query.limit as usize)
            .map(SearchHit::User)
            .collect(),
    })
}

/// User API keys as their list shows them: only the signed-in user's own. An API key caller
/// lists none, so it finds none.
async fn user_api_keys(
    s: &ServiceFactory,
    scope: &SearchScope,
    query: &SearchQuery,
) -> anyhow::Result<Page> {
    let Some(user_id) = scope.session_user_id else {
        return Ok(empty());
    };
    let base = StorableFilter::<UserApiKey>::new_from_user_id(&user_id);
    let Some((filter, order)) = search_filter(base, query) else {
        return Ok(empty());
    };
    let found = s
        .user_api_key_service
        .get_paginated_ordered(filter, &order)
        .await?;
    Ok(Page {
        items: found.items.into_iter().map(SearchHit::UserApiKey).collect(),
        total_count: found.total_count,
    })
}
