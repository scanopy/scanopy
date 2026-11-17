//! Taking ports back from the placeholder services while a discovery payload is stored.
//!
//! Unclaimed Open Ports holds the ports a scan found open with no service to give them to, and
//! Web Service holds those of them that answered HTTP. A service that later claims one of them
//! takes it, whether the service is new to the host or was stored by an earlier scan, so a
//! placeholder never outranks a real owner (GH #784).
use super::stale_ports::{overlaps, taken_claims};
use super::*;
use crate::server::services::definitions::open_ports::OpenPorts;
use crate::server::shared::types::metadata::HasId;

/// A port binding as the (port, address) claim the conflict checks compare.
pub(crate) fn port_claim(binding: &Binding) -> Option<(Uuid, Option<Uuid>)> {
    match binding.base.binding_type {
        BindingType::Port {
            port_id,
            ip_address_id,
        } => Some((port_id, ip_address_id)),
        BindingType::IPAddress { .. } => None,
    }
}

/// Whether `placeholder` gives its ports up to `claimant`. Unclaimed Open Ports ranks below Web
/// Service, and both below an identified service, so a placeholder yields to any claimant but
/// its own definition and Unclaimed Open Ports.
fn yields_to(placeholder: &Service, claimant: &Service) -> bool {
    let definition = |s: &Service| s.base.service_definition.id();
    ServiceDefinitionExt::is_open_ports(&placeholder.base.service_definition)
        && placeholder.id != claimant.id
        && definition(placeholder) != definition(claimant)
        && definition(claimant) != OpenPorts.id()
}

/// Whether a placeholder among `services` that yields to `claimant` binds the port of `claim` at
/// an overlapping address (an unscoped binding overlaps every address).
pub(crate) fn held_by_open_ports(
    claim: &(Uuid, Option<Uuid>),
    services: &[Service],
    claimant: &Service,
) -> bool {
    let (port_id, claim_iface) = claim;
    services.iter().any(|s| {
        yields_to(s, claimant)
            && s.base.bindings.iter().any(|b| {
                b.port_id() == Some(*port_id) && bindings_overlap(claim_iface, &b.ip_address_id())
            })
    })
}

impl HostService {
    /// Remove `claims` from every placeholder service on the host that yields to `claimant`,
    /// deleting a placeholder when no binding is left, and mirror the change in `services` so
    /// later services in the payload see it.
    pub(crate) async fn reclaim_from_open_ports(
        &self,
        claims: &[(Uuid, Option<Uuid>)],
        services: &mut [Service],
        claimant: &Service,
        authentication: AuthenticatedEntity,
    ) {
        let reason = DeletionReason::ServiceOpenPortsReclaimed {
            to_service_id: claimant.id,
            by: claimant.base.service_definition.clone(),
        };
        for placeholder in services.iter_mut().filter(|s| yields_to(s, claimant)) {
            let reclaimed = taken_claims(placeholder, claims);
            if reclaimed.is_empty() {
                continue;
            }
            let placeholder_id = placeholder.id;
            placeholder
                .base
                .bindings
                .retain(|b| !claims.iter().any(|claim| overlaps(b, claim)));
            let remaining_bindings = placeholder.base.bindings.len();

            // The taken bindings go first, so deleting the emptied service below reports none of
            // them as deleted with it.
            let _ = self
                .service_service
                .remove_port_bindings(
                    &placeholder_id,
                    &reclaimed,
                    authentication.clone(),
                    reason.clone(),
                )
                .await;
            if remaining_bindings == 0 {
                let _ = self
                    .service_service
                    .delete(&placeholder_id, authentication.clone(), reason.clone())
                    .await;
            }
        }
    }
}
