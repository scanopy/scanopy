use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    credentials::r#impl::{
        base::Credential,
        junction::{HostCredentialStorage, NetworkCredential, NetworkCredentialStorage},
        mapping::{
            CredentialMapping, CredentialQueryPayload, IntegrationTarget, IpOverride,
            SnmpCredentialMapping, SnmpQueryCredential,
        },
        types::{
            CredentialAssignment, CredentialHostAssignment, CredentialType,
            CredentialTypeDiscriminants, SnmpVersion, Target,
        },
    },
    hosts::{r#impl::base::Host, service::HostService},
    ip_addresses::{r#impl::base::IPAddress, service::IPAddressService},
    networks::service::NetworkService,
    organizations::service::OrganizationService,
    shared::{
        events::{
            bus::EventBus,
            traits::{Event, OrgScope},
            types::{OnboardingOperation, OnboardingOperationDiscriminants},
        },
        services::traits::{CrudService, EventBusService},
        storage::{filter::StorableFilter, generic::GenericPostgresStorage},
    },
    tags::entity_tags::EntityTagService,
};
use anyhow::Error;
use async_trait::async_trait;
use std::collections::{BTreeMap, HashSet};
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, OnceLock};
use strum::IntoDiscriminant;
use uuid::Uuid;

/// The set of things a credential targets, normalized across the network and host
/// junction tables so two credentials can be tested for overlap. Used to enforce
/// the single-endpoint invariant — see
/// [`CredentialService::find_single_endpoint_conflict`].
#[derive(Debug, Default)]
struct CredentialTargets {
    networks: HashSet<Uuid>,
    /// host_id → the IPs it is scoped to (`None` = the whole host).
    hosts: Vec<(Uuid, Option<HashSet<Uuid>>)>,
}

impl CredentialTargets {
    fn build(networks: &[Uuid], host_assignments: &[CredentialHostAssignment]) -> Self {
        Self {
            networks: networks.iter().copied().collect(),
            hosts: host_assignments
                .iter()
                .map(|h| {
                    (
                        h.host_id,
                        h.ip_address_ids
                            .as_ref()
                            .map(|ids| ids.iter().copied().collect()),
                    )
                })
                .collect(),
        }
    }

    /// A credential targeting exactly one whole host (e.g. a daemon-host target).
    fn for_host(host_id: Uuid) -> Self {
        Self {
            hosts: vec![(host_id, None)],
            ..Default::default()
        }
    }

    /// Two target sets overlap if they share a network, a target IP, or a host —
    /// where a shared host overlaps when either side covers the whole host or
    /// their IP scopes intersect.
    fn overlaps(&self, other: &Self) -> bool {
        if !self.networks.is_disjoint(&other.networks) {
            return true;
        }
        self.hosts.iter().any(|(h1, s1)| {
            other.hosts.iter().any(|(h2, s2)| {
                h1 == h2
                    && match (s1, s2) {
                        // One side covers the whole host → always overlaps.
                        (None, _) | (_, None) => true,
                        (Some(a), Some(b)) => !a.is_disjoint(b),
                    }
            })
        })
    }
}

/// The single-endpoint rule between two credentials: they conflict when both
/// belong to the same single-endpoint integration (e.g. both Docker, or both
/// Podman) and their targets overlap. This is the one rule shared by the
/// credential-write check ([`CredentialService::find_single_endpoint_conflict`])
/// and the discovery-update check ([`CredentialService::find_daemon_host_target_conflict`]);
/// the two callers differ only in the targets they feed in.
fn single_endpoint_targets_conflict(
    a_type: &CredentialType,
    a_targets: &CredentialTargets,
    b_type: &CredentialType,
    b_targets: &CredentialTargets,
) -> bool {
    a_type.single_endpoint_per_host()
        && b_type.single_endpoint_per_host()
        && ServiceDefinition::name(&*a_type.associated_service())
            == ServiceDefinition::name(&*b_type.associated_service())
        && a_targets.overlaps(b_targets)
}

pub struct CredentialService {
    storage: Arc<GenericPostgresStorage<Credential>>,
    event_bus: Arc<EventBus>,
    entity_tag_service: Arc<EntityTagService>,
    #[allow(dead_code)]
    network_service: Arc<NetworkService>,
    ip_address_service: Arc<IPAddressService>,
    organization_service: Arc<OrganizationService>,
    host_service: OnceLock<Arc<HostService>>,
    network_credential_storage: NetworkCredentialStorage,
    host_credential_storage: HostCredentialStorage,
}

impl EventBusService<Credential> for CredentialService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, _entity: &Credential) -> Option<Uuid> {
        None
    }

    fn get_organization_id(&self, entity: &Credential) -> Option<Uuid> {
        Some(entity.base.organization_id)
    }
}

#[async_trait]
impl CrudService<Credential> for CredentialService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Credential>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        Some(&self.entity_tag_service)
    }

    async fn create(
        &self,
        entity: Credential,
        authentication: AuthenticatedEntity,
    ) -> Result<Credential, Error> {
        entity.base.validate_settings()?;

        let created = self.create_base(entity, authentication.clone()).await?;

        // Emit onboarding events for credential creation
        let organization_id = created.base.organization_id;
        if let Some(organization) = self
            .organization_service
            .get_by_id(&organization_id)
            .await?
        {
            // Generic event for any credential type
            if organization.not_onboarded(&OnboardingOperationDiscriminants::FirstCredentialCreated)
            {
                self.event_bus
                    .publish(Event::new(
                        OrgScope { organization_id },
                        OnboardingOperation::FirstCredentialCreated,
                        authentication.clone(),
                    ))
                    .await?;
            }

            // SNMP-specific event (preserves existing Brevo tracking) — any SNMP version counts.
            if matches!(
                created.base.credential_type,
                CredentialType::SnmpV1 { .. }
                    | CredentialType::SnmpV2c { .. }
                    | CredentialType::SnmpV3 { .. }
            ) && organization
                .not_onboarded(&OnboardingOperationDiscriminants::FirstSnmpCredentialCreated)
            {
                self.event_bus
                    .publish(Event::new(
                        OrgScope { organization_id },
                        OnboardingOperation::FirstSnmpCredentialCreated,
                        authentication,
                    ))
                    .await?;
            }
        }

        Ok(created)
    }
}

