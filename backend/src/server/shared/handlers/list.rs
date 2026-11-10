//! The paginated list behind every entity table: one page of rows in the list's order, with the
//! full size of every group the page cuts.
//!
//! An entity opts in by implementing [`PaginatedList`]; its list handler is then a call to
//! [`list_page`]. Filtering stays with [`CrudHandlers::list_filter`], so the CSV export and the
//! list read the same rows for the same query.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Display;

use async_trait::async_trait;
use serde::Serialize;
use uuid::Uuid;

use crate::server::{
    config::AppState,
    shared::{
        entities::{ChangeTriggersTopologyStaleness, Entity as EntityEnum},
        handlers::{
            ordering::{OrderField, TagGroupOrdering},
            query::{FilterQueryExtractor, OrderDirection, PaginationParams},
            traits::{CrudHandlers, ListScope},
            tree::{TreePosition, tree_order},
        },
        services::traits::CrudService,
        storage::{
            filter::StorableFilter,
            traits::{Entity, Storage},
        },
        types::api::{ApiResult, GroupCount, PaginatedApiResponse},
    },
};

/// How a list request orders its rows: the grouping, the sort inside it, and either one taken
/// from a tag group in their place.
#[derive(Debug, Clone, Copy, Default)]
pub struct ListOrdering<O> {
    pub group_by: Option<O>,
    pub order_by: Option<O>,
    pub direction: Option<OrderDirection>,
    pub tag_group: TagGroupOrdering,
}

impl<O: OrderField> ListOrdering<O> {
    /// The expression the list is grouped on, which is also the key of each entry in
    /// `group_counts`. `None` when the list isn't grouped.
    pub fn group_sql(&self) -> Option<&'static str> {
        match (self.tag_group.group_by, self.group_by) {
            (Some(_), _) => Some(TagGroupOrdering::GROUP_SQL),
            (None, Some(field)) => Some(field.to_sql()),
            (None, None) => None,
        }
    }

    /// The ORDER BY for this ordering, with the JOINs it reads from added to `filter`.
    pub fn apply<T: Entity>(
        &self,
        filter: StorableFilter<T>,
        default_order: &str,
    ) -> (StorableFilter<T>, String) {
        self.tag_group.apply(
            T::entity_type(),
            self.group_by,
            self.order_by,
            self.direction,
            filter,
            default_order,
        )
    }

    /// This ordering without its grouping: the sort alone.
    fn sort_only(&self) -> Self {
        Self {
            group_by: None,
            tag_group: TagGroupOrdering {
                group_by: None,
                order_by: self.tag_group.order_by,
            },
            ..*self
        }
    }
}

/// How a list's rows sort, group and filter, wherever the rows come from: storage, for a
/// [`PaginatedList`], or code, for a list paged by [`computed_page`].
pub trait ListRows {
    /// What one row of the list is sent as.
    type Item: Serialize + Send;
    /// The fields the list sorts and groups by.
    type Field: OrderField;
    /// The request the list is read with.
    type Query;

    /// How `query` orders the list.
    fn ordering(query: &Self::Query) -> ListOrdering<Self::Field>;

    /// Whether only code can derive `field` (a daemon's status, a subnet's utilization), so a list
    /// sorted or grouped by it is ordered here rather than in SQL. Its `to_sql` is then unused.
    fn is_computed(field: &Self::Field) -> bool {
        let _ = field;
        false
    }

    /// The value of a computed `field` for one row. Called only for a field
    /// [`Self::is_computed`] accepts, and for every field of a [`computed_page`] list.
    fn computed(field: &Self::Field, item: &Self::Item) -> Option<Computed> {
        let _ = (field, item);
        None
    }

    /// Whether a row passes the filters only code can apply: for a [`PaginatedList`], those
    /// [`PaginatedList::filters_in_memory`] reports.
    fn keep(query: &Self::Query, item: &Self::Item) -> bool {
        let _ = (query, item);
        true
    }
}

/// An entity whose list is paged, sorted, grouped and counted on the server.
#[async_trait]
pub trait PaginatedList:
    CrudHandlers
    + ListRows<
        Field = <Self as CrudHandlers>::OrderField,
        Query = <Self as CrudHandlers>::FilterQuery,
    > + 'static
