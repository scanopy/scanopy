//! Filters on what a row is linked to through a junction table, for list filters on a relation
//! the row doesn't hold as a column.
use super::*;

/// A junction table linking rows of one entity to another.
#[derive(Debug, Clone, Copy)]
pub struct Junction {
    /// The junction table.
    pub table: &'static str,
    /// Its column naming the row being filtered.
    pub own_column: &'static str,
    /// Its column naming the linked row.
    pub other_column: &'static str,
    /// Whether the junction keeps SCD2 history, so only rows with no `valid_to` are live links.
    pub has_history: bool,
}

impl<T: Storable> StorableFilter<T> {
    /// Rows linked through `junction` to one of `ids`.
    ///
    /// `IN (subquery)` rather than a JOIN, so a row linked to several of `ids` is still one row
    /// and the paginated `COUNT(*)` stays right. An empty `ids` matches nothing.
    pub fn linked_to(self, junction: Junction, ids: &[Uuid]) -> Self {
        self.linked_where(junction, ids, false)
    }

    /// [`Self::linked_to`], also matching rows whose link names no row, which `junction` uses to
    /// mean every row (an All sites assignment).
    pub fn linked_to_or_every(self, junction: Junction, ids: &[Uuid]) -> Self {
        self.linked_where(junction, ids, true)
    }

    fn linked_where(mut self, junction: Junction, ids: &[Uuid], or_every: bool) -> Self {
        if ids.is_empty() {
            self.conditions.push("FALSE".to_string());
            return self;
        }

        let col = self.qualify_column("id");
        let placeholders: Vec<String> = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", self.values.len() + i + 1))
            .collect();
        let live = if junction.has_history {
            " AND link.valid_to IS NULL"
        } else {
            ""
        };

        let every = if or_every {
            format!(" OR link.{} IS NULL", junction.other_column)
        } else {
            String::new()
        };

        self.conditions.push(format!(
            "{col} IN (SELECT link.{own} FROM {table} link WHERE (link.{other} IN ({ids}){every}){live})",
            own = junction.own_column,
            table = junction.table,
            other = junction.other_column,
            ids = placeholders.join(", "),
        ));
        self.values.extend(ids.iter().copied().map(SqlValue::Uuid));
        self
    }
}
