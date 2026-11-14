//! Generic "value is one of these" filters over a plain text or boolean column, or a text leaf in a JSONB
//! column, for the field filters a paginated list applies server-side.
use super::*;

impl<T: Storable> StorableFilter<T> {
    /// Rows whose text `column` holds one of `values`.
    ///
    /// An empty selection matches nothing, the rule every inclusion filter here follows.
    pub fn text_column_in(mut self, column: &str, values: &[String]) -> Self {
        let col = self.qualify_column(column);
        self.push_text_in(col, values.iter().cloned());
        self
    }

    /// Rows whose `expr`, read as text, is one of `values`. For a filter on the same expression an
    /// order field sorts and groups by, so a derived column filters on exactly what it shows.
    /// `expr` is SQL written in code, never request input; the values are bound.
    pub fn expr_text_in(mut self, expr: &str, values: &[String]) -> Self {
        self.push_text_in(format!("({expr})::text"), values.iter().cloned());
        self
    }

    /// Rows whose boolean `column` holds one of `values`. Both values match every row with one;
    /// none matches nothing.
    pub fn bool_column_in(mut self, column: &str, values: &[bool]) -> Self {
        let col = self.qualify_column(column);
        match (values.contains(&true), values.contains(&false)) {
            (true, true) => self.conditions.push(format!("{col} IS NOT NULL")),
            (false, false) => self.conditions.push("FALSE".to_string()),
            (only, _) => {
                self.conditions
                    .push(format!("{} = ${}", col, self.values.len() + 1));
                self.values.push(SqlValue::Bool(only));
            }
        }
        self
    }

    /// Rows whose `column` is NULL (`null`) or holds a value (`!null`).
    pub fn column_is_null(mut self, column: &str, null: bool) -> Self {
        let col = self.qualify_column(column);
        self.conditions.push(if null {
            format!("{col} IS NULL")
        } else {
            format!("{col} IS NOT NULL")
        });
        self
    }

    /// Rows whose JSONB `column`, followed down `path`, ends at a text value that is one of
    /// `values`. `json_text_in("os", &["family"], …)` compares `os->>'family'`;
    /// `json_text_in("run_type", &["results", "phase"], …)` compares
    /// `run_type->'results'->>'phase'`.
    ///
    /// `path` names keys the caller writes in code, never request input, so they are inlined. The
    /// values are bound. An empty selection matches nothing.
    pub fn json_text_in<V: AsRef<str>>(
        mut self,
        column: &str,
        path: &[&str],
        values: &[V],
    ) -> Self {
        let expr = json_text_path(&self.qualify_column(column), path);
        self.push_text_in(expr, values.iter().map(|v| v.as_ref().to_string()));
        self
    }

    /// [`Self::json_text_in`] for a leaf that serde defaults to `default` when absent. A row whose
    /// `column` is set but holds no such leaf reads as `default`, so it matches when `default` is
    /// selected. A row with `column` NULL never matches.
    pub fn json_text_in_defaulting<V: AsRef<str>>(
        mut self,
        column: &str,
        path: &[&str],
        values: &[V],
        default: &str,
    ) -> Self {
        let qualified = self.qualify_column(column);
        let expr = json_text_path(&qualified, path);
        let includes_default = values.iter().any(|v| v.as_ref() == default);
        self.push_text_in(expr.clone(), values.iter().map(|v| v.as_ref().to_string()));
        if includes_default && let Some(condition) = self.conditions.pop() {
            self.conditions.push(format!(
                "({condition} OR ({qualified} IS NOT NULL AND {expr} IS NULL))"
            ));
        }
        self
    }

    /// Push `expr = ANY($n)` with the values bound as one text array, or `FALSE` for none.
    fn push_text_in(&mut self, expr: String, values: impl ExactSizeIterator<Item = String>) {
        if values.len() == 0 {
            self.conditions.push("FALSE".to_string());
            return;
        }
        self.push_any(&expr, SqlValue::StringArray(values.collect()));
    }
}

/// `col->'a'->>'b'`: every key but the last keeps the value JSON, the last reads it as text.
fn json_text_path(column: &str, path: &[&str]) -> String {
    match path.split_last() {
        Some((last, init)) => {
            let mut expr = column.to_string();
            for key in init {
                expr.push_str(&format!("->'{key}'"));
            }
            expr.push_str(&format!("->>'{last}'"));
            expr
        }
        None => format!("{column}#>>'{{}}'"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::discovery::r#impl::base::Discovery;
    use crate::server::hosts::r#impl::base::Host;

    /// The intermediate keys must stay JSON and only the leaf become text: `->>` midway would
    /// hand the next accessor a text value, which Postgres rejects.
    #[test]
    fn json_text_in_reads_nested_leaves_as_text() {
        let filter = StorableFilter::<Discovery>::new_unfiltered().json_text_in(
            "run_type",
            &["results", "phase"],
            &["Complete", "Failed"],
        );

        assert_eq!(
            filter.to_where_clause(),
            "WHERE discovery.run_type->'results'->>'phase' = ANY($1)"
        );
        assert_eq!(filter.values().len(), 1);
    }

    #[test]
    fn text_column_in_binds_after_earlier_conditions() {
        let filter = StorableFilter::<Host>::new_unfiltered()
            .text_column_in("model", &["C9300".to_string()])
            .text_column_in("manufacturer", &["Cisco".to_string()]);

        assert!(
            filter
                .to_where_clause()
                .contains("hosts.manufacturer = ANY($2)"),
            "placeholders must continue from the binds already pushed: {}",
            filter.to_where_clause()
        );
        assert_eq!(filter.values().len(), 2);
    }
}
