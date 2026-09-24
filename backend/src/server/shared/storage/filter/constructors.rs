//! Constructor factories (new_*) for common filter scopes.
use super::*;

impl<T: Storable> StorableFilter<T> {
    pub fn new() -> Self {
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

    /// Empty filter (no WHERE conditions). Test-only base for exercising filter
    /// modifiers (`id_or_lineage_in`, `as_of`, `live`, …) in isolation. Not part
    /// of the public API: production callers must start from a scoped
    /// constructor (`new_from_*` / `new_for_*`) so queries can't accidentally
    /// fan out across every tenant.
    #[cfg(test)]
    pub(crate) fn new_unfiltered() -> Self {
        Self::new()
    }

    /// All rows captured under one snapshot (`snapshot_id = $1`). Used by the
    /// snapshot close-and-clone post-pass to re-read the just-cloned closed
    /// copies of a type.
    pub fn new_from_snapshot_id(snapshot_id: &Uuid) -> Self {
        Self::new().snapshot_id(snapshot_id)
    }

    /// Every row of `T`, unscoped — for the daily org-wide retention sweep,
    /// which by definition iterates all organizations. Intentionally unfiltered;
    /// named so the "I really mean everything" intent is explicit at the call
    /// site rather than an ad-hoc empty filter.
    pub fn new_for_retention_sweep() -> Self {
        Self::new()
    }

    /// Every row of `T`, unscoped — for the one-shot Brevo domain-classification
    /// backfill, which iterates all users. Ephemeral release code: remove with
    /// `BrevoService::backfill_domain_classifications` next release.
    pub fn new_for_brevo_backfill() -> Self {
        Self::new()
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

    pub fn new_from_interface_id(ip_address_id: &Uuid) -> Self {
        Self::new().ip_address_id(ip_address_id)
    }

    pub fn new_from_dependency_ids(dependency_ids: &[Uuid]) -> Self {
        Self::new().dependency_ids(dependency_ids)
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

    pub fn new_for_historical_session(session_id: Uuid) -> Self {
        Self::new().historical_session(session_id)
    }

    pub fn new_for_unresolved_fdb_in_network(network_id: Uuid) -> Self {
        Self::new().unresolved_fdb_in_network(network_id)
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

    /// Organizations holding an air-gapped key whose licence period ends in
    /// the given window. Their servers never call home, so a renewal only
    /// reaches them by email.
    pub fn new_with_airgap_key_expiring_between(
        after: DateTime<Utc>,
        before: DateTime<Utc>,
    ) -> Self {
        Self::new()
            .license_key_type(LicenseKeyType::Offline)
            .license_paid_through_between(after, before)
    }

    pub fn new_for_daemon_poller_system_job() -> Self {
        Self::new()
            .daemon_mode(DaemonMode::ServerPoll)
            .is_unreachable(false)
            .standby(false)
    }

    pub fn new_for_active_daemons() -> Self {
        Self::new().standby(false).is_unreachable(false)
    }
}
