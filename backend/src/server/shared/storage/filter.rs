use std::marker::PhantomData;

use chrono::{DateTime, Utc};
use email_address::EmailAddress;
use mac_address::MacAddress;
use uuid::Uuid;

use crate::server::{
    daemons::r#impl::base::DaemonMode,
    shared::{entities::EntityDiscriminants, storage::traits::SqlValue},
    users::r#impl::permissions::UserOrgPermissions,
};

use super::traits::Storable;

/// Builder pattern for common WHERE clauses with optional pagination and JOINs.
/// Generic over entity type T to automatically qualify column names with the table name.
#[derive(Clone)]
pub struct StorableFilter<T: Storable> {
    _marker: PhantomData<T>,
    conditions: Vec<String>,
    values: Vec<SqlValue>,
    limit_value: Option<u32>,
    offset_value: Option<u32>,
    joins: Vec<String>,
}

impl<T: Storable> StorableFilter<T> {
    fn new() -> Self {
        Self {
            _marker: PhantomData,
            conditions: Vec::new(),
            values: Vec::new(),
            limit_value: None,
            offset_value: None,
            joins: Vec::new(),
        }
    }

    pub fn new_from_org_id(org_id: &Uuid) -> Self {
        Self::new().organization_id(org_id)
    }

    pub fn new_from_network_ids(network_ids: &[Uuid]) -> Self {
        Self::new().network_ids(network_ids)
    }

    pub fn new_from_entity_id(entity_id: &Uuid) -> Self {
        Self::new().entity_id(entity_id)
    }

    pub fn new_from_entity_ids(entity_ids: &[Uuid]) -> Self {
        Self::new().entity_ids(entity_ids)
    }

    pub fn new_from_api_key(api_key: String) -> Self {
        Self::new().api_key(api_key)
    }

    pub fn new_from_email(email: &EmailAddress) -> Self {
        Self::new().email(email)
    }

    pub fn new_from_oidc_subject(oidc_subject: String) -> Self {
        Self::new().oidc_subject(oidc_subject)
    }

    pub fn new_from_password_reset_token(token: &str) -> Self {
        Self::new().password_reset_token(token)
    }

    pub fn new_from_email_verification_token(token: &str) -> Self {
        Self::new().email_verification_token(token)
    }

    pub fn new_from_host_ids(host_ids: &[Uuid]) -> Self {
        Self::new().host_ids(host_ids)
    }

    pub fn new_from_service_id(service_id: &Uuid) -> Self {
        Self::new().service_id(service_id)
    }

    pub fn new_from_subnet_id(subnet_id: &Uuid) -> Self {
        Self::new().subnet_id(subnet_id)
    }

    pub fn new_from_binding_id(binding_id: &Uuid) -> Self {
        Self::new().binding_id(binding_id)
    }

    pub fn new_from_user_id(user_id: &Uuid) -> Self {
        Self::new().user_id(user_id)
    }

    pub fn new_from_user_ids(user_ids: &[Uuid]) -> Self {
        Self::new().user_ids(user_ids)
    }

    pub fn new_from_interface_id(interface_id: &Uuid) -> Self {
        Self::new().interface_id(interface_id)
    }

    pub fn new_from_group_ids(group_ids: &[Uuid]) -> Self {
        Self::new().group_ids(group_ids)
    }

    pub fn new_from_uuid_column(column: &str, id: &Uuid) -> Self {
        Self::new().uuid_column(column, id)
    }

    pub fn new_from_uuids_column(column: &str, ids: &[Uuid]) -> Self {
        Self::new().uuids_column(column, ids)
    }

    pub fn new_for_scheduled_discoveries() -> Self {
        Self::new().scheduled_discovery()
    }

    pub fn new_for_unresolved_lldp_in_network(network_id: Uuid) -> Self {
        Self::new().unresolved_lldp_in_network(network_id)
    }

    pub fn new_without_brevo_company_id() -> Self {
        Self::new().without_brevo_company_id()
    }

    pub fn new_with_brevo_company_id() -> Self {
        Self::new().with_brevo_company_id()
    }

