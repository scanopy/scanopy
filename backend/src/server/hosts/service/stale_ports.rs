//! Taking ports from stored services that are no longer there.
//!
//! A stored port binding is a claim that outlives the software behind it unless something takes
//! it back. Two things do, and only at a (port, address) the submission's coverage includes:
//!
//! - **Handover.** An identified service answers on a port another service holds, and the holder
//!   was not found in this submission. The newcomer takes the port.
//! - **Release.** Nothing answers on a covered port, and the port has gone unseen past the site's
//!   stale window. The holder gives it up. A single quiet scan (a restart, a maintenance window)
//!   releases nothing.
//!
//! A holder left with no bindings is deleted. Manual and system services are never touched, and
//! empty coverage (an older daemon, an integration that probes no ports) moves nothing.
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;

use super::*;
use crate::server::ports::r#impl::{base::PortType, coverage::PortCoverage};
use crate::server::shared::storage::snapshot::DiscoveryTracked;

use super::open_ports::port_claim;

/// A (port, address) claim, as `port_claim` returns it.
type PortClaim = (Uuid, Option<Uuid>);

/// What one submission says about which stored port bindings may change hands.
pub(crate) struct PortTurnover<'a> {
    coverage: &'a PortCoverage,
    /// Port id → number and protocol, for the host's stored ports.
    ports: HashMap<Uuid, Port>,
    /// Address id → address, for every address the host has stored.
    addresses: HashMap<Uuid, IpAddr>,
    /// Stored services found again in this submission, and services it created. Both answered
    /// this scan, so neither gives anything up.
    present: HashSet<Uuid>,
}

impl<'a> PortTurnover<'a> {
    pub(crate) fn is_empty(&self) -> bool {
        self.coverage.is_empty()
    }

    /// Mark a service as answering in this submission.
    pub(crate) fn mark_present(&mut self, service_id: Uuid) {
        self.present.insert(service_id);
    }

    /// Whether the submission covered `claim`. An all-addresses binding is covered only when its
    /// port was covered at every address the host has.
    pub(crate) fn covers(&self, claim: &PortClaim) -> bool {
        let (port_id, ip_address_id) = claim;
        let Some(port) = self.ports.get(port_id) else {
            return false;
        };
        let port_type: &PortType = &port.base.port_type;
        match ip_address_id {
            Some(id) => self
                .addresses
                .get(id)
                .is_some_and(|ip| self.coverage.covers(*ip, port_type)),
            None => {
                !self.addresses.is_empty()
                    && self
                        .addresses
                        .values()
                        .all(|ip| self.coverage.covers(*ip, port_type))
            }
        }
    }

    /// Whether `holder` gives up its ports to `claimant` when they collide on a covered port.
    ///
    /// The claimant must be identified: a generic port-only match is weaker evidence than the
    /// stored identification, so it takes nothing. A container is the exception: the runtime
    /// publishes a host port to one container at a time, so a different container from the same
    /// runtime on the holder's port means the holder's container is gone.
    pub(crate) fn hands_over(&self, holder: &Service, claimant: &Service) -> bool {
        if holder.id == claimant.id
            || self.present.contains(&holder.id)
            || !holder.base.source.is_from_discovery()
            || ServiceDefinitionExt::is_open_ports(&claimant.base.service_definition)
            || claimant.base.virtualization_service_id == Some(holder.id)
        {
            return false;
        }
        if !ServiceDefinitionExt::is_generic(&claimant.base.service_definition) {
            return true;
        }
        match (container_id_of(claimant), container_id_of(holder)) {
            (Some(new), Some(old)) => {
                new != old
                    && claimant.base.virtualization_service_id.is_some()
                    && claimant.base.virtualization_service_id
                        == holder.base.virtualization_service_id
            }
            _ => false,
        }
    }

    /// The services in `services` other than Unclaimed Open Ports that hold `claims` and give
    /// them up to `claimant`. `None` when one of them keeps its port, or holds a claim the
    /// submission didn't cover. Unclaimed Open Ports is left to `reclaim_from_open_ports`, so a
    /// claim only it holds needs no coverage, as before.
    pub(crate) fn holders_handing_over(
        &self,
        claims: &[PortClaim],
        services: &[Service],
        claimant: &Service,
    ) -> Option<Vec<Uuid>> {
        let mut holders = Vec::new();
        for claim in claims {
            for holder in services.iter().filter(|s| {
                s.id != claimant.id
                    && !ServiceDefinitionExt::is_open_ports(&s.base.service_definition)
                    && s.base.bindings.iter().any(|b| overlaps(b, claim))
            }) {
                // The holder's binding must be covered as well as the claim: an all-addresses
                // binding overlaps a claim at one address, and gives way only when the scan
                // covered the port at all of them.
                let holder_covered = holder
                    .base
                    .bindings
                    .iter()
                    .filter(|b| overlaps(b, claim))
                    .filter_map(port_claim)
                    .all(|held| self.covers(&held));
                if !self.covers(claim) || !holder_covered || !self.hands_over(holder, claimant) {
                    return None;
                }
                if !holders.contains(&holder.id) {
                    holders.push(holder.id);
                }
            }
        }
        Some(holders)
    }
}