where
    Self: Display + ChangeTriggersTopologyStaleness<Self> + Default,
    EntityEnum: From<Self>,
{
    /// The order of a list nobody sorted or grouped.
    fn default_order() -> String {
        format!("{}.created_at ASC", Self::table_name())
    }

    /// The rows of one page as the list sends them, in the order given.
    async fn items(
        state: &AppState,
        rows: Vec<Self>,
        query: &Self::FilterQuery,
    ) -> ApiResult<Vec<Self::Item>>;

    /// Whether grouping by `field` nests the rows as trees, one group per root.
    fn groups_as_tree(field: &Self::OrderField) -> bool {
        let _ = field;
        false
    }

    /// Where each of `ids` sits in its tree. A row in no tree has no entry. Called only for a list
    /// grouped by a field [`Self::groups_as_tree`] accepts.
    async fn tree_positions(
        state: &AppState,
        ids: &[Uuid],
    ) -> ApiResult<HashMap<Uuid, TreePosition>> {
        let _ = (state, ids);
        Ok(HashMap::new())
    }

    /// Whether `query` filters on something only code can derive, so every matching row is
    /// loaded and [`ListRows::keep`] decides.
    fn filters_in_memory(query: &Self::FilterQuery) -> bool {
        let _ = query;
        false
    }
}

/// A row's value for a field only code can derive.
#[derive(Debug, Clone, PartialEq)]
pub enum ComputedValue {
    Number(f64),
    Text(String),
}

