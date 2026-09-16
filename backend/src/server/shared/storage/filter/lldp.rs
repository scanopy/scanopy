//! LLDP/FDB neighbor-resolution filters.
use super::*;
use crate::server::interfaces::r#impl::base::if_type::EXCLUDED_IF_TYPES;

impl<T: Storable> StorableFilter<T> {
    // =========================================================================
    // LLDP resolution filters
    // =========================================================================

    /// Filter by IP address (for ip_addresses table)
    pub fn ip_address(mut self, ip: std::net::IpAddr) -> Self {
        let col = self.qualify_column("ip_address");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::IpAddr(ip));
        self
    }

    /// Filter by if_descr (for interfaces table)
    pub fn if_descr(mut self, descr: &str) -> Self {
        let col = self.qualify_column("if_descr");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::String(descr.to_string()));
        self
    }

    /// Filter by if_name (for interfaces table)
    pub fn if_name(mut self, name: &str) -> Self {
        let col = self.qualify_column("if_name");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::String(name.to_string()));
        self
    }

    /// Filter by if_alias (for interfaces table)
    ///
    /// `ifAlias` is the operator-assigned description (`ifXTable`), and on several families it is
    /// the only column carrying the bare port name: the Westermo WeOS switches report
    /// `ifDescr = "100-T eth9"` (media type prefixed) while `ifName` and `ifAlias` both hold
    /// `eth9`. Non-unique by nature — it is user-configurable — so every caller resolves it on a
    /// single match only.
    pub fn if_alias(mut self, alias: &str) -> Self {
        let col = self.qualify_column("if_alias");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::String(alias.to_string()));
        self
    }

    /// Restrict to interfaces that are physical ports, excluding the virtual/software families.
    ///
    /// A device's VLAN and loopback rows routinely repeat the chassis base MAC that no physical
    /// port carries — the customer's Westermo has six `propVirtual` VLAN interfaces sharing
    /// `…02:E0` while all ten physical ports have unique addresses. Counting those rows against a
    /// MAC-uniqueness test makes a lookup ambiguous that no port would have contested, so the test
    /// is scoped to the rows that can actually be the far end of a cable.
    ///
    pub fn physical_if_types(mut self) -> Self {
        let col = self.qualify_column("if_type");
        let start = self.values.len() + 1;
        let placeholders: Vec<String> = (start..start + EXCLUDED_IF_TYPES.len())
            .map(|i| format!("${i}"))
            .collect();
        // `NULL NOT IN (...)` is NULL, not true, so without the null arm every row whose type
        // was never read would be filtered out — silently removing exactly the ports learned from
        // a neighbour's advertisement from MAC-based resolution. Unknown counts as physical: the
        // far end of a cable is a physical port by construction.
        self.conditions.push(format!(
            "({col} IS NULL OR {col} NOT IN ({}))",
            placeholders.join(", ")
        ));
        for if_type in EXCLUDED_IF_TYPES {
            self.values.push(SqlValue::I32(*if_type));
        }
        self
    }

    /// Filter by if_index (for interfaces table)
    pub fn if_index(mut self, if_index: i32) -> Self {
        let col = self.qualify_column("if_index");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::I32(if_index));
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

    /// Filter by sys_name (for hosts table)
    pub fn sys_name(mut self, sys_name: &str) -> Self {
        let col = self.qualify_column("sys_name");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::String(sys_name.to_string()));
        self
    }

    /// Filter by ip_address_id FK (for interfaces table)
    pub fn ip_address_id(mut self, ip_address_id: &Uuid) -> Self {
        let col = self.qualify_column("ip_address_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*ip_address_id));
        self
    }

    /// Filter interfaces with unresolved single-MAC FDB data in a network.
    ///
    /// Matches entries that have exactly 1 learned MAC, no candidate row (raw LLDP/CDP evidence —
    /// FDB is lower-priority than protocol-based discovery), and no existing resolved row of
    /// either kind.
    ///
    /// GH #701: `neighbor_interface_id`/`neighbor_host_id`/`lldp_chassis_id`/`cdp_device_id` moved
    /// off `interfaces` into `interface_neighbor_interfaces`/`interface_neighbor_hosts`/
    /// `interface_neighbor_candidates` — the four `IS NULL` scalar-column checks this filter used
    /// became three `NOT EXISTS` subqueries against those tables (candidates cover both LLDP and
    /// CDP in one check, since a row's evidence type no longer has its own column here).
    pub fn unresolved_fdb_in_network(mut self, network_id: Uuid) -> Self {
        let network_col = self.qualify_column("network_id");
        let fdb_col = self.qualify_column("fdb_macs");
        let id_col = self.qualify_column("id");

        self.conditions
            .push(format!("{} = ${}", network_col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(network_id));

        // Has single-MAC FDB data.
        self.conditions.push(format!(
            "{} IS NOT NULL AND jsonb_array_length({}) = 1",
            fdb_col, fdb_col
        ));
        // No raw LLDP/CDP evidence at all.
        self.conditions.push(format!(
            "NOT EXISTS (SELECT 1 FROM interface_neighbor_candidates \
             WHERE interface_neighbor_candidates.interface_id = {id_col})"
        ));
        // No existing resolved row of either kind.
        self.conditions.push(format!(
            "NOT EXISTS (SELECT 1 FROM interface_neighbor_interfaces \
             WHERE interface_neighbor_interfaces.interface_id = {id_col} \
             AND interface_neighbor_interfaces.valid_to IS NULL)"
        ));
        self.conditions.push(format!(
            "NOT EXISTS (SELECT 1 FROM interface_neighbor_hosts \
             WHERE interface_neighbor_hosts.interface_id = {id_col} \
             AND interface_neighbor_hosts.valid_to IS NULL)"
        ));

        self.live()
    }
}
