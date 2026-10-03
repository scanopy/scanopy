//! Host-specific filters.
use super::*;

use crate::server::hosts::r#impl::base::Host;

impl StorableFilter<Host> {
    /// Hosts assigned one of `credential_ids`, through the `host_credentials` junction.
    ///
    /// `IN (subquery)` rather than a JOIN so a host holding several matching credentials is still
    /// one row, the same reason [`Self::has_service_named`] gives. An empty selection matches
    /// nothing.
    pub fn has_credential(mut self, credential_ids: &[Uuid]) -> Self {
        if credential_ids.is_empty() {
            self.conditions.push("FALSE".to_string());
            return self;
        }

        let col = self.qualify_column("id");
        self.conditions.push(format!(
            "{} IN (SELECT hc.host_id FROM host_credentials hc WHERE hc.credential_id = ANY(${}))",
            col,
            self.values.len() + 1
        ));
        self.values
            .push(SqlValue::UuidArray(credential_ids.to_vec()));
        self
    }
}
