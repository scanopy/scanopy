//! Generic ordering support for entity queries.
//!
//! Provides the `OrderField` trait and `apply_ordering` function to eliminate
//! duplicated ordering logic across entity handlers.

use serde::{Deserialize, Serialize};

use crate::server::shared::storage::{filter::StorableFilter, traits::Storable};

use super::query::OrderDirection;

// ============================================================================
// OrderField Trait
// ============================================================================

/// Trait for order field enums that generate SQL ORDER BY expressions.
///
/// Implement this for entity-specific OrderField enums to enable generic
/// ordering via `apply_ordering()`.
pub trait OrderField: Clone + Copy + Default + Send + Sync + 'static {
    /// Returns the SQL ORDER BY expression for this field.
    ///
    /// The expression should be fully qualified with table name or alias.
    /// Examples: `"hosts.created_at"`, `"COALESCE(virt_service.name, '')"`
    fn to_sql(&self) -> &'static str;

    /// Returns the JOIN clause if this field requires one, None otherwise.
    ///
    /// Example: `"LEFT JOIN services AS virt_service ON ..."`
    fn join_sql(&self) -> Option<&'static str> {
        None
    }

    /// Whether rows with no value for this field sort last in both directions.
    ///
    /// Postgres puts NULLs last for ASC but first for DESC. A field whose NULL means "nothing to
    /// sort by" (e.g. a host with no MAC) returns true so those rows never lead a DESC listing.
    fn nulls_last(&self) -> bool {
        false
    }

    /// The field's ORDER BY term in the given direction.
    fn order_term(&self, dir: &str) -> String {
        if self.nulls_last() {
            format!("{} {} NULLS LAST", self.to_sql(), dir)
        } else {
            format!("{} {}", self.to_sql(), dir)
        }
    }
}

/// The order field of an entity with no server-side ordering.
///
/// Its one variant is the order the generic `get_all_handler` already lists by, unqualified
/// because that handler's query carries no JOIN.
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, Default, utoipa::ToSchema, strum::EnumIter,
)]
#[serde(rename_all = "snake_case")]
pub enum NoOrderField {
    #[default]
    CreatedAt,
}

impl OrderField for NoOrderField {
    fn to_sql(&self) -> &'static str {
        match self {
            Self::CreatedAt => "created_at",
        }
    }
}

// ============================================================================
// Generic apply_ordering Function
// ============================================================================

/// Apply ordering to a filter based on group_by, order_by, and direction.
///
/// This function handles:
/// - Adding JOINs required by order fields
/// - Avoiding duplicate JOINs when group_by and order_by use the same JOIN
/// - Building the ORDER BY clause with group_by first (always ASC) then order_by
/// - Ending a field ordering with the row id, so rows that tie on the field (every NULL, say)
///   keep one order across pages instead of repeating on one page and vanishing from the next
///
/// Returns: (modified_filter, order_by_sql)
pub fn apply_ordering<T, O>(
    group_by: Option<O>,
    order_by: Option<O>,
    direction: Option<OrderDirection>,
    mut filter: StorableFilter<T>,
    default_order: &str,
) -> (StorableFilter<T>, String)
where
    T: Storable,
    O: OrderField,
{
    let mut order_parts = Vec::new();

    // Primary: group_by field (always ASC to keep groups together)
    if let Some(group_field) = group_by {
        if let Some(join) = group_field.join_sql() {
            filter = filter.join(join);
        }
        order_parts.push(group_field.order_term("ASC"));
    }

    // Secondary: order_by field with specified direction
    if let Some(order_field) = order_by {
        // Only add JOIN if not already added by group_by
        let group_join = group_by.and_then(|g| g.join_sql());
        let order_join = order_field.join_sql();
        if let Some(join) = order_join
            && group_join != order_join
        {
            filter = filter.join(join);
        }
        let dir = direction.unwrap_or_default().to_sql();
        order_parts.push(order_field.order_term(dir));
    }

    let order_by_sql = if order_parts.is_empty() {
        default_order.to_string()
    } else {
        order_parts.push(format!("{}.id ASC", T::table_name()));
        order_parts.join(", ")
    };

    (filter, order_by_sql)
}