    pub fn new_with_stripe_customer_id(id: &str) -> Self {
        Self::new().stripe_customer_id(id)
    }

    pub fn new_with_expiry_before(timestamp: DateTime<Utc>) -> Self {
        Self::new().expires_before(timestamp)
    }

    pub fn new_for_daemon_poller_system_job() -> Self {
        Self::new()
            .daemon_mode(DaemonMode::ServerPoll)
            .is_unreachable(false)
            .standby(false)
    }

    /// Qualify a column name with the table name.
    fn qualify_column(&self, column: &str) -> String {
        format!("{}.{}", T::table_name(), column)
    }

    /// Set the maximum number of results to return.
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit_value = Some(limit);
        self
    }

    /// Set the number of results to skip.
    pub fn offset(mut self, offset: u32) -> Self {
        self.offset_value = Some(offset);
        self
    }

    /// Get the limit value, if set.
    pub fn get_limit(&self) -> Option<u32> {
        self.limit_value
    }

    /// Get the offset value, if set.
    pub fn get_offset(&self) -> Option<u32> {
        self.offset_value
    }

    /// Generate LIMIT clause if limit is set.
    pub fn to_limit_clause(&self) -> String {
        match self.limit_value {
            Some(limit) => format!("LIMIT {}", limit),
            None => String::new(),
        }
    }

    /// Generate OFFSET clause if offset is set.
    pub fn to_offset_clause(&self) -> String {
        match self.offset_value {
            Some(offset) if offset > 0 => format!("OFFSET {}", offset),
            _ => String::new(),
        }
    }

    /// Generate combined LIMIT and OFFSET clause.
    pub fn to_pagination_clause(&self) -> String {
        let mut parts = Vec::new();
        if let Some(limit) = self.limit_value {
            parts.push(format!("LIMIT {}", limit));
        }
        if let Some(offset) = self.offset_value
            && offset > 0
        {
            parts.push(format!("OFFSET {}", offset));
        }
        parts.join(" ")
    }

    /// Add a JOIN clause to the filter.
    /// Example: `filter.join("LEFT JOIN services AS s ON hosts.service_id = s.id")`
    pub fn join(mut self, join_clause: &str) -> Self {
        self.joins.push(join_clause.to_string());
        self
    }

    /// Generate the combined JOIN clause string.
    pub fn to_join_clause(&self) -> String {
        self.joins.join(" ")
    }

    /// Returns true if this filter has any JOIN clauses.
    pub fn has_joins(&self) -> bool {
        !self.joins.is_empty()
    }

    pub fn entity_id(mut self, id: &Uuid) -> Self {
        let col = self.qualify_column("id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*id));
        self
    }

    pub fn entity_ids(mut self, ids: &[Uuid]) -> Self {
        if ids.is_empty() {
            // Empty IN clause should match nothing
            self.conditions.push("FALSE".to_string());
            return self;
        }

        let col = self.qualify_column("id");
        let placeholders: Vec<String> = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", self.values.len() + i + 1))
            .collect();

        self.conditions
            .push(format!("{} IN ({})", col, placeholders.join(", ")));

        for id in ids {
            self.values.push(SqlValue::Uuid(*id));
        }

        self
    }

    pub fn network_ids(mut self, ids: &[Uuid]) -> Self {
        if ids.is_empty() {
            // Empty IN clause should match nothing
            self.conditions.push("FALSE".to_string());
            return self;
        }

        let col = self.qualify_column("network_id");
        let placeholders: Vec<String> = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", self.values.len() + i + 1))
            .collect();

        self.conditions
            .push(format!("{} IN ({})", col, placeholders.join(", ")));

        for id in ids {
            self.values.push(SqlValue::Uuid(*id));
        }

        self
    }

    pub fn user_id(mut self, id: &Uuid) -> Self {
        let col = self.qualify_column("user_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*id));
        self
    }

    pub fn user_ids(mut self, ids: &[Uuid]) -> Self {
        if ids.is_empty() {
            // Empty IN clause should match nothing
            self.conditions.push("FALSE".to_string());
            return self;
        }

        let col = self.qualify_column("user_id");
        let placeholders: Vec<String> = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", self.values.len() + i + 1))
            .collect();

        self.conditions
            .push(format!("{} IN ({})", col, placeholders.join(", ")));

        for id in ids {
            self.values.push(SqlValue::Uuid(*id));
        }

        self
    }

    pub fn hidden_is(mut self, hidden: bool) -> Self {
        let col = self.qualify_column("hidden");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Bool(hidden));
        self
    }

    pub fn host_id(mut self, id: &Uuid) -> Self {
        let col = self.qualify_column("host_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*id));
        self
    }

    pub fn subnet_id(mut self, id: &Uuid) -> Self {
        let col = self.qualify_column("subnet_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*id));
        self
    }

    pub fn mac_address(mut self, mac: &MacAddress) -> Self {
        let col = self.qualify_column("mac_address");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::MacAddress(*mac));
        self
    }

    pub fn password_reset_token(mut self, token: &str) -> Self {
        let col = self.qualify_column("password_reset_token");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::String(token.to_string()));
        self
    }

    pub fn email_verification_token(mut self, token: &str) -> Self {
        let col = self.qualify_column("email_verification_token");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::String(token.to_string()));
        self
    }

    pub fn name(mut self, name: String) -> Self {
        let col = self.qualify_column("name");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::String(name));
        self
    }

    pub fn group_id(mut self, id: &Uuid) -> Self {
        let col = self.qualify_column("group_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*id));
        self
    }

    pub fn group_ids(mut self, ids: &[Uuid]) -> Self {
        if ids.is_empty() {
            self.conditions.push("FALSE".to_string());
            return self;
        }

        let col = self.qualify_column("group_id");
        let placeholders: Vec<String> = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", self.values.len() + i + 1))
            .collect();

        self.conditions
            .push(format!("{} IN ({})", col, placeholders.join(", ")));

        for id in ids {
            self.values.push(SqlValue::Uuid(*id));
        }

        self
    }

    pub fn binding_id(mut self, id: &Uuid) -> Self {
        let col = self.qualify_column("binding_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*id));
        self
    }

    pub fn host_ids(mut self, ids: &[Uuid]) -> Self {
        if ids.is_empty() {
            // Empty IN clause should match nothing
            self.conditions.push("FALSE".to_string());
            return self;
        }

        let col = self.qualify_column("host_id");
        let placeholders: Vec<String> = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", self.values.len() + i + 1))
            .collect();

        self.conditions
            .push(format!("{} IN ({})", col, placeholders.join(", ")));

        for id in ids {
            self.values.push(SqlValue::Uuid(*id));
        }

        self
    }

    pub fn api_key(mut self, api_key: String) -> Self {
        let col = self.qualify_column("key");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::String(api_key));
        self
    }

    pub fn scheduled_discovery(mut self) -> Self {
        self.conditions
            .push("run_type->>'type' = 'Scheduled'".to_string());
        self.conditions
            .push("(run_type->>'enabled')::boolean = true".to_string());
        self
    }

    pub fn historical_discovery(mut self) -> Self {
        self.conditions
            .push("run_type->>'type' = 'Historical'".to_string());
        self
    }

    pub fn oidc_subject(mut self, subject: String) -> Self {
        let col = self.qualify_column("oidc_subject");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::String(subject));
        let provider_col = self.qualify_column("oidc_provider");
        self.conditions
            .push(format!("{} IS NOT NULL", provider_col));
        self
    }

    pub fn email(mut self, email: &EmailAddress) -> Self {
        let col = self.qualify_column("email");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Email(email.clone()));
        self
    }

    pub fn organization_id(mut self, organization_id: &Uuid) -> Self {
        let col = self.qualify_column("organization_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*organization_id));
        self
    }

    pub fn topology_id(mut self, topology_id: &Uuid) -> Self {
        let col = self.qualify_column("topology_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*topology_id));
        self
    }

    pub fn user_permissions(mut self, permissions: &UserOrgPermissions) -> Self {
        let col = self.qualify_column("permissions");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::UserOrgPermissions(*permissions));
        self
    }

    pub fn expires_before(mut self, timestamp: DateTime<Utc>) -> Self {
        let col = self.qualify_column("expires_at");
        self.conditions
            .push(format!("{} < ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Timestamp(timestamp));
        self
    }

    /// Generic UUID filter for any column name.
    /// Used by generic child entity handlers to filter by parent_column dynamically.
    pub fn uuid_column(mut self, column: &str, id: &Uuid) -> Self {
        let col = self.qualify_column(column);
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*id));
        self
    }

    /// Generic UUID IN filter for any column name.
    /// Used by generic child entity services to filter by parent_column dynamically.
    pub fn uuids_column(mut self, column: &str, ids: &[Uuid]) -> Self {
        if ids.is_empty() {
            self.conditions.push("FALSE".to_string());
            return self;
        }

        let col = self.qualify_column(column);
        let placeholders: Vec<String> = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", self.values.len() + i + 1))
            .collect();

        self.conditions
            .push(format!("{} IN ({})", col, placeholders.join(", ")));

        for id in ids {
            self.values.push(SqlValue::Uuid(*id));
        }

        self
    }

    /// Filter by service_id (for bindings)
    pub fn service_id(mut self, id: &Uuid) -> Self {
        let col = self.qualify_column("service_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*id));
        self
    }

    /// Filter by mode (for daemons)
    pub fn daemon_mode(mut self, mode: DaemonMode) -> Self {
        let col = self.qualify_column("mode");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::DaemonMode(mode));
        self
    }

    /// Filter by mode (for daemons)
    pub fn is_unreachable(mut self, is_unreachable: bool) -> Self {
        let col = self.qualify_column("is_unreachable");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Bool(is_unreachable));
        self
    }

    pub fn standby(mut self, standby: bool) -> Self {
        let col = self.qualify_column("standby");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Bool(standby));
        self
    }

    /// Filter by entity_type (for entity_tags junction table)
    pub fn entity_type(mut self, entity_type: &EntityDiscriminants) -> Self {
        let col = self.qualify_column("entity_type");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        // Use EntityDiscriminant to match JSON serialization used when inserting
        self.values.push(SqlValue::EntityDiscriminant(*entity_type));
        self
    }

    /// Filter by tag_id (for entity_tags junction table)
    pub fn tag_id(mut self, id: &Uuid) -> Self {
        let col = self.qualify_column("tag_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*id));
        self
    }

    /// Filter entities that have ANY of the specified tags.
    /// Uses a subquery against the entity_tags junction table.
    ///
    /// Example SQL: `entities.id IN (SELECT entity_id FROM entity_tags WHERE entity_type = 'Service' AND tag_id IN ($1, $2))`
    pub fn has_any_tags(mut self, tag_ids: &[Uuid], entity_type: EntityDiscriminants) -> Self {
        if tag_ids.is_empty() {
            return self;
        }

        let col = self.qualify_column("id");
        let entity_type_idx = self.values.len() + 1;
        let placeholders: Vec<String> = tag_ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", self.values.len() + i + 2))
            .collect();

        self.conditions.push(format!(
            "{} IN (SELECT entity_id FROM entity_tags WHERE entity_type = ${} AND tag_id IN ({}))",
            col,
            entity_type_idx,
            placeholders.join(", ")
        ));

        self.values.push(SqlValue::EntityDiscriminant(entity_type));
        for id in tag_ids {
            self.values.push(SqlValue::Uuid(*id));
        }

        self
    }

    pub fn to_where_clause(&self) -> String {
        if self.conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", self.conditions.join(" AND "))
        }
    }

    pub fn values(&self) -> &[SqlValue] {
        &self.values
    }

    // =========================================================================
    // LLDP resolution filters
    // =========================================================================

    /// Filter by IP address (for interfaces table)
    pub fn ip_address(mut self, ip: std::net::IpAddr) -> Self {
        let col = self.qualify_column("ip_address");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::IpAddr(ip));
        self
    }

    /// Filter by if_descr (for if_entries table)
    pub fn if_descr(mut self, descr: &str) -> Self {
        let col = self.qualify_column("if_descr");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::String(descr.to_string()));
        self
    }

    /// Filter by chassis_id (for hosts table)
    pub fn chassis_id(mut self, chassis_id: &str) -> Self {
        let col = self.qualify_column("chassis_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::String(chassis_id.to_string()));
        self
    }

    /// Filter by interface_id FK (for if_entries table)
    pub fn interface_id(mut self, interface_id: &Uuid) -> Self {
        let col = self.qualify_column("interface_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*interface_id));
        self
    }

    /// Filter if_entries with unresolved LLDP/CDP neighbors in a network.
    /// Matches entries that have LLDP or CDP data but no neighbor (neither if_entry nor host).
    pub fn unresolved_lldp_in_network(mut self, network_id: Uuid) -> Self {
        let network_col = self.qualify_column("network_id");
        let lldp_chassis_col = self.qualify_column("lldp_chassis_id");
        let cdp_device_col = self.qualify_column("cdp_device_id");
        let cdp_addr_col = self.qualify_column("cdp_address");
        let neighbor_if_entry_col = self.qualify_column("neighbor_if_entry_id");
        let neighbor_host_col = self.qualify_column("neighbor_host_id");

        self.conditions
            .push(format!("{} = ${}", network_col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(network_id));

        // Has LLDP or CDP data but not yet resolved (no neighbor of either type)
        self.conditions.push(format!(
            "({} IS NOT NULL OR {} IS NOT NULL OR {} IS NOT NULL)",
            lldp_chassis_col, cdp_device_col, cdp_addr_col
        ));
        self.conditions
            .push(format!("{} IS NULL", neighbor_if_entry_col));
        self.conditions
            .push(format!("{} IS NULL", neighbor_host_col));

        self
    }

    /// Filter if_entries that have any resolved neighbor (full or partial resolution)
    pub fn has_neighbor(mut self) -> Self {
        let neighbor_if_entry_col = self.qualify_column("neighbor_if_entry_id");
        let neighbor_host_col = self.qualify_column("neighbor_host_id");

        self.conditions.push(format!(
            "({} IS NOT NULL OR {} IS NOT NULL)",
            neighbor_if_entry_col, neighbor_host_col
        ));

        self
    }

    /// Filter if_entries with full neighbor resolution (specific remote port known)
    pub fn has_neighbor_if_entry(mut self) -> Self {
        let col = self.qualify_column("neighbor_if_entry_id");
        self.conditions.push(format!("{} IS NOT NULL", col));
        self
    }

    /// Filter if_entries connected to a specific host (either resolution type)
    pub fn neighbor_host(mut self, host_id: Uuid) -> Self {
        let neighbor_if_entry_col = self.qualify_column("neighbor_if_entry_id");
        let neighbor_host_col = self.qualify_column("neighbor_host_id");

        // Either directly connected to host (partial resolution)
        // Or connected to an if_entry on that host (full resolution)
        // For full resolution, we need a subquery
        self.conditions.push(format!(
            "({} = ${} OR {} IN (SELECT id FROM if_entries WHERE host_id = ${}))",
            neighbor_host_col,
            self.values.len() + 1,
            neighbor_if_entry_col,
            self.values.len() + 1
        ));
        self.values.push(SqlValue::Uuid(host_id));

        self
    }

    // =========================================================================
    // Organization filters
    // =========================================================================

    /// Filter for organizations that haven't been synced to Brevo yet
    pub fn without_brevo_company_id(mut self) -> Self {
        let col = self.qualify_column("brevo_company_id");
        self.conditions.push(format!("{} IS NULL", col));
        self
    }

    /// Filter for organizations that have already been synced to Brevo
    pub fn with_brevo_company_id(mut self) -> Self {
        let col = self.qualify_column("brevo_company_id");
        self.conditions.push(format!("{} IS NOT NULL", col));
        self
    }

    /// Filter for organizations by Stripe customer ID
    pub fn stripe_customer_id(mut self, id: &str) -> Self {
        let col = self.qualify_column("stripe_customer_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::String(id.to_string()));
        self
    }
}