/// The (port, address) claims of `holder`'s bindings that `claims` take.
pub(crate) fn taken_claims(holder: &Service, claims: &[PortClaim]) -> Vec<PortClaim> {
    holder
        .base
        .bindings
        .iter()
        .filter(|b| claims.iter().any(|claim| overlaps(b, claim)))
        .filter_map(port_claim)
        .collect()
}

/// The container id a service carries, if it is a container.
fn container_id_of(service: &Service) -> Option<String> {
    service
        .base
        .virtualization_metadata
        .as_ref()
        .and_then(|v| v.container_id().map(str::to_owned))
}

/// The stored service a container placeholder report is, found by container id. A stopped
/// container can only be reported as a placeholder, since nothing can be probed, and the runtime's
/// container id says it is the stored one, whatever that was identified as.
///
/// Only a placeholder report: a generic protocol such as SSH identified at the container is an
/// identification in its own right, and replaces a stored placeholder rather than being folded into
/// it (`Service::is_placeholder_for`).
pub(crate) fn stored_container<'a>(
    stored: &'a [Service],
    incoming: &Service,
) -> Option<&'a Service> {
    if !incoming.is_container_placeholder() {
        return None;
    }
    let container_id = container_id_of(incoming)?;
    stored
        .iter()
        .find(|s| container_id_of(s).as_deref() == Some(container_id.as_str()))
}

/// Whether `binding` is a port binding overlapping `claim`.
pub(crate) fn overlaps(binding: &Binding, claim: &PortClaim) -> bool {
    binding.port_id() == Some(claim.0) && bindings_overlap(&claim.1, &binding.ip_address_id())
}