impl CredentialService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        storage: Arc<GenericPostgresStorage<Credential>>,
        event_bus: Arc<EventBus>,
        entity_tag_service: Arc<EntityTagService>,
        network_service: Arc<NetworkService>,
        ip_address_service: Arc<IPAddressService>,
        organization_service: Arc<OrganizationService>,
        pool: sqlx::PgPool,
    ) -> Self {
        Self {
            storage,
            event_bus,
            entity_tag_service,
            network_service,
            ip_address_service,
            organization_service,
            host_service: OnceLock::new(),
            network_credential_storage: NetworkCredentialStorage::new(pool.clone()),
            host_credential_storage: HostCredentialStorage::new(pool),
        }
    }

    /// Set the host service dependency after construction (breaks circular dep).
    pub fn set_host_service(&self, service: Arc<HostService>) -> Result<(), Arc<HostService>> {
        self.host_service.set(service)
    }

    // ========================================================================
    // Junction table methods — delegates to typed storage
    // ========================================================================

    /// Get credential IDs for a network from the junction table.
    pub async fn get_credential_ids_for_network(
        &self,
        network_id: &Uuid,
    ) -> Result<Vec<Uuid>, Error> {
        self.network_credential_storage
            .get_credential_ids_for_network(network_id)
            .await
    }

    /// Get credential IDs for multiple networks (batch).
    pub async fn get_credential_ids_for_networks(
        &self,
        network_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, Vec<Uuid>>, Error> {
        self.network_credential_storage
            .get_credential_ids_for_networks(network_ids)
            .await
    }

    /// Get credential assignments for a host from the junction table.
    pub async fn get_credential_assignments_for_host(
        &self,
        host_id: &Uuid,
    ) -> Result<Vec<CredentialAssignment>, Error> {
        self.host_credential_storage
            .get_assignments_for_host(host_id)
            .await
    }

    /// Get credential assignments for multiple hosts (batch).
    pub async fn get_credential_assignments_for_hosts(
        &self,
        host_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, Vec<CredentialAssignment>>, Error> {
        self.host_credential_storage
            .get_assignments_for_hosts(host_ids)
            .await
    }

    /// Replace all credentials for a network (atomic).
    pub async fn set_network_credentials(
        &self,
        network_id: &Uuid,
        credential_ids: &[Uuid],
    ) -> Result<(), Error> {
        self.network_credential_storage
            .save_for_network(network_id, credential_ids)
            .await
    }

    /// Bulk-insert many network↔credential assignments at once, skipping the
    /// per-network lock/delete. For seed paths on a freshly reset org. Each
    /// pair is `(network_id, credential_id)`.
    pub async fn create_network_credentials(&self, pairs: &[(Uuid, Uuid)]) -> Result<(), Error> {
        let records: Vec<NetworkCredential> = pairs
            .iter()
            .map(|&(network_id, credential_id)| NetworkCredential {
                network_id,
                credential_id,
            })
            .collect();
        self.network_credential_storage.create_many(&records).await
    }

    /// Merge `incoming` credentials into a network's junction (additive — never prunes).
    ///
    /// The network-side counterpart of [`Self::merge_host_credentials`], for promoting a
    /// discovery's one-shot `IntegrationTarget::Network` targets into the durable network-wide
    /// channel when the scan completes. Replacing would delete credentials assigned from the
    /// networks page or the credential modal, which this path knows nothing about.
    pub async fn merge_network_credentials(
        &self,
        network_id: &Uuid,
        incoming: &[Uuid],
    ) -> Result<(), Error> {
        let existing = self.get_credential_ids_for_network(network_id).await?;
        let mut merged = existing;
        for credential_id in incoming {
            if !merged.contains(credential_id) {
                merged.push(*credential_id);
            }
        }
        self.set_network_credentials(network_id, &merged).await
    }

    /// Replace all credential assignments for a host (atomic).
    pub async fn set_host_credentials(
        &self,
        host_id: &Uuid,
        assignments: &[CredentialAssignment],
    ) -> Result<(), Error> {
        self.host_credential_storage
            .save_for_host(host_id, assignments)
            .await
    }

    /// Merge `incoming` credential assignments into a host's junction (additive — never prunes).
    ///
    /// Discovery self-reports only the credentials that probed successfully on a host. Using the
    /// replace path (`set_host_credentials`) there would delete user/init-assigned daemon-host
    /// credentials that didn't probe this scan (e.g. a Docker socket on a Podman-only host),
    /// regressing the junction-as-source-of-truth model. Merge keeps existing assignments and
    /// overlays `incoming` (incoming wins per `credential_id`, refreshing `ip_address_ids`), so
    /// discovery only adds. Explicit user edits still go through the replace path.
    pub async fn merge_host_credentials(
        &self,
        host_id: &Uuid,
        incoming: &[CredentialAssignment],
    ) -> Result<(), Error> {
        let mut by_host = self
            .get_credential_assignments_for_hosts(&[*host_id])
            .await?;
        let existing = by_host.remove(host_id).unwrap_or_default();

        let incoming_ids: std::collections::HashSet<Uuid> =
            incoming.iter().map(|a| a.credential_id).collect();
        let mut merged: Vec<CredentialAssignment> = existing
            .into_iter()
            .filter(|a| !incoming_ids.contains(&a.credential_id))
            .collect();
        merged.extend(incoming.iter().cloned());

        self.set_host_credentials(host_id, &merged).await
    }

    /// Get the network IDs a credential is assigned to (reverse lookup).
    pub async fn get_network_ids_for_credential(
        &self,
        credential_id: &Uuid,
    ) -> Result<Vec<Uuid>, Error> {
        self.network_credential_storage
            .get_network_ids_for_credential(credential_id)
            .await
    }

    /// Get the network IDs for multiple credentials (batch, reverse lookup).
    pub async fn get_network_ids_for_credentials(
        &self,
        credential_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, Vec<Uuid>>, Error> {
        self.network_credential_storage
            .get_network_ids_for_credentials(credential_ids)
            .await
    }

    /// Get the host assignments for a credential (reverse lookup).
    pub async fn get_host_assignments_for_credential(
        &self,
        credential_id: &Uuid,
    ) -> Result<Vec<CredentialHostAssignment>, Error> {
        self.host_credential_storage
            .get_host_assignments_for_credential(credential_id)
            .await
    }

    /// Get the host assignments for multiple credentials (batch, reverse lookup).
    pub async fn get_host_assignments_for_credentials(
        &self,
        credential_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, Vec<CredentialHostAssignment>>, Error> {
        self.host_credential_storage
            .get_host_assignments_for_credentials(credential_ids)
            .await
    }

    /// Replace the full set of networks a credential is assigned to (atomic).
    pub async fn set_credential_networks(
        &self,
        credential_id: &Uuid,
        network_ids: &[Uuid],
    ) -> Result<(), Error> {
        self.network_credential_storage
            .save_networks_for_credential(credential_id, network_ids)
            .await
    }

    /// Replace the full set of host assignments for a credential (atomic).
    pub async fn set_credential_host_assignments(
        &self,
        credential_id: &Uuid,
        assignments: &[CredentialHostAssignment],
    ) -> Result<(), Error> {
        self.host_credential_storage
            .save_host_assignments_for_credential(credential_id, assignments)
            .await
    }

    /// Enforce the single-endpoint-per-host invariant: integrations whose
    /// credential type returns `single_endpoint_per_host()` (e.g. Docker) resolve
    /// to exactly one endpoint per host, so two credentials of that integration
    /// must not target an overlapping network, host, or IP (incl. the daemon host
    /// at 127.0.0.1). Returns the name of the first conflicting credential, if any.
    ///
    /// `candidate` carries its intended assignments in `base` (network ids, host
    /// assignments) — call this before persisting. Try-many
    /// integrations (e.g. SNMP, multiple communities per network) are unconstrained.
    pub async fn find_single_endpoint_conflict(
        &self,
        candidate: &Credential,
    ) -> Result<Option<String>, Error> {
        let ct = &candidate.base.credential_type;
        if !ct.single_endpoint_per_host() {
            return Ok(None);
        }
        let integration = ServiceDefinition::name(&*ct.associated_service());
        let org_id = candidate.base.organization_id;

        // Other credentials of the same integration in this org.
        let filter = StorableFilter::<Credential>::new_from_org_id(&org_id);
        let others: Vec<Credential> = self
            .get_all(filter)
            .await?
            .into_iter()
            .filter(|c| c.id != candidate.id)
            .filter(|c| {
                ServiceDefinition::name(&*c.base.credential_type.associated_service())
                    == integration
            })
            .collect();
        if others.is_empty() {
            return Ok(None);
        }

        // Hydrate the others' junction-backed assignments.
        let other_ids: Vec<Uuid> = others.iter().map(|c| c.id).collect();
        let net_map = self.get_network_ids_for_credentials(&other_ids).await?;
        let host_map = self
            .get_host_assignments_for_credentials(&other_ids)
            .await?;

        let cand = CredentialTargets::build(
            &candidate.base.assigned_network_ids,
            &candidate.base.host_assignments,
        );
        for other in &others {
            let empty_nets = Vec::new();
            let empty_hosts = Vec::new();
            let other_targets = CredentialTargets::build(
                net_map.get(&other.id).unwrap_or(&empty_nets),
                host_map.get(&other.id).unwrap_or(&empty_hosts),
            );
            if single_endpoint_targets_conflict(
                ct,
                &cand,
                &other.base.credential_type,
                &other_targets,
            ) {
                return Ok(Some(other.base.name.clone()));
            }
        }
        Ok(None)
    }

    /// Discovery-update counterpart to [`Self::find_single_endpoint_conflict`]: a
    /// daemon's `Discovery.integration_targets` all target that one daemon host, so
    /// two single-endpoint credentials of the same integration both targeting the
    /// daemon host (e.g. a Docker socket and a Docker proxy, or the Podman pair)
    /// conflict. Returns the first conflicting pair of credential names.
    ///
    /// Feeds the same [`single_endpoint_targets_conflict`] rule the credential-write
    /// path uses — only the targets differ (here, each daemon-host credential is
    /// modeled as targeting `daemon_host_id`).
    pub async fn find_daemon_host_target_conflict(
        &self,
        daemon_host_id: Uuid,
        integration_targets: &[IntegrationTarget],
    ) -> Result<Option<(String, String)>, Error> {
        // Credentials that target the daemon's own host: an explicit DaemonHost
        // scope, or a Hosts scope whose IPs are all loopback.
        let host_cred_ids: Vec<Uuid> = integration_targets
            .iter()
            .filter_map(|t| match t {
                IntegrationTarget::DaemonHost { credential_id } => Some(*credential_id),
                IntegrationTarget::Hosts { credential_id, ips }
                    if !ips.is_empty() && ips.iter().all(|ip| ip.is_loopback()) =>
                {
                    Some(*credential_id)
                }
                _ => None,
            })
            .collect();

        // Resolve each to (name, type), modeled as targeting the daemon host.
        let mut items: Vec<(String, CredentialType, CredentialTargets)> = Vec::new();
        for cred_id in host_cred_ids {
            if let Some(cred) = self.get_by_id(&cred_id).await? {
                items.push((
                    cred.base.name,
                    cred.base.credential_type,
                    CredentialTargets::for_host(daemon_host_id),
                ));
            }
        }

        for i in 0..items.len() {
            for j in (i + 1)..items.len() {
                if single_endpoint_targets_conflict(
                    &items[i].1,
                    &items[i].2,
                    &items[j].1,
                    &items[j].2,
                ) {
                    return Ok(Some((items[i].0.clone(), items[j].0.clone())));
                }
            }
        }
        Ok(None)
    }

    // ========================================================================
    // Discovery credential building
    // ========================================================================

    // === Legacy Daemon Support (pre-v0.15.0) ===

    /// Legacy: Supports daemons < v0.15.0 using SnmpCredentialMapping in DiscoveryType::Network.
    /// Modern equivalent: `build_credential_mappings_for_discovery()` with CredentialQueryPayload.
    /// Remove when minimum daemon version >= 0.15.0.
    pub async fn build_snmp_credentials_for_discovery(
        &self,
        network_id: Uuid,
    ) -> Result<SnmpCredentialMapping, Error> {
        let host_service = self
            .host_service
            .get()
            .ok_or_else(|| anyhow::anyhow!("HostService not initialized"))?;
        let host_filter = StorableFilter::<Host>::new_from_network_ids(&[network_id]);
        let hosts = host_service.get_all(host_filter).await?;

        let interface_filter = StorableFilter::<IPAddress>::new_from_network_ids(&[network_id]);
        let ip_addresses = self.ip_address_service.get_all(interface_filter).await?;

        // Get network's SNMP credentials (from junction table)
        let network_cred_ids = self.get_credential_ids_for_network(&network_id).await?;
        tracing::debug!(
            network_id = %network_id,
            credential_count = network_cred_ids.len(),
            "Credential IDs found for network via junction table"
        );
        // Legacy mapping only carries SNMPv2c — pre-v0.15.0 daemons can't speak
        // v1 or v3. v1/v3 credentials reach modern daemons via
        // build_all_credential_mappings.
        let mut network_snmp_credential: Option<SnmpQueryCredential> = None;
        for cred_id in &network_cred_ids {
            if let Some(cred) = self.get_by_id(cred_id).await?
                && let CredentialQueryPayload::Snmp(snmp) =
                    cred.base.credential_type.to_query_payload()
                && snmp.version == SnmpVersion::V2c
            {
                network_snmp_credential = Some(snmp);
                break;
            }
        }
        tracing::debug!(
            network_id = %network_id,
            has_default = network_snmp_credential.is_some(),
            "Network default SNMP credential resolution"
        );

        // Get host-level SNMP credential overrides
        let host_ids: Vec<Uuid> = hosts.iter().map(|h| h.id).collect();
        let host_cred_map = self.get_credential_assignments_for_hosts(&host_ids).await?;

        let mut overrides: Vec<IpOverride<SnmpQueryCredential>> = Vec::new();

        for host in &hosts {
            if let Some(assignments) = host_cred_map.get(&host.id) {
                for assignment in assignments {
                    if let Some(cred) = self.get_by_id(&assignment.credential_id).await?
                        && let CredentialQueryPayload::Snmp(query_cred) =
                            cred.base.credential_type.to_query_payload()
                        && query_cred.version == SnmpVersion::V2c
                    {
                        // If ip_address_ids is set, only create overrides for those ip_addresses
                        let relevant_interfaces: Vec<_> = ip_addresses
                            .iter()
                            .filter(|i| {
                                i.base.host_id == host.id
                                    && match &assignment.ip_address_ids {
                                        Some(ids) => ids.contains(&i.id),
                                        None => true,
                                    }
                            })
                            .collect();
                        overrides.extend(relevant_interfaces.iter().map(|i| IpOverride {
                            ip: i.base.ip_address,
                            credential: query_cred.clone(),
                            credential_id: cred.id,
                            // Legacy pre-v0.15.0 path, serving daemons that predate host-aware
                            // grouping — they get the per-address rule they have always had.
                            host_id: None,
                            mac_address: None,
                        }));
                        break;
                    }
                }
            }
        }

        tracing::debug!(
            network_id = %network_id,
            ip_overrides = overrides.len(),
            has_default = network_snmp_credential.is_some(),
            "SNMP credential mapping built for discovery"
        );

        Ok(SnmpCredentialMapping {
            daemon_os: Default::default(),
            default_credential: network_snmp_credential,
            // Legacy pre-v0.15.0 path: this mapping is built per network rather than per
            // credential, so there is no single id for its default to carry, and the daemons it
            // serves predate coded warnings anyway.
            default_credential_id: None,
            ip_overrides: overrides,
        })
    }

    // === End Legacy Daemon Support ===

    /// Build generic credential mappings for unified discovery dispatch.
    /// Returns one `CredentialMapping<CredentialQueryPayload>` per credential type discriminant.
    ///
    /// Combines: network-level credentials (broadcast defaults), host-level credential
    /// assignments (IP overrides on discovered hosts), and the per-daemon `integration_targets`
    /// from the daemon's `Discovery` (init-command targeting — both credentialed cred↔IP and
    /// credential-less local sockets). The `integration_targets` source replaces the old global
    /// `credential.target_ips` org-wide bootstrap (which was consumed/cleared once, racing across
    /// daemons — #637) and the discovery modal's one-shot `pending_credential_ids`.
    pub async fn build_all_credential_mappings(
        &self,
        network_id: Uuid,
        integration_targets: &[IntegrationTarget],
        daemon_version: Option<&semver::Version>,
    ) -> Result<Vec<CredentialMapping<CredentialQueryPayload>>, Error> {
        let host_service = self
            .host_service
            .get()
            .ok_or_else(|| anyhow::anyhow!("HostService not initialized"))?;

        // Fetch hosts + ip_addresses on network
        let host_filter = StorableFilter::<Host>::new_from_network_ids(&[network_id]);
        let hosts = host_service.get_all(host_filter).await?;

        let interface_filter = StorableFilter::<IPAddress>::new_from_network_ids(&[network_id]);
        let ip_addresses = self.ip_address_service.get_all(interface_filter).await?;

        // Defense-in-depth: only ever hand a daemon credentials owned by the
        // scanned network's own organization. Write-time validation already
        // rejects cross-org credential↔network/host assignments, but a stale or
        // otherwise-inconsistent junction row must never serialize a foreign
        // tenant's secret onto the wire. If the network's org can't be resolved
        // we fall back to the write-time guard rather than break a live scan.
        let network_org_id = self
            .network_service
            .get_by_id(&network_id)
            .await?
            .map(|n| n.base.organization_id);
        let owned_by_network_org = |cred: &Credential| -> bool {
            network_org_id
                .map(|org| cred.base.organization_id == org)
                .unwrap_or(true)
        };

        // Fetch network-level credentials
        let network_cred_ids = self.get_credential_ids_for_network(&network_id).await?;

        // One mapping per *credential*, not per credential type. Keying by type meant a network
        // assigned two SNMPv2c communities dispatched only the lower-UUID one and silently dropped
        // the other, leaving every device that answers to the dropped community unscanned. Two
        // communities is ordinary practice (per site, per vendor), and the daemon already expects
        // several mappings sharing one wire discriminant — it appends exactly such a sibling
        // itself (the injected "public" default) and dedups them at execute time.
        //
        // BTreeMap so the emitted order is stable across runs, and keyed the same way the source
        // rows are ordered (`credential_id ASC`).
        let mut mappings_by_credential: BTreeMap<Uuid, TypedCredentialMapping> = BTreeMap::new();

        for cred_id in &network_cred_ids {
            if let Some(cred) = self.get_by_id(cred_id).await?
                && owned_by_network_org(&cred)
            {
                let cred_type = &cred.base.credential_type;
                let mapping = mapping_for(&mut mappings_by_credential, cred.id, cred_type);
                mapping.default_credential = Some(cred_type.to_query_payload());
                mapping.daemon_os = cred.base.daemon_os;
            }
        }

        // Fetch host-level credential assignments
        let host_ids: Vec<Uuid> = hosts.iter().map(|h| h.id).collect();
        let host_cred_map = self.get_credential_assignments_for_hosts(&host_ids).await?;

        for host in &hosts {
            if let Some(assignments) = host_cred_map.get(&host.id) {
                for assignment in assignments {
                    if let Some(cred) = self.get_by_id(&assignment.credential_id).await?
                        && owned_by_network_org(&cred)
                    {
                        apply_host_assignment(
                            &mut mappings_by_credential,
                            host.id,
                            assignment,
                            &cred,
                            &ip_addresses,
                        );
                    }
                }
            }
        }

        // Per-daemon integration targets from this daemon's Discovery (init-command targeting).
        // The single home for cred↔IP and local-socket targeting — it replaces the old org-wide
        // target_ips bootstrap and the modal's pending_credential_ids. Resolving the credential is
        // the only I/O; override-building is delegated to the pure `apply_integration_target`.
        for target in integration_targets {
            let Some(cred) = self.get_by_id(&target.credential_id()).await? else {
                tracing::warn!(
                    credential_id = %target.credential_id(),
                    "Integration target references unknown credential; skipping"
                );
                continue;
            };
            apply_integration_target(
                &mut mappings_by_credential,
                target,
                &cred.base.credential_type,
            );
            if let Some(typed) = mappings_by_credential.get_mut(&cred.id) {
                typed.mapping.daemon_os = cred.base.daemon_os;
            }
        }

        // Version gate: never dispatch a credential mapping the target daemon can't
        // deserialize. Filter on the `CredentialType` discriminant (kept alongside each
        // mapping, 7-way) BEFORE the lossy collapse to the 5-way `CredentialQueryPayload`
        // wire tag — SnmpV1/V3 have a higher floor than SnmpV2c. A missing version is
        // treated conservatively (keep only the 0.16.2 wire floor). This protects the
        // installed base of already-released daemons that predate `serde(other)`.
        retain_daemon_compatible(&mut mappings_by_credential, daemon_version);

        Ok(order_mappings_for_dispatch(mappings_by_credential))
    }
}

/// A credential mapping plus the 7-way credential type it came from.
///
/// The wire payload collapses SnmpV1/V2c/V3 into one `Snmp` tag, but the daemon-version floor
/// differs between them, so the version gate has to run before that collapse — which means the
/// discriminant has to survive alongside the mapping.
pub(crate) struct TypedCredentialMapping {
    pub discriminant: CredentialTypeDiscriminants,
    pub mapping: CredentialMapping<CredentialQueryPayload>,
}

impl TypedCredentialMapping {
    pub(crate) fn new(discriminant: CredentialTypeDiscriminants) -> Self {
        Self {
            discriminant,
            mapping: CredentialMapping {
                daemon_os: Default::default(),
                default_credential: None,
                // Stamped from the accumulator key in `order_mappings_for_dispatch`, which is the
                // one place that knows which credential this mapping belongs to.
                default_credential_id: None,
                ip_overrides: vec![],
            },
        }
    }
}

/// The mapping for one credential, created on first use.
///
/// The single place the accumulator is keyed, shared by the network-default, host-override and
/// integration-target paths so they cannot drift back apart.
pub(crate) fn mapping_for<'a>(
    mappings: &'a mut BTreeMap<Uuid, TypedCredentialMapping>,
    credential_id: Uuid,
    credential_type: &CredentialType,
) -> &'a mut CredentialMapping<CredentialQueryPayload> {
    &mut mappings
        .entry(credential_id)
        .or_insert_with(|| TypedCredentialMapping::new(credential_type.discriminant()))
        .mapping
}

/// Drop any mapping the target daemon is too old to deserialize.
///
/// Filters on the 7-way `CredentialTypeDiscriminants` rather than the 5-way wire tag, because
/// SnmpV1 and SnmpV3 carry a higher daemon floor than SnmpV2c but all three collapse to one `Snmp`
/// tag on the wire — gating after that collapse could not tell them apart.
/// Configuring an incompatible credential is already refused at the API with an actionable 400,
/// but only for a discovery's `integration_targets`. This set also carries network credentials and
/// host assignments, which are network-scoped and cannot be validated against one daemon version
/// because a network may hold several daemons on different ones. So reaching here means a
/// mixed-version fleet or a downgrade, where the credential silently does nothing on this daemon
/// until it is upgraded — worth saying out loud, since a silent filter is indistinguishable from
/// a credential that simply never worked.
pub(crate) fn retain_daemon_compatible(
    mappings: &mut BTreeMap<Uuid, TypedCredentialMapping>,
    daemon_version: Option<&semver::Version>,
) {
    mappings.retain(|credential_id, typed| {
        let compatible = typed.discriminant.compatible_with_daemon(daemon_version);
        if !compatible {
            tracing::warn!(
                %credential_id,
                credential_type = %typed.discriminant.display_name(),
                minimum_daemon_version = %typed.discriminant.minimum_daemon_version(),
                daemon_version = daemon_version.map(|v| v.to_string()).unwrap_or_else(|| "unknown".to_string()),
                "Not dispatching a credential this daemon is too old to use; upgrade the daemon \
                 or the credential will keep being skipped on its scans"
            );
        }
        compatible
    });
}

/// Flatten the per-credential mappings into the order the daemon should probe them.
///
/// Mappings carrying IP overrides are emitted **last**, and this is load-bearing. Within a single
/// mapping the daemon tries overrides before the default and stops at the first success, so a
/// host-specific credential beats a network-wide one. Across mappings the rule inverts — the *last*
/// successful probe is recorded as that host's working credential — so splitting one-mapping-per-type
/// into one-per-credential would have flipped that precedence and started attributing hosts to a
/// network-wide credential instead of the specific one assigned to them.
///
/// Ties are broken by credential id (the `BTreeMap` order), so dispatch order is reproducible
/// between scans rather than depending on hash iteration.
fn order_mappings_for_dispatch(
    mappings: BTreeMap<Uuid, TypedCredentialMapping>,
) -> Vec<CredentialMapping<CredentialQueryPayload>> {
    let (broadcast_only, with_overrides): (Vec<_>, Vec<_>) = mappings
        .into_iter()
        .map(|(credential_id, typed)| {
            // The key is the credential this whole mapping belongs to, and dropping it here is
            // what used to leave a broadcast default unattributable on the daemon. Stamped rather
            // than set at build time so all three accumulator paths — network default, host
            // override, integration target — get it from the one place they are keyed by.
            let mut mapping = typed.mapping;
            if mapping.default_credential.is_some() {
                mapping.default_credential_id = Some(credential_id);
            }
            mapping
        })
        .partition(|mapping| mapping.ip_overrides.is_empty());

    broadcast_only.into_iter().chain(with_overrides).collect()
}

/// Apply one [`IntegrationTarget`] to the per-credential-type mapping accumulator.
///
/// Pure (no I/O): the caller resolves the credential by `target.credential_id()`. The target's
/// scope (its [`Target`] discriminant) must be permitted by the credential type's `targets()`,
/// otherwise it is skipped. Every target carries a real credential id — a local socket is just a
/// credential whose type targets only the daemon host, so there is no nil sentinel.
///
/// Idempotent — applying the same target twice does not duplicate overrides. That matters within
/// a single dispatch, where a credential can arrive both as a target and as the host assignment it
/// already earned, and it makes the transitional scan (target still present, assignment written)
/// produce exactly one override.
pub(crate) fn apply_integration_target(
    mappings_by_credential: &mut BTreeMap<Uuid, TypedCredentialMapping>,
    target: &IntegrationTarget,
    credential_type: &CredentialType,
) {
    let scope = Target::from(target);
    if !credential_type.targets().contains(&scope) {
        tracing::warn!(
            ?scope,
            credential_id = %target.credential_id(),
            "Integration target scope not permitted by its credential type; skipping"
        );
        return;
    }

    // Every target is offered to the daemon and earns its host assignment only by probing
    // successfully — nothing is assigned up front. A daemon-host target is reached over the
    // loopback, so it becomes a 127.0.0.1 override like any other IP target.
    let target_ips: Vec<IpAddr> = match target {
        IntegrationTarget::DaemonHost { .. } => vec![IpAddr::V4(Ipv4Addr::LOCALHOST)],
        IntegrationTarget::Hosts { ips, .. } => ips.clone(),
        IntegrationTarget::Network { .. } => Vec::new(),
    };
    if matches!(target, IntegrationTarget::Hosts { .. }) && target_ips.is_empty() {
        return;
    }

    let payload = credential_type.to_query_payload();
    let credential_id = target.credential_id();
    // Keyed by credential, so a second `Network` target of the same type no longer displaces the
    // first — the same collapse that dropped network-assigned credentials applied here too.
    let mapping = mapping_for(mappings_by_credential, credential_id, credential_type);

    match target {
        IntegrationTarget::Network { .. } => {
            mapping.default_credential = Some(payload);
        }
        IntegrationTarget::Hosts { .. } | IntegrationTarget::DaemonHost { .. } => {
            for ip in target_ips {
                push_unique_override(mapping, ip, payload.clone(), credential_id);
            }
        }
    }
}

/// Apply one host credential assignment to the per-credential mapping accumulator.
///
/// Pure (no I/O): the caller resolves the credential and supplies the network's addresses.
/// `ip_address_ids` scopes the assignment to specific addresses on the host; `None` means the
/// whole host (how SNMP reports its assignments, and the fallback when a daemon-reported
/// address can't be resolved — see `remap_assignment_ip_ids`).
///
/// This is the channel a promoted credential is dispatched through on every later scan, once
/// the discovery's one-shot `integration_targets` are consumed. In particular a Docker/Podman
/// socket credential assigned to the daemon host's `127.0.0.1` re-emerges here as a loopback
/// override, which is what the daemon's localhost-integration phase selects on.
pub(crate) fn apply_host_assignment(
    mappings_by_credential: &mut BTreeMap<Uuid, TypedCredentialMapping>,
    host_id: Uuid,
    assignment: &CredentialAssignment,
    credential: &Credential,
    ip_addresses: &[IPAddress],
) {
    let cred_type = &credential.base.credential_type;
    let payload = cred_type.to_query_payload();
    let mapping = mapping_for(mappings_by_credential, credential.id, cred_type);
    mapping.daemon_os = credential.base.daemon_os;

    let relevant_interfaces = ip_addresses.iter().filter(|i| {
        if i.base.host_id != host_id {
            return false;
        }
        match &assignment.ip_address_ids {
            // Scoped to named addresses: honour exactly what was named, loopback included. That
            // is how a promoted Docker/Podman socket credential comes back out on the daemon
            // host's `127.0.0.1` — see `promoted_socket_credential_is_dispatched_over_loopback`.
            Some(ids) => ids.contains(&i.id),
            // Host-wide: every address the host holds, except a loopback one. A remote device's
            // own `127.0.0.1` is an interface it reports, not a way to reach it — and it is not
            // even unique, since every device reports the same one. Swept up here it produced a
            // credential attempt against the daemon's own machine and an "unreachable" finding
            // for a credential that was working fine at the host's real address.
            //
            // Only the incidental case is dropped: naming a loopback address explicitly still
            // targets it, which is the difference between a swept-up interface and an intent.
            None => !i.base.ip_address.is_loopback(),
        }
    });

    mapping
        .ip_overrides
        .extend(relevant_interfaces.map(|i| IpOverride {
            ip: i.base.ip_address,
            credential: payload.clone(),
            credential_id: credential.id,
            // The device these addresses belong to, so the daemon can tell a multi-homed host's
            // addresses apart from unrelated ones.
            host_id: Some(host_id),
            // What Wake-on-LAN addresses: a sleeping host answers to no IP.
            mac_address: i.base.mac_address.as_ref().map(|m| m.value().0),
        }));
}

/// Push an IP-override unless an identical `(ip, credential_id)` override is already present
/// (de-dups against host-assignment overrides and repeated scans).
fn push_unique_override(
    mapping: &mut CredentialMapping<CredentialQueryPayload>,
    ip: IpAddr,
    payload: CredentialQueryPayload,
    credential_id: Uuid,
) {
    if !mapping
        .ip_overrides
        .iter()
        .any(|o| o.ip == ip && o.credential_id == credential_id)
    {
        mapping.ip_overrides.push(IpOverride {
            ip,
            credential: payload,
            credential_id,
            // An integration target names addresses directly; there is no host behind it to
            // group its siblings by.
            host_id: None,
            mac_address: None,
        });
    }
}

#[cfg(test)]
mod single_endpoint_tests {
    use super::*;
    use crate::server::credentials::r#impl::types::CredentialHostAssignment;

    fn host(id: Uuid, ips: Option<Vec<Uuid>>) -> CredentialHostAssignment {
        CredentialHostAssignment {
            host_id: id,
            ip_address_ids: ips,
        }
    }

    #[test]
    fn disjoint_targets_do_not_overlap() {
        let a = CredentialTargets::build(&[Uuid::nil()], &[]);
        let b = CredentialTargets::build(&[Uuid::from_u128(9)], &[]);
        assert!(!a.overlaps(&b));
    }

    #[test]
    fn shared_network_overlaps() {
        let net = Uuid::from_u128(1);
        let a = CredentialTargets::build(&[net], &[]);
        let b = CredentialTargets::build(&[net], &[]);
        assert!(a.overlaps(&b));
    }

    #[test]
    fn same_host_whole_host_overlaps() {
        let h = Uuid::from_u128(5);
        let a = CredentialTargets::build(&[], &[host(h, None)]);
        let b = CredentialTargets::build(&[], &[host(h, Some(vec![Uuid::from_u128(2)]))]);
        // One side covers the whole host → overlaps regardless of the other's scope.
        assert!(a.overlaps(&b));
    }

    #[test]
    fn same_host_disjoint_ip_scopes_do_not_overlap() {
        let h = Uuid::from_u128(5);
        let a = CredentialTargets::build(&[], &[host(h, Some(vec![Uuid::from_u128(1)]))]);
        let b = CredentialTargets::build(&[], &[host(h, Some(vec![Uuid::from_u128(2)]))]);
        assert!(!a.overlaps(&b));
    }

    #[test]
    fn same_host_intersecting_ip_scopes_overlap() {
        let h = Uuid::from_u128(5);
        let shared = Uuid::from_u128(7);
        let a = CredentialTargets::build(&[], &[host(h, Some(vec![shared, Uuid::from_u128(1)]))]);
        let b = CredentialTargets::build(&[], &[host(h, Some(vec![shared]))]);
        assert!(a.overlaps(&b));
    }

    #[test]
    fn different_hosts_do_not_overlap() {
        let a = CredentialTargets::build(&[], &[host(Uuid::from_u128(1), None)]);
        let b = CredentialTargets::build(&[], &[host(Uuid::from_u128(2), None)]);
        assert!(!a.overlaps(&b));
    }

    // --- single_endpoint_targets_conflict (the shared rule both write paths use) ---

    use crate::server::credentials::r#impl::types::CredentialTypeDiscriminants;

    fn cred_type(d: CredentialTypeDiscriminants) -> CredentialType {
        d.to_credential_type()
    }

    #[test]
    fn socket_and_proxy_same_integration_same_host_conflict() {
        // A Docker socket and a Docker proxy both targeting the daemon host conflict
        // — bidirectionally (order doesn't matter).
        let h = Uuid::from_u128(42);
        let socket = cred_type(CredentialTypeDiscriminants::DockerSocket);
        let proxy = cred_type(CredentialTypeDiscriminants::DockerProxy);
        let t = CredentialTargets::for_host(h);
        assert!(single_endpoint_targets_conflict(&socket, &t, &proxy, &t));
        assert!(single_endpoint_targets_conflict(&proxy, &t, &socket, &t));
    }

    #[test]
    fn podman_socket_and_proxy_same_host_conflict() {
        let h = Uuid::from_u128(42);
        let socket = cred_type(CredentialTypeDiscriminants::PodmanSocket);
        let proxy = cred_type(CredentialTypeDiscriminants::PodmanProxy);
        let t = CredentialTargets::for_host(h);
        assert!(single_endpoint_targets_conflict(&socket, &t, &proxy, &t));
    }

    #[test]
    fn different_integrations_same_host_do_not_conflict() {
        // Docker and Podman are distinct integrations — both may target one host.
        let h = Uuid::from_u128(42);
        let docker = cred_type(CredentialTypeDiscriminants::DockerSocket);
        let podman = cred_type(CredentialTypeDiscriminants::PodmanSocket);
        let t = CredentialTargets::for_host(h);
        assert!(!single_endpoint_targets_conflict(&docker, &t, &podman, &t));
    }

    #[test]
    fn same_integration_different_hosts_do_not_conflict() {
        let socket = cred_type(CredentialTypeDiscriminants::DockerSocket);
        let proxy = cred_type(CredentialTypeDiscriminants::DockerProxy);
        let a = CredentialTargets::for_host(Uuid::from_u128(1));
        let b = CredentialTargets::for_host(Uuid::from_u128(2));
        assert!(!single_endpoint_targets_conflict(&socket, &a, &proxy, &b));
    }

    #[test]
    fn non_single_endpoint_integration_never_conflicts() {
        // SNMP is try-many, not single-endpoint per host.
        let h = Uuid::from_u128(42);
        let snmp = cred_type(CredentialTypeDiscriminants::SnmpV2c);
        let t = CredentialTargets::for_host(h);
        assert!(!single_endpoint_targets_conflict(&snmp, &t, &snmp, &t));
    }
}

/// Characterization tests for #637 (per-daemon credential↔IP targeting via `IntegrationTarget`).
/// These exercise the pure `apply_integration_target` transformation — the heart of the fix —
/// without a database.
#[cfg(test)]
mod integration_target_tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    type Mappings = BTreeMap<Uuid, TypedCredentialMapping>;

    fn localhost() -> IpAddr {
        IpAddr::V4(Ipv4Addr::LOCALHOST)
    }

    fn snmp_v2c(community: &str) -> CredentialType {
        CredentialType::SnmpV2c {
            community: crate::server::credentials::r#impl::types::SecretValue::Inline {
                value: secrecy::SecretString::from(community.to_string()),
            },
        }
    }

    /// The community a dispatched SNMP payload actually carries.
    fn payload_community(payload: &CredentialQueryPayload) -> String {
        use crate::server::credentials::r#impl::mapping::ResolvableSecret;
        match payload {
            CredentialQueryPayload::Snmp(snmp) => match &snmp.community {
                ResolvableSecret::Value { value } => value.clone(),
                other => panic!("expected an inline community, got {other:?}"),
            },
            other => panic!("expected an SNMP payload, got {other:?}"),
        }
    }

    /// The single mapping in `map`, which most of these tests build from one target.
    fn only(map: &Mappings) -> &CredentialMapping<CredentialQueryPayload> {
        assert_eq!(map.len(), 1, "expected exactly one mapping");
        &map.values().next().unwrap().mapping
    }

    /// A `DaemonHost` target is reached over the loopback, so it becomes a 127.0.0.1 override —
    /// the daemon probes it during discovery and it earns its host assignment only if that probe
    /// succeeds. Producing no override would leave the credential unreachable and silently unused.
    #[test]
    fn daemon_host_target_is_probed_over_loopback() {
        let cred_id = Uuid::new_v4();
        let target = IntegrationTarget::DaemonHost {
            credential_id: cred_id,
        };
        for disc in [
            CredentialTypeDiscriminants::DockerProxy,
            CredentialTypeDiscriminants::DockerSocket,
        ] {
            let mut map: Mappings = Mappings::new();
            apply_integration_target(&mut map, &target, &disc.to_credential_type());
            let mapping = only(&map);
            assert_eq!(mapping.ip_overrides.len(), 1);
            assert_eq!(mapping.ip_overrides[0].ip, localhost());
            assert_eq!(mapping.ip_overrides[0].credential_id, cred_id);
            // Reached over loopback, not fanned out across every interface on the host.
            assert!(mapping.default_credential.is_none());
        }
    }

    /// A `Hosts` target keeps every IP it names, loopback included — targeting the daemon host by
    /// its loopback address is the same thing as a `DaemonHost` target and is probed identically.
    #[test]
    fn hosts_target_keeps_loopback_ip() {
        let cred_id = Uuid::new_v4();
        let cred_type = CredentialTypeDiscriminants::DockerProxy.to_credential_type();
        let remote: IpAddr = "10.0.0.7".parse().unwrap();
        let target = IntegrationTarget::Hosts {
            credential_id: cred_id,
            ips: vec![localhost(), remote],
        };
        let mut map: Mappings = Mappings::new();
        apply_integration_target(&mut map, &target, &cred_type);
        let mapping = only(&map);
        assert_eq!(
            mapping
                .ip_overrides
                .iter()
                .map(|o| o.ip)
                .collect::<Vec<_>>(),
            vec![localhost(), remote]
        );
    }

    /// Applying the same target twice within one dispatch must not duplicate overrides — the
    /// case that reaches this is a credential arriving both as a target and as the host
    /// assignment it earned, on the scan where both are still present.
    #[test]
    fn reapplying_same_target_is_idempotent() {
        let cred_id = Uuid::new_v4();
        let cred_type = CredentialTypeDiscriminants::DockerProxy.to_credential_type();
        let target = IntegrationTarget::Hosts {
            credential_id: cred_id,
            ips: vec!["10.0.0.5".parse::<IpAddr>().unwrap()],
        };
        let mut map: Mappings = Mappings::new();
        apply_integration_target(&mut map, &target, &cred_type);
        apply_integration_target(&mut map, &target, &cred_type);
        let mapping = only(&map);
        assert_eq!(
            mapping.ip_overrides.len(),
            1,
            "a re-applied target must not duplicate the override"
        );
    }

    /// A target whose scope isn't permitted by its credential type's `targets()` is skipped —
    /// e.g. a `DockerSocket` credential (daemon-host only) given a `Network` scope.
    #[test]
    fn scope_not_permitted_by_credential_type_is_skipped() {
        let cred_type = CredentialTypeDiscriminants::DockerSocket.to_credential_type();
        let target = IntegrationTarget::Network {
            credential_id: Uuid::new_v4(),
        };
        let mut map: Mappings = Mappings::new();
        apply_integration_target(&mut map, &target, &cred_type);
        assert!(map.is_empty(), "invalid scope must not produce a mapping");
    }

    /// Two credentials of the same type must both reach the daemon.
    ///
    /// Keying the accumulator by credential *type* meant a network assigned two SNMPv2c
    /// communities dispatched only one of them — chosen by whichever had the lower UUID — and
    /// dropped the other with no log. Every device answering to the dropped community was then
    /// never scanned at all: no interfaces, no LLDP, absent from L2. Two communities (per site,
    /// per vendor) is ordinary practice.
    #[test]
    fn two_credentials_of_one_type_both_reach_the_daemon() {
        let mut map: Mappings = Mappings::new();
        let first = Uuid::from_u128(1);
        let second = Uuid::from_u128(2);

        for (id, community) in [(first, "netdefault"), (second, "secret42")] {
            let cred_type = snmp_v2c(community);
            mapping_for(&mut map, id, &cred_type).default_credential =
                Some(cred_type.to_query_payload());
        }

        assert_eq!(map.len(), 2, "both credentials must survive");
        let dispatched = order_mappings_for_dispatch(map);
        let communities: Vec<String> = dispatched
            .iter()
            .filter_map(|m| m.default_credential.as_ref())
            .map(payload_community)
            .collect();
        assert_eq!(communities, vec!["netdefault", "secret42"]);
    }

    /// A broadcast default must reach the daemon knowing which credential it is.
    ///
    /// The accumulator is keyed by credential id, and this is the only place that key can be put
    /// on the mapping — it was dropped here, which left a malformed network-wide credential able
    /// to say what failed but not which record to open. A malformed default is the one attempt
    /// outcome that is reported for a broadcast credential (see `issue_for_attempt`), so this is
    /// the whole path by which that warning gets an id.
    #[test]
    fn a_broadcast_default_carries_the_credential_it_came_from() {
        let mut map: Mappings = Mappings::new();
        let first = Uuid::from_u128(1);
        let second = Uuid::from_u128(2);

        for (id, community) in [(first, "netdefault"), (second, "secret42")] {
            let cred_type = snmp_v2c(community);
            mapping_for(&mut map, id, &cred_type).default_credential =
                Some(cred_type.to_query_payload());
        }

        let dispatched = order_mappings_for_dispatch(map);

        // Paired against the community, not just asserted non-None: two defaults of the same type
        // are exactly the case address alone cannot tell apart, so an id on the wrong mapping
        // would send the operator to the wrong record.
        let pairs: Vec<(String, Option<Uuid>)> = dispatched
            .iter()
            .filter_map(|m| {
                m.default_credential
                    .as_ref()
                    .map(|c| (payload_community(c), m.default_credential_id))
            })
            .collect();
        assert_eq!(
            pairs,
            vec![
                ("netdefault".to_string(), Some(first)),
                ("secret42".to_string(), Some(second)),
            ]
        );
    }

    /// A mapping with no default has nothing to attribute, so the id stays absent rather than
    /// naming a credential that was never broadcast.
    #[test]
    fn a_mapping_without_a_default_carries_no_default_id() {
        let mut map: Mappings = Mappings::new();
        let host_id = Uuid::from_u128(7);

        let specific = snmp_v2c("host-specific");
        mapping_for(&mut map, host_id, &specific)
            .ip_overrides
            .push(IpOverride {
                ip: "10.0.0.5".parse().unwrap(),
                credential: specific.to_query_payload(),
                credential_id: host_id,
                host_id: None,
                mac_address: None,
            });

        let dispatched = order_mappings_for_dispatch(map);

        assert_eq!(dispatched.len(), 1);
        assert_eq!(dispatched[0].default_credential_id, None);
    }

    /// The version gate has to run on the 7-way credential type, not the 5-way wire tag: SnmpV1
    /// and SnmpV2c both serialize as `Snmp`, but only SnmpV1 requires a 0.17.0 daemon. Gating
    /// after the collapse could not tell them apart and would either starve old daemons of
    /// SNMPv2c or hand them a payload they cannot deserialize.
    #[test]
    fn version_gate_drops_only_the_type_the_daemon_cannot_read() {
        let mut map: Mappings = Mappings::new();
        let v2c_id = Uuid::from_u128(1);
        let v1_id = Uuid::from_u128(2);

        let v2c = snmp_v2c("public");
        mapping_for(&mut map, v2c_id, &v2c).default_credential = Some(v2c.to_query_payload());
        let v1 = CredentialTypeDiscriminants::SnmpV1.to_credential_type();
        mapping_for(&mut map, v1_id, &v1).default_credential = Some(v1.to_query_payload());

        retain_daemon_compatible(&mut map, Some(&semver::Version::new(0, 16, 2)));

        assert_eq!(map.keys().copied().collect::<Vec<_>>(), vec![v2c_id]);
    }

    /// Overrides are emitted last so a host-specific credential still beats a network-wide one.
    ///
    /// Within one mapping the daemon tries overrides first and stops at the first success; across
    /// mappings it records the *last* success as the host's working credential. Splitting one
    /// mapping per type into one per credential inverts which rule applies, so without this
    /// ordering a host with its own assigned credential would start being attributed to whichever
    /// network-wide credential also happened to work.
    #[test]
    fn overrides_are_probed_after_broadcasts() {
        let mut map: Mappings = Mappings::new();
        // Lower id, so a naive credential-id ordering would put the broadcast last.
        let broadcast_id = Uuid::from_u128(1);
        let host_id = Uuid::from_u128(2);

        let broadcast = snmp_v2c("network-wide");
        mapping_for(&mut map, broadcast_id, &broadcast).default_credential =
            Some(broadcast.to_query_payload());

        let specific = snmp_v2c("host-specific");
        let specific_mapping = mapping_for(&mut map, host_id, &specific);
        specific_mapping.ip_overrides.push(IpOverride {
            ip: "10.0.0.5".parse().unwrap(),
            credential: specific.to_query_payload(),
            credential_id: host_id,
            host_id: None,
            mac_address: None,
        });

        let dispatched = order_mappings_for_dispatch(map);
        assert!(
            dispatched[0].ip_overrides.is_empty(),
            "the broadcast-only mapping must be probed first"
        );
        assert_eq!(
            payload_community(&dispatched[1].ip_overrides[0].credential),
            "host-specific",
            "the host-specific credential must be probed last so it wins the merge"
        );
    }

    fn credential(id: Uuid, credential_type: CredentialType) -> Credential {
        Credential {
            id,
            base: crate::server::credentials::r#impl::base::CredentialBase {
                credential_type,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn ip_address(id: Uuid, host_id: Uuid, ip: IpAddr) -> IPAddress {
        IPAddress {
            id,
            base: crate::server::ip_addresses::r#impl::base::IPAddressBase {
                host_id,
                ip_address: ip,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// The other half of the one-shot `integration_targets` contract: once a scan completes the
    /// discovery drops its `DaemonHost` target, and what keeps a working Docker/Podman socket
    /// credential scanning containers is the `host_credentials` assignment it earned on the
    /// daemon host's loopback address. That assignment has to come back out as a *localhost*
    /// override, because that is what the daemon's localhost-integration phase selects on
    /// (`runner.rs`: mappings with any `is_localhost()` override). If it didn't, container
    /// discovery would silently stop after the first scan.
    #[test]
    fn promoted_socket_credential_is_dispatched_over_loopback() {
        let cred_id = Uuid::new_v4();
        let daemon_host_id = Uuid::new_v4();
        let loopback_ip_id = Uuid::new_v4();
        let cred = credential(
            cred_id,
            CredentialTypeDiscriminants::DockerSocket.to_credential_type(),
        );
        let assignment = CredentialAssignment {
            credential_id: cred_id,
            ip_address_ids: Some(vec![loopback_ip_id]),
        };
        let ip_addresses = vec![
            ip_address(loopback_ip_id, daemon_host_id, localhost()),
            ip_address(Uuid::new_v4(), daemon_host_id, "10.0.0.4".parse().unwrap()),
        ];

        let mut map: Mappings = Mappings::new();
        apply_host_assignment(&mut map, daemon_host_id, &assignment, &cred, &ip_addresses);

        let mapping = only(&map);
        assert_eq!(mapping.ip_overrides.len(), 1);
        assert!(
            mapping.ip_overrides[0].is_localhost(),
            "a socket credential assigned to the daemon host's loopback must dispatch as a \
             localhost override, or the localhost phase will not select it"
        );
        assert_eq!(mapping.ip_overrides[0].credential_id, cred_id);
    }

    /// A host-wide assignment (`ip_address_ids: None`) covers every address on its host and
    /// nothing on any other. SNMP reports its assignments this way, and it is what a scoped
    /// assignment widens to when none of the daemon's reported addresses resolve
    /// (`remap_assignment_ip_ids`) — an empty id list would instead match nothing at all.
    #[test]
    fn host_wide_assignment_covers_only_its_own_host() {
        let cred_id = Uuid::new_v4();
        let host_id = Uuid::new_v4();
        let other_host_id = Uuid::new_v4();
        let cred = credential(cred_id, snmp_v2c("public"));
        let assignment = CredentialAssignment {
            credential_id: cred_id,
            ip_address_ids: None,
        };
        let ip_addresses = vec![
            ip_address(Uuid::new_v4(), host_id, "10.0.0.4".parse().unwrap()),
            ip_address(Uuid::new_v4(), host_id, "10.0.0.5".parse().unwrap()),
            ip_address(Uuid::new_v4(), other_host_id, "10.0.0.6".parse().unwrap()),
        ];

        let mut map: Mappings = Mappings::new();
        apply_host_assignment(&mut map, host_id, &assignment, &cred, &ip_addresses);

        let mut ips: Vec<IpAddr> = only(&map).ip_overrides.iter().map(|o| o.ip).collect();
        ips.sort();
        assert_eq!(
            ips,
            vec![
                "10.0.0.4".parse::<IpAddr>().unwrap(),
                "10.0.0.5".parse::<IpAddr>().unwrap()
            ]
        );
    }

    /// A device's own loopback is an interface it reports, not a way to reach it — and every
    /// device reports the same one, so it is not even a unique target. Swept into a host-wide
    /// assignment it produced an SNMP attempt against the daemon's own machine and an
    /// "unreachable" finding for a credential that was working at the host's real address.
    ///
    /// Only the incidental case goes: `promoted_socket_credential_is_dispatched_over_loopback`
    /// covers the other side, where the assignment names a loopback address explicitly and must
    /// still target it.
    #[test]
    fn a_host_wide_assignment_skips_the_hosts_own_loopback() {
        let cred_id = Uuid::new_v4();
        let host_id = Uuid::new_v4();
        let cred = credential(cred_id, snmp_v2c("public"));
        let assignment = CredentialAssignment {
            credential_id: cred_id,
            ip_address_ids: None,
        };
        let ip_addresses = vec![
            ip_address(Uuid::new_v4(), host_id, localhost()),
            ip_address(Uuid::new_v4(), host_id, "10.0.0.4".parse().unwrap()),
        ];

        let mut map: Mappings = Mappings::new();
        apply_host_assignment(&mut map, host_id, &assignment, &cred, &ip_addresses);

        let overrides = &only(&map).ip_overrides;
        assert_eq!(
            overrides.iter().map(|o| o.ip).collect::<Vec<_>>(),
            vec!["10.0.0.4".parse::<IpAddr>().unwrap()]
        );
        // And the device it came from rides along, so the daemon can tell a multi-homed host's
        // addresses apart from unrelated ones.
        assert_eq!(overrides[0].host_id, Some(host_id));
    }

    /// Wake-on-LAN addresses a sleeping host by MAC, and the daemon holds no host records to
    /// look one up in: each address's MAC has to travel on its own override, and an address the
    /// server holds no MAC for has to say so rather than borrow its sibling's.
    #[test]
    fn a_host_assignment_carries_each_addresss_own_mac() {
        use crate::server::ip_addresses::r#impl::base::{MacEvidence, MacEvidenceValue};
        use crate::server::shared::attribution::AttributeSource;

        let cred_id = Uuid::new_v4();
        let host_id = Uuid::new_v4();
        let cred = credential(
            cred_id,
            CredentialTypeDiscriminants::WakeOnLan.to_credential_type(),
        );
        let assignment = CredentialAssignment {
            credential_id: cred_id,
            ip_address_ids: None,
        };
        let mac: mac_address::MacAddress = "3c:ec:ef:12:34:56".parse().unwrap();
        let mut with_mac = ip_address(Uuid::new_v4(), host_id, "10.0.0.4".parse().unwrap());
        with_mac.base.mac_address = Some(MacEvidence::new(
            MacEvidenceValue(mac),
            AttributeSource::ArpReply,
        ));
        let without_mac = ip_address(Uuid::new_v4(), host_id, "10.0.0.5".parse().unwrap());

        let mut map: Mappings = Mappings::new();
        apply_host_assignment(
            &mut map,
            host_id,
            &assignment,
            &cred,
            &[with_mac, without_mac],
        );

        let mut macs: Vec<(IpAddr, Option<mac_address::MacAddress>)> = only(&map)
            .ip_overrides
            .iter()
            .map(|o| (o.ip, o.mac_address))
            .collect();
        macs.sort();
        assert_eq!(
            macs,
            vec![
                ("10.0.0.4".parse().unwrap(), Some(mac)),
                ("10.0.0.5".parse().unwrap(), None),
            ]
        );
    }

    /// A second `Network` target of the same credential type must not displace the first — the
    /// same collapse, reached through the init-command targeting path instead.
    #[test]
    fn two_network_targets_of_one_type_both_survive() {
        let mut map: Mappings = Mappings::new();
        let cred_type = CredentialTypeDiscriminants::SnmpV2c.to_credential_type();

        for id in [Uuid::from_u128(1), Uuid::from_u128(2)] {
            apply_integration_target(
                &mut map,
                &IntegrationTarget::Network { credential_id: id },
                &cred_type,
            );
        }

        assert_eq!(map.len(), 2);
        assert!(
            map.values().all(|t| t.mapping.default_credential.is_some()),
            "both targets must produce a dispatchable default"
        );
    }

    /// A `Network`-scoped target becomes a network-level default, not an IP override.
    #[test]
    fn network_scope_sets_default_not_override() {
        let cred_type = CredentialTypeDiscriminants::SnmpV2c.to_credential_type();
        let target = IntegrationTarget::Network {
            credential_id: Uuid::new_v4(),
        };
        let mut map: Mappings = Mappings::new();
        apply_integration_target(&mut map, &target, &cred_type);
        let mapping = only(&map);
        assert!(mapping.ip_overrides.is_empty());
        assert!(mapping.default_credential.is_some());
    }

    /// A `Hosts` target produces one override per IP.
    #[test]
    fn hosts_scope_overrides_each_ip() {
        let cred_id = Uuid::new_v4();
        let cred_type = CredentialTypeDiscriminants::SnmpV2c.to_credential_type();
        let ips = vec![
            "10.0.0.5".parse::<IpAddr>().unwrap(),
            "10.0.0.6".parse::<IpAddr>().unwrap(),
        ];
        let target = IntegrationTarget::Hosts {
            credential_id: cred_id,
            ips: ips.clone(),
        };
        let mut map: Mappings = Mappings::new();
        apply_integration_target(&mut map, &target, &cred_type);
        let mapping = only(&map);
        assert_eq!(mapping.ip_overrides.len(), 2);
        assert_eq!(
            mapping
                .ip_overrides
                .iter()
                .map(|o| o.ip)
                .collect::<Vec<_>>(),
            ips
        );
    }

    /// The wire/storage format of `IntegrationTarget` is an internally-tagged enum keyed on
    /// `scope`; lock it so the JSONB column and registration request stay stable, and so `Target`
    /// (its discriminant) tracks the variant names.
    #[test]
    fn integration_target_serde_is_tagged() {
        let cred_id = Uuid::nil();
        let daemon_host = IntegrationTarget::DaemonHost {
            credential_id: cred_id,
        };
        let json = serde_json::to_value(&daemon_host).unwrap();
        assert_eq!(json["scope"], "DaemonHost");
        assert_eq!(json["credential_id"], cred_id.to_string());

        let hosts = IntegrationTarget::Hosts {
            credential_id: cred_id,
            ips: vec![localhost()],
        };
        let json = serde_json::to_value(&hosts).unwrap();
        assert_eq!(json["scope"], "Hosts");
        assert_eq!(json["ips"][0], "127.0.0.1");

        // Discriminant tracks variant names.
        assert_eq!(Target::from(&daemon_host), Target::DaemonHost);
        assert_eq!(Target::from(&hosts), Target::Hosts);

        // Round-trip.
        for t in [daemon_host, hosts] {
            assert_eq!(
                t,
                serde_json::from_value(serde_json::to_value(&t).unwrap()).unwrap()
            );
        }
    }
}
