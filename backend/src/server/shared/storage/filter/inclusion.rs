//! Generic "value is one of these" filters over a plain text column or a text leaf inside a JSONB
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

    /// Push `expr IN ($n, …)` with one bound string per value, or `FALSE` for none.
    fn push_text_in(&mut self, expr: String, values: impl ExactSizeIterator<Item = String>) {
        if values.len() == 0 {
            self.conditions.push("FALSE".to_string());
            return;
        }

        let start = self.values.len();
        let placeholders: Vec<String> = (0..values.len())
            .map(|i| format!("${}", start + i + 1))
            .collect();
        self.conditions
            .push(format!("{} IN ({})", expr, placeholders.join(", ")));
        self.values.extend(values.map(SqlValue::String));
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
            "WHERE discovery.run_type->'results'->>'phase' IN ($1, $2)"
        );
        assert_eq!(filter.values().len(), 2);
    }

    #[test]
    fn text_column_in_binds_after_earlier_conditions() {
        let filter = StorableFilter::<Host>::new_unfiltered()
            .hidden_in(&[false])
            .text_column_in("manufacturer", &["Cisco".to_string()]);

        assert!(
            filter
                .to_where_clause()
                .contains("hosts.manufacturer IN ($2)"),
            "placeholders must continue from the binds already pushed: {}",
            filter.to_where_clause()
        );
        assert_eq!(filter.values().len(), 2);
    }
}