impl ComputedValue {
    /// Ascending order; a row with no value sorts after every row with one.
    fn compare(a: &Option<Self>, b: &Option<Self>) -> Ordering {
        match (a, b) {
            (Some(Self::Number(a)), Some(Self::Number(b))) => a.total_cmp(b),
            (Some(Self::Text(a)), Some(Self::Text(b))) => a.cmp(b),
            (Some(Self::Number(_)), Some(Self::Text(_))) => Ordering::Less,
            (Some(Self::Text(_)), Some(Self::Number(_))) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
    }
}

/// How a computed field sorts and groups one row.
#[derive(Debug, Clone, PartialEq)]
pub struct Computed {
    /// What the row sorts by, which also orders its group among the others.
    pub value: Option<ComputedValue>,
    /// The key of the row's group in `group_counts`, as the UI's `getGroupValue` gives it.
    pub group: Option<String>,
}

/// One page of `T`'s list for this caller and query, with `group_counts` when it is grouped.
pub async fn list_page<T>(
    state: &AppState,
    scope: &ListScope,
    query: &T::FilterQuery,
) -> ApiResult<PaginatedApiResponse<T::Item>>
where
    T: PaginatedList + ChangeTriggersTopologyStaleness<T> + Default,
    EntityEnum: From<T>,
{
    let filter = T::list_filter(state, query, scope).await?;
    let pagination = query.pagination();
    let limit = pagination.effective_limit();
    let offset = pagination.effective_offset();
    let ordering = T::ordering(query);

    let tree_grouped = ordering.tag_group.group_by.is_none()
        && ordering
            .group_by
            .is_some_and(|field| T::groups_as_tree(&field));
    let in_memory = T::filters_in_memory(query)
        || ordering.group_by.is_some_and(|f| T::is_computed(&f))
        || ordering.order_by.is_some_and(|f| T::is_computed(&f));

    if in_memory {
        // A tree grouping wins over the SQL group a computed sort would otherwise fall back to.
        let (items, total_count, group_counts) = if tree_grouped {
            tree_memory_page::<T>(state, filter, ordering, query, limit, offset).await?
        } else {
            memory_page::<T>(state, filter, ordering, query, limit, offset).await?
        };
        return Ok(
            PaginatedApiResponse::success(items, total_count, limit.unwrap_or(0), offset)
                .with_optional_group_counts(group_counts),
        );
    }

    let (rows, total_count, group_counts) = if tree_grouped {
        tree_page::<T>(state, filter, ordering, limit, offset).await?
    } else {
        let (filter, order) =
            ordering.apply(pagination.apply_to_filter(filter), &T::default_order());
        let service = T::get_service(state);
        // Each group's full size, not the slice of it on this page. Counted under the same
        // filter, JOINs included, since the group expression may read from one.
        let group_counts = match ordering.group_sql() {
            Some(group_sql) => Some(service.count_by_group(filter.clone(), group_sql).await?),
            None => None,
        };
        let page = service.get_paginated_ordered(filter, &order).await?;
        (page.items, page.total_count, group_counts)
    };

    let items = T::items(state, rows, query).await?;
    Ok(
        PaginatedApiResponse::success(items, total_count, limit.unwrap_or(0), offset)
            .with_optional_group_counts(group_counts),
    )
}

/// One page of a list built in code, for this query. `rows` arrive in the order kept wherever
/// the ordering ties. Every field is computed, since there is no SQL to order by.
pub fn computed_page<L: ListRows>(
    rows: Vec<L::Item>,
    query: &L::Query,
    pagination: PaginationParams,
) -> PaginatedApiResponse<L::Item> {
    let rows = rows
        .into_iter()
        .filter(|row| L::keep(query, row))
        .map(|row| (None, row))
        .collect();
    let (limit, offset) = (pagination.effective_limit(), pagination.effective_offset());
    let (items, total_count, group_counts) = page_in_memory(
        rows,
        &L::ordering(query),
        |_| true,
        L::computed,
        limit,
        offset,
    );
    PaginatedApiResponse::success(items, total_count, limit.unwrap_or(0), offset)
        .with_optional_group_counts(group_counts)
}

/// A page of a list grouped by tree. SQL can't place a row under a parent it hasn't read, so
/// the order is built here from every matching row's id, and the page is sliced from it: each
/// parent leads its subtree, and siblings follow the sort. With no sort, siblings follow the
/// order global search lists the entity in.
async fn tree_page<T>(
    state: &AppState,
    filter: StorableFilter<T>,
    ordering: ListOrdering<T::OrderField>,
    limit: Option<u32>,
    offset: u32,
) -> ApiResult<(Vec<T>, u64, Option<Vec<GroupCount>>)>
where
    T: PaginatedList + ChangeTriggersTopologyStaleness<T> + Default,
    EntityEnum: From<T>,
{
    let sort = ListOrdering {
        order_by: ordering.order_by.or_else(T::search_order),
        ..ordering.sort_only()
    };
    let (filter, order) = sort.apply(filter, &T::default_order());
    let service = T::get_service(state);
    let ids: Vec<Uuid> = service
        .storage()
        .get_ordered_values(filter, &order, &[])
        .await?
        .into_iter()
        .map(|(id, _)| id)
        .collect();

    let (ordered, group_counts) = tree_layout::<T>(state, &ids).await?;
    let page = slice(ordered, limit, offset);
    let mut rows = service
        .get_all(StorableFilter::<T>::new_from_entity_ids(&page))
        .await?;
    let rank: HashMap<Uuid, usize> = page.iter().enumerate().map(|(i, id)| (*id, i)).collect();
    rows.sort_by_key(|row| rank.get(&row.id()).copied().unwrap_or(usize::MAX));

    Ok((rows, ids.len() as u64, Some(group_counts)))
}

/// A page of a list grouped by tree and sorted by something only code can derive: every matching
/// row is built into its item and ordered in memory, then laid out as trees like [`tree_page`].
/// Siblings follow the computed sort.
async fn tree_memory_page<T>(
    state: &AppState,
    filter: StorableFilter<T>,
    ordering: ListOrdering<T::OrderField>,
    query: &T::FilterQuery,
    limit: Option<u32>,
    offset: u32,
) -> ApiResult<(Vec<T::Item>, u64, Option<Vec<GroupCount>>)>
where
    T: PaginatedList + ChangeTriggersTopologyStaleness<T> + Default,
    EntityEnum: From<T>,
{
    let sql = ListOrdering {
        order_by: ordering.order_by.filter(|f| !T::is_computed(f)),
        ..ordering.sort_only()
    };
    let (filter, order) = sql.apply(filter, &T::default_order());
    let service = T::get_service(state);
    let ids: Vec<Uuid> = service
        .storage()
        .get_ordered_values(filter, &order, &[])
        .await?
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    let rank: HashMap<Uuid, usize> = ids.iter().enumerate().map(|(i, id)| (*id, i)).collect();
    let mut rows = service
        .get_all(StorableFilter::<T>::new_from_entity_ids(&ids))
        .await?;
    rows.sort_by_key(|row| rank.get(&row.id()).copied().unwrap_or(usize::MAX));

    // `items` keeps the order it is given, so each item lines up with its id.
    let items = T::items(state, rows, query).await?;
    let listed: Vec<_> = ids
        .into_iter()
        .zip(items)
        .filter(|(_, item)| T::keep(query, item))
        .map(|pair| (None::<String>, pair))
        .collect();
    let (sorted, total_count, _) = page_in_memory(
        listed,
        &ordering.sort_only(),
        T::is_computed,
        |field, (_, item): &(Uuid, T::Item)| T::computed(field, item),
        None,
        0,
    );

    let sorted_ids: Vec<Uuid> = sorted.iter().map(|(id, _)| *id).collect();
    let (ordered, group_counts) = tree_layout::<T>(state, &sorted_ids).await?;
    let mut by_id: HashMap<Uuid, T::Item> = sorted.into_iter().collect();
    let page = slice(ordered, limit, offset)
        .into_iter()
        .filter_map(|id| by_id.remove(&id))
        .collect();

    Ok((page, total_count, Some(group_counts)))
}

/// `ids` (every matching row, in sort order) in tree order, and each group's size in that order.
async fn tree_layout<T>(state: &AppState, ids: &[Uuid]) -> ApiResult<(Vec<Uuid>, Vec<GroupCount>)>
where
    T: PaginatedList + ChangeTriggersTopologyStaleness<T> + Default,
    EntityEnum: From<T>,
{
    let positions = T::tree_positions(state, ids).await?;
    let rows: Vec<(Uuid, String, Option<Uuid>)> = ids
        .iter()
        .map(|id| {
            let position = positions.get(id);
            (
                *id,
                TreePosition::group_key(position),
                position.and_then(|p| p.parent_id),
            )
        })
        .collect();
    let ordered = tree_order(&rows);

    let key_of: HashMap<Uuid, &str> = rows
        .iter()
        .map(|(id, key, _)| (*id, key.as_str()))
        .collect();
    let mut group_counts: Vec<GroupCount> = Vec::new();
    for id in &ordered {
        let key = key_of.get(id).copied().unwrap_or_default();
        match group_counts.last_mut() {
            Some(last) if last.value.as_deref() == Some(key) => last.count += 1,
            _ => group_counts.push(GroupCount {
                value: Some(key.to_string()),
                count: 1,
            }),
        }
    }
    Ok((ordered, group_counts))
}

/// The ids of one page.
fn slice(ordered: Vec<Uuid>, limit: Option<u32>, offset: u32) -> Vec<Uuid> {
    ordered
        .into_iter()
        .skip(offset as usize)
        .take(limit.map_or(usize::MAX, |l| l as usize))
        .collect()
}

/// A page of a list ordered, grouped or filtered by something only code can derive. Every
/// matching row is loaded and built into its item, so this is for lists that stay small per
/// organization. The SQL part of the ordering still runs in the database: rows arrive in it, and a
/// SQL grouping keeps its order while a computed sort orders each group.
async fn memory_page<T>(
    state: &AppState,
    filter: StorableFilter<T>,
    ordering: ListOrdering<T::OrderField>,
    query: &T::FilterQuery,
    limit: Option<u32>,
    offset: u32,
) -> ApiResult<(Vec<T::Item>, u64, Option<Vec<GroupCount>>)>
where
    T: PaginatedList + ChangeTriggersTopologyStaleness<T> + Default,
    EntityEnum: From<T>,
{
    let sql = ListOrdering {
        group_by: ordering.group_by.filter(|f| !T::is_computed(f)),
        order_by: ordering.order_by.filter(|f| !T::is_computed(f)),
        ..ordering
    };
    let sql_group = sql.group_sql();
    let (filter, order) = sql.apply(filter, &T::default_order());

    let service = T::get_service(state);
    let keyed = service
        .storage()
        .get_ordered_values(filter, &order, sql_group.as_slice())
        .await?;
    let ids: Vec<Uuid> = keyed.iter().map(|(id, _)| *id).collect();
    let rank: HashMap<Uuid, usize> = ids.iter().enumerate().map(|(i, id)| (*id, i)).collect();
    let mut rows = service
        .get_all(StorableFilter::<T>::new_from_entity_ids(&ids))
        .await?;
    rows.sort_by_key(|row| rank.get(&row.id()).copied().unwrap_or(usize::MAX));
    let sql_groups: Vec<Option<String>> = keyed
        .into_iter()
        .map(|(_, values)| values.into_iter().next().flatten())
        .collect();

    // `items` keeps the order it is given, so each item lines up with its SQL rank and group.
    let items = T::items(state, rows, query).await?;
    let rows: Vec<(Option<String>, T::Item)> = sql_groups
        .into_iter()
        .zip(items)
        .filter(|(_, item)| T::keep(query, item))
        .collect();
    Ok(page_in_memory(
        rows,
        &ordering,
        T::is_computed,
        T::computed,
        limit,
        offset,
    ))
}

/// Order, group, count and slice rows that only code can fully order. `rows` arrive in the order
/// to keep wherever the computed ordering ties, each with its SQL group key (`None` when the
/// grouping is computed or absent). A SQL grouping keeps the order its groups arrived in; a
/// computed grouping and a computed sort order by their values, the sort in the requested
/// direction.
pub fn page_in_memory<I, O: OrderField>(
    rows: Vec<(Option<String>, I)>,
    ordering: &ListOrdering<O>,
    is_computed: impl Fn(&O) -> bool,
    computed: impl Fn(&O, &I) -> Option<Computed>,
    limit: Option<u32>,
    offset: u32,
) -> (Vec<I>, u64, Option<Vec<GroupCount>>) {
    let computed_group = ordering.group_by.filter(|f| is_computed(f));
    let computed_sort = ordering.order_by.filter(|f| is_computed(f));
    let value = |field: &O, item: &I| computed(field, item).and_then(|c| c.value);

    let mut listed: Vec<(usize, Option<String>, I)> = rows
        .into_iter()
        .enumerate()
        .map(|(rank, (group, item))| (rank, group, item))
        .collect();
    let first_seen: HashMap<Option<String>, usize> =
        listed
            .iter()
            .fold(HashMap::new(), |mut seen, (rank, group, _)| {
                seen.entry(group.clone()).or_insert(*rank);
                seen
            });
    let descending = matches!(ordering.direction, Some(OrderDirection::Desc));
    listed.sort_by(|(rank_a, group_a, a), (rank_b, group_b, b)| {
        let by_group = match computed_group {
            Some(field) => ComputedValue::compare(&value(&field, a), &value(&field, b)),
            None => first_seen[group_a].cmp(&first_seen[group_b]),
        };
        let by_sort = match computed_sort {
            Some(field) => {
                let order = ComputedValue::compare(&value(&field, a), &value(&field, b));
                if descending { order.reverse() } else { order }
            }
            None => rank_a.cmp(rank_b),
        };
        by_group.then(by_sort).then(rank_a.cmp(rank_b))
    });

    let grouped = ordering.group_by.is_some() || ordering.tag_group.group_by.is_some();
    let group_counts = grouped.then(|| {
        let mut counts: Vec<GroupCount> = Vec::new();
        for (_, sql_group, item) in &listed {
            let key = match computed_group {
                Some(field) => computed(&field, item).and_then(|c| c.group),
                None => sql_group.clone(),
            };
            match counts.last_mut() {
                Some(last) if last.value == key => last.count += 1,
                _ => counts.push(GroupCount {
                    value: key,
                    count: 1,
                }),
            }
        }
        counts
    });

    let total_count = listed.len() as u64;
    let page = listed
        .into_iter()
        .skip(offset as usize)
        .take(limit.map_or(usize::MAX, |l| l as usize))
        .map(|(_, _, item)| item)
        .collect();
    (page, total_count, group_counts)
}