impl HostService {
    /// Load what [`PortTurnover`] needs for `host_id`. With empty coverage nothing is loaded,
    /// since nothing can move.
    pub(crate) async fn port_turnover<'a>(
        &self,
        host_id: &Uuid,
        coverage: &'a PortCoverage,
        present: HashSet<Uuid>,
    ) -> Result<PortTurnover<'a>> {
        let (ports, addresses) = if coverage.is_empty() {
            (HashMap::new(), HashMap::new())
        } else {
            let ports = self
                .port_service
                .get_for_host(host_id)
                .await?
                .into_iter()
                .map(|p| (p.id, p))
                .collect();
            let addresses = self
                .ip_address_service
                .get_for_host(host_id)
                .await?
                .into_iter()
                .map(|a| (a.id, a.base.ip_address))
                .collect();
            (ports, addresses)
        };
        Ok(PortTurnover {
            coverage,
            ports,
            addresses,
            present,
        })
    }

    /// Remove `claims` from each of `holder_ids`, deleting a holder left with no binding, and
    /// mirror the change in `services` so later services in the payload see it.
    pub(crate) async fn take_ports(
        &self,
        holder_ids: &[Uuid],
        claims: &[PortClaim],
        services: &mut [Service],
        reason: DeletionReason,
        authentication: AuthenticatedEntity,
    ) -> Result<()> {
        // A rescanned service can sit in `services` twice, stored and re-stored. Every copy drops
        // the bindings, and the store is written and the move announced once.
        let mut seen: HashSet<Uuid> = HashSet::new();
        for holder in services.iter_mut().filter(|s| holder_ids.contains(&s.id)) {
            if !seen.insert(holder.id) {
                holder
                    .base
                    .bindings
                    .retain(|b| !claims.iter().any(|claim| overlaps(b, claim)));
                continue;
            }
            let taken = taken_claims(holder, claims);
            if taken.is_empty() {
                continue;
            }
            holder
                .base
                .bindings
                .retain(|b| !claims.iter().any(|claim| overlaps(b, claim)));

            // The taken bindings go first, so a holder deleted below carries only what it still
            // held: a moved binding is never also reported as deleted with its service.
            self.service_service
                .remove_port_bindings(&holder.id, &taken, authentication.clone(), reason.clone())
                .await?;

            // A runtime or hypervisor left with no bindings stays: deleting it would delete
            // everything it runs.
            if holder.base.bindings.is_empty()
                && holder
                    .base
                    .service_definition
                    .virtualization_role()
                    .is_none()
            {
                self.service_service
                    .delete(&holder.id, authentication.clone(), reason.clone())
                    .await?;
            }
        }
        Ok(())
    }

    /// Fold into `claimant` the generic placeholders stored for its container. Returns them, cleared
    /// of their bindings, for [`Self::finish_absorbing`] once `claimant` is stored.
    ///
    /// A container the daemon couldn't identify is stored as a generic container. Once a scan
    /// identifies it, the runtime reports the same container id on the same runtime, so the
    /// placeholder and the claimant are one container: the claimant takes every binding the
    /// placeholder held, whatever this scan covered. A placeholder found again in this
    /// submission is still answering as itself and is left alone.
    pub(crate) async fn absorb_container_placeholders(
        &self,
        turnover: &PortTurnover<'_>,
        claimant: &mut Service,
        services: &mut Vec<Service>,
        authentication: AuthenticatedEntity,
    ) -> Result<Vec<Service>> {
        let placeholders: Vec<Service> = services
            .iter()
            .filter(|s| {
                s.base.source.is_from_discovery()
                    && !turnover.present.contains(&s.id)
                    && s.is_placeholder_for(claimant)
            })
            .cloned()
            .collect();

        for placeholder in &placeholders {
            claimant.absorb_placeholder_bindings(placeholder);
            // Its bindings go now, so they don't count as conflicts with the claimant's. The row and
            // its dependency memberships stay until the claimant is stored and they can be
            // repointed.
            self.service_service
                .clear_bindings(
                    &placeholder.id,
                    authentication.clone(),
                    DeletionReason::ServiceIdentifiedAs {
                        service_id: claimant.id,
                    },
                )
                .await?;
            services.retain(|s| s.id != placeholder.id);
        }
        Ok(placeholders)
    }

    /// Point what referenced each absorbed placeholder at `claimant`, now stored, and delete it. A
    /// dependency member follows the same container to its identified service, so the delete
    /// leaves no member behind and announces none.
    pub(crate) async fn finish_absorbing(
        &self,
        placeholders: Vec<Service>,
        claimant: &Service,
        authentication: AuthenticatedEntity,
    ) -> Result<()> {
        for placeholder in &placeholders {
            let binding_map: HashMap<Uuid, Uuid> = placeholder
                .base
                .bindings
                .iter()
                .filter_map(|old| {
                    claimant
                        .base
                        .bindings
                        .iter()
                        .find(|new| new.base.binding_type == old.base.binding_type)
                        .map(|new| (old.id(), new.id()))
                })
                .collect();
            self.service_service
                .replace_service_references(
                    placeholder,
                    claimant,
                    &binding_map,
                    authentication.clone(),
                )
                .await?;
            self.service_service
                .delete(
                    &placeholder.id,
                    authentication.clone(),
                    DeletionReason::ServiceIdentifiedAs {
                        service_id: claimant.id,
                    },
                )
                .await?;
        }
        Ok(())
    }

    /// Release every covered port binding nothing answered on, once its port has gone unseen past
    /// the site's stale window.
    ///
    /// `answered` is the set of port ids this submission reported open on the host. A port
    /// reported at any address counts as answering, which only ever delays a release.
    pub(crate) async fn release_silent_ports(
        &self,
        turnover: &PortTurnover<'_>,
        site_id: &Uuid,
        answered: &HashSet<Uuid>,
        services: &mut [Service],
        authentication: AuthenticatedEntity,
    ) -> Result<()> {
        if turnover.is_empty() {
            return Ok(());
        }
        let Some(site) = self.site_service.get_by_id(site_id).await? else {
            return Ok(());
        };
        let cutoff = site.stale_cutoff(crate::server::shared::clock::now());

        let mut releases: Vec<(Uuid, Vec<PortClaim>)> = Vec::new();
        for service in services
            .iter()
            .filter(|s| s.base.source.is_from_discovery())
        {
            let claims: Vec<PortClaim> = service
                .base
                .bindings
                .iter()
                .filter_map(port_claim)
                .filter(|claim| {
                    !answered.contains(&claim.0)
                        && turnover.covers(claim)
                        && turnover
                            .ports
                            .get(&claim.0)
                            .is_some_and(|port| port.last_seen_at() < cutoff)
                })
                .collect();
            if !claims.is_empty() {
                releases.push((service.id, claims));
            }
        }

        for (service_id, claims) in releases {
            self.take_ports(
                &[service_id],
                &claims,
                services,
                DeletionReason::ServicePortsReleased,
                authentication.clone(),
            )
            .await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::services::definitions::{
        docker_container::DockerContainer, gitea::Gitea, ssh::Ssh,
    };
    use crate::server::services::r#impl::definitions::ServiceDefinition;
    use crate::server::services::r#impl::virtualization::{
        DockerVirtualization, ServiceVirtualization,
    };

    fn in_container(definition: Box<dyn ServiceDefinition>) -> Service {
        let mut service = Service {
            id: Uuid::new_v4(),
            ..Default::default()
        };
        service.base.service_definition = definition;
        service.base.virtualization_metadata =
            Some(ServiceVirtualization::Docker(DockerVirtualization {
                container_name: None,
                container_id: Some("c1".to_string()),
                compose_project: None,
                state: Default::default(),
            }));
        service
    }

    /// A placeholder report (a stopped container, say) is the stored container, whatever that was
    /// identified as. A generic protocol identified at the container is not a placeholder report:
    /// it must stay its own service so it can replace a stored placeholder.
    #[test]
    fn only_a_placeholder_report_is_folded_into_the_stored_container() {
        let stored = vec![in_container(Box::new(Gitea))];
        assert_eq!(
            stored_container(&stored, &in_container(Box::new(DockerContainer))).map(|s| s.id),
            Some(stored[0].id)
        );
        assert!(stored_container(&stored, &in_container(Box::new(Ssh))).is_none());
    }
}
