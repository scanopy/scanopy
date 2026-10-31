//! Recording a hypervisor's guests, and what became of each.
//!
//! A guest links to its hypervisor's service by the service's stored id, and each of its network
//! identities links to the guest's Network Identities service the same way, so a guest goes in
//! before its identities and its hypervisor before it. What a run records decides which
//! hypervisors had every guest listed, which is what lets the server delete the guests a run no
//! longer reported.

use std::collections::HashSet;

use anyhow::Error;
use futures::stream::{self, StreamExt};
use uuid::Uuid;

use crate::daemon::discovery::integration::IntegrationContext;
use crate::server::interfaces::r#impl::base::{Interface, InterfaceDataComplete};
use crate::server::services::r#impl::definitions::{ServiceDefinitionExt, VirtualizationRole};
use crate::server::subnets::r#impl::base::Subnet;

use super::guest::GuestRecord;
use super::identities::{self, NetworkIdentity};

/// Guests read and recorded at once. In ServerPoll mode each record waits for the server's next
/// collection, every 30s, so recorded one at a time a dozen guests outlast the 300s budget and
/// the whole collection is thrown away.
pub const CONCURRENT_GUEST_SYNCS: usize = 8;

/// What became of one guest.
pub enum GuestOutcome {
    /// No address and no NIC, so nothing was recorded. Carries the guest's label.
    Unidentifiable(String),
    Recorded {
        host_id: Uuid,
        /// Every network identity was read and recorded.
        identities_listed: bool,
    },
    /// Its record failed, or the run was cancelled before it. Carries its hypervisor host's key
    /// (a Proxmox node's name, a libvirt host's address).
    Failed { owner: String },
}

/// Every guest's outcome, gathered.
#[derive(Default)]
pub struct GuestSync {
    pub created: usize,
    pub unidentifiable: Vec<String>,
    /// Hypervisor hosts with a guest that failed to record: their guests are not listed in full.
    pub short_owners: HashSet<String>,
    /// Guests whose network identities were all read and recorded.
    pub identities_listed: Vec<Uuid>,
}

/// Run every guest's sync, [`CONCURRENT_GUEST_SYNCS`] at a time, and gather the outcomes.
pub async fn sync_guests(syncs: Vec<impl Future<Output = GuestOutcome>>) -> GuestSync {
    let outcomes: Vec<GuestOutcome> = stream::iter(syncs)
        .buffer_unordered(CONCURRENT_GUEST_SYNCS)
        .collect()
        .await;
    let mut gathered = GuestSync::default();
    for outcome in outcomes {
        match outcome {
            GuestOutcome::Unidentifiable(label) => gathered.unidentifiable.push(label),
            GuestOutcome::Recorded {
                host_id,
                identities_listed,
            } => {
                gathered.created += 1;
                if identities_listed {
                    gathered.identities_listed.push(host_id);
                }
            }
            GuestOutcome::Failed { owner } => {
                gathered.short_owners.insert(owner);
            }
        }
    }
    gathered
}

/// Record a guest's host, then each of its network identities as a host of its own. `label` names
/// the guest in log lines.
///
/// A guest with identities carries the Network Identities service, and each identity links to
/// that service's stored id, read back from the guest's response the way its hypervisor's service
/// id is. So the guest goes first. An identity that fails to record is logged and the rest go on:
/// the guest itself is recorded.
///
/// Returns the guest host's stored id, and whether every identity was recorded.
pub async fn record_guest(
    ctx: &IntegrationContext<'_>,
    subnets: &[Subnet],
    label: &str,
    record: GuestRecord,
    identities: &[NetworkIdentity],
    site_id: Uuid,
) -> Result<(Uuid, bool), Error> {
    let services = identities::guest_services(&record.host, identities);
    // The interfaces its identities sit on, so each identity can name its interface by stored id.
    let mut interfaces = record.interfaces;
    interfaces.extend(
        identities
            .iter()
            .map(|i| identities::identity_interface(i, site_id)),
    );
    let response = ctx
        .ops
        .create_integration_host(
            ctx.integration,
            record.host,
            record.ip_addresses,
            vec![],
            services,
            interfaces,
            vec![],
            // A hypervisor reports a guest's addresses and NIC MACs, never its interface table.
            false,
            InterfaceDataComplete::none(),
            ctx.cancel,
        )
        .await?;
    if identities.is_empty() {
        return Ok((response.id, true));
    }

    let Some(owner) = response
        .services
        .iter()
        .find(|s| {
            matches!(
                s.base.service_definition.virtualization_role(),
                Some(VirtualizationRole::IdentityHost { .. })
            )
        })
        .map(|s| s.id)
    else {
        tracing::warn!(
            guest = %label,
            "Guest was recorded without its Network Identities service; its network identities \
             were not recorded"
        );
        return Ok((response.id, false));
    };
    // Recorded together: in ServerPoll mode each waits for the server's next collection.
    let records: Vec<_> = identities
        .iter()
        .map(|identity| {
            record_identity(
                ctx,
                label,
                identity,
                owner,
                &response.interfaces,
                subnets,
                site_id,
            )
        })
        .collect();
    let all_recorded = futures::future::join_all(records)
        .await
        .into_iter()
        .all(|recorded| recorded);
    Ok((response.id, all_recorded))
}

