//! Subnet-specific filters.
use super::*;

use crate::server::shared::types::metadata::HasId;
use crate::server::subnets::r#impl::{base::Subnet, types::SubnetType};

impl StorableFilter<Subnet> {
    /// SQL form of [`Subnet::is_user_managed`] — the subnets that belong to the
    /// inventory the user curates, which is what the management lists show and
    /// what the dashboard's subnet count counts.
    ///
    /// Keyed on provenance as well as category, so a subnet the user created is
    /// counted whatever category they gave it. Counting the fabricated rows while
    /// the lists omitted them is the discrepancy reported in GH #677.
    ///
    /// This mirrors [`Subnet::is_user_managed`] — the two must be changed
    /// together, the same convention [`Self::stale_by_site`] follows with
    /// [`DiscoveryTracked::freshness`](crate::server::shared::storage::snapshot::DiscoveryTracked::freshness).
    /// The category list itself is not restated here: it is bound from
    /// [`SubnetType::synthetic_categories`].
    pub fn user_managed(mut self) -> Self {
        let source_col = self.qualify_column("source");
        let type_col = self.qualify_column("subnet_type");

        let placeholders: Vec<String> = SubnetType::synthetic_categories()
            .iter()
            .map(|subnet_type| {
                let idx = self.values.len() + 1;
                self.values
                    .push(SqlValue::String(subnet_type.id().to_string()));
                format!("${idx}")
            })
            .collect();

        self.conditions.push(format!(
            "({source_col}->>'type' = 'Manual' OR {type_col} NOT IN ({}))",
            placeholders.join(", ")
        ));
        self
    }
}
