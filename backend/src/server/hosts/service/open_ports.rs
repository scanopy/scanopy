//! Taking ports back from Unclaimed Open Ports while a discovery payload is stored.
//!
//! Unclaimed Open Ports holds the ports a scan found open with no service to give them to. A
//! service that later claims one of them takes it, whether the service is new to the host or was
//! stored by an earlier scan, so the placeholder never outranks a real owner (GH #784).
use super::stale_ports::{overlaps, taken_claims};
use super::*;

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

/// Whether an Unclaimed Open Ports service among `services` binds the port of `claim` at an
/// overlapping address (an unscoped binding overlaps every address).
pub(crate) fn held_by_open_ports(claim: &(Uuid, Option<Uuid>), services: &[Service]) -> bool {
    let (port_id, claim_iface) = claim;
    services.iter().any(|s| {
        ServiceDefinitionExt::is_open_ports(&s.base.service_definition)
            && s.base.bindings.iter().any(|b| {
                b.port_id() == Some(*port_id) && bindings_overlap(claim_iface, &b.ip_address_id())
            })
    })
}

impl HostService {
    /// Remove `claims` from the host's Unclaimed Open Ports service, deleting the service when no
    /// binding is left, and mirror the change in `services` so later services in the payload see
    /// it.
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
        let Some(open_ports) = services
            .iter_mut()
            .find(|s| ServiceDefinitionExt::is_open_ports(&s.base.service_definition))
        else {
            return;
        };
        let reclaimed = taken_claims(open_ports, claims);
        if reclaimed.is_empty() {
            return;
        }
        let open_ports_id = open_ports.id;
        open_ports
            .base
            .bindings
            .retain(|b| !claims.iter().any(|claim| overlaps(b, claim)));
        let remaining_bindings = open_ports.base.bindings.len();

        // The taken bindings go first, so deleting the emptied service below reports none of them
        // as deleted with it.
        let _ = self
            .service_service
            .remove_port_bindings(
                &open_ports_id,
                &reclaimed,
                authentication.clone(),
                reason.clone(),
            )
            .await;
        if remaining_bindings == 0 {
            let _ = self
                .service_service
                .delete(&open_ports_id, authentication, reason)
                .await;
        }
    }
}