/// Record one of a guest's network identities as a host of its own, linked to `owner`, the
/// guest's Network Identities service. Returns whether it was recorded.
async fn record_identity(
    ctx: &IntegrationContext<'_>,
    label: &str,
    identity: &NetworkIdentity,
    owner: Uuid,
    guest_interfaces: &[Interface],
    subnets: &[Subnet],
    site_id: Uuid,
) -> bool {
    let presenting = identities::presenting_interface_id(identity, guest_interfaces);
    if presenting.is_none() {
        tracing::warn!(
            guest = %label,
            interface = identity.interface.as_deref().unwrap_or("-"),
            "The guest's interface for a network identity was not stored; recording the \
             identity without it"
        );
    }
    let (host, ip_addresses) =
        identities::identity_host(identity, owner, presenting, subnets, site_id);
    if let Err(e) = ctx
        .ops
        .create_integration_host(
            ctx.integration,
            host,
            ip_addresses,
            vec![],
            vec![],
            vec![],
            vec![],
            // One interface of the guest's, seen from the guest; never an interface table.
            false,
            InterfaceDataComplete::none(),
            ctx.cancel,
        )
        .await
    {
        tracing::warn!(
            guest = %label,
            interface = identity.interface.as_deref().unwrap_or("-"),
            error = %e,
            "Failed to record a guest's network identity"
        );
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// In ServerPoll mode every record waits for the server's next collection, 30s apart, so
    /// thirteen guests recorded one at a time take 390s and outlast the 300s collection budget.
    /// Recorded together they take a couple of collections. `poll` stands in for the 30s.
    #[tokio::test]
    async fn guests_waiting_on_the_server_are_recorded_together() {
        let poll = Duration::from_millis(100);
        let started = std::time::Instant::now();
        let syncs = (0..13)
            .map(|_| async move {
                tokio::time::sleep(poll).await;
                GuestOutcome::Recorded {
                    host_id: Uuid::new_v4(),
                    identities_listed: true,
                }
            })
            .collect();

        let sync = sync_guests(syncs).await;

        assert_eq!(sync.created, 13);
        assert!(started.elapsed() < poll * 5, "took {:?}", started.elapsed());
    }

    /// A guest that failed to record keeps its hypervisor host from being listed in full, and only
    /// that host.
    #[tokio::test]
    async fn a_failed_guest_holds_back_only_its_own_hypervisor() {
        let recorded = |identities_listed| async move {
            GuestOutcome::Recorded {
                host_id: Uuid::new_v4(),
                identities_listed,
            }
        };
        let failed = |owner: &str| {
            let owner = owner.to_string();
            async move { GuestOutcome::Failed { owner } }
        };

        let sync = sync_guests(vec![
            futures::future::Either::Left(recorded(true)),
            futures::future::Either::Left(recorded(false)),
            futures::future::Either::Right(failed("pve2")),
        ])
        .await;

        assert_eq!(sync.created, 2);
        assert_eq!(sync.short_owners, HashSet::from(["pve2".to_string()]));
        assert_eq!(sync.identities_listed.len(), 1);
    }
}
