//! Topology subscriber: live-update SSE broadcast.
//!
//! When discovery inserts/updates/deletes any topology-relevant entity (host,
//! ip_address, service, subnet, dependency, port, binding, interface, vlan,
//! tag), we broadcast the affected `site_id` on `live_update_tx` so frontend
//! SSE consumers refetch. The graph itself is built on request from current
//! entities + options (no stored graph to rebuild here), so the subscriber only
//! pings — it does not touch any topology row.

use std::collections::HashMap;
use std::collections::HashSet;

use crate::server::{
    shared::{
        entities::EntityDiscriminants,
        events::{
            registry::SubscriberRegistration,
            traits::{EntityEventFilter, Event, EventScope, ScopeOrganization, Subscriber},
            types::EntityOperation,
        },
        services::traits::CrudService,
        storage::filter::StorableFilter as StorageFilter,
    },
    topology::service::main::TopologyService,
};
use anyhow::Error;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
impl Subscriber<EntityOperation> for TopologyService {
    fn filter(&self) -> EntityEventFilter {
        let all_ops = None;
        EntityEventFilter::by_entity(HashMap::from([
            (EntityDiscriminants::Host, all_ops.clone()),
            (EntityDiscriminants::IPAddress, all_ops.clone()),
            (EntityDiscriminants::Service, all_ops.clone()),
            (EntityDiscriminants::Subnet, all_ops.clone()),
            (EntityDiscriminants::Dependency, all_ops.clone()),
            (EntityDiscriminants::Port, all_ops.clone()),
            (EntityDiscriminants::Binding, all_ops.clone()),
            (EntityDiscriminants::Interface, all_ops.clone()),
            (EntityDiscriminants::Vlan, all_ops.clone()),
            (EntityDiscriminants::Tag, all_ops),
        ]))
    }

    async fn handle(&self, events: Vec<Event<EntityOperation>>) -> Result<(), Error> {
        if events.is_empty() {
            return Ok(());
        }

        let mut affected_sites: HashSet<Uuid> = HashSet::new();

        for event in events {
            // For org-scoped events (e.g., Tag changes), fan out to every
            // site in the org so live consumers refetch.
            match event.scope.organization() {
                Some(ScopeOrganization::Site(site_id)) => {
                    affected_sites.insert(site_id);
                }
                Some(ScopeOrganization::Org(org_id)) => {
                    let nets = self
                        .site_service
                        .get_all(
                            StorageFilter::<crate::server::sites::r#impl::Site>::new_from_org_id(
                                &org_id,
                            ),
                        )
                        .await?;
                    for n in nets {
                        affected_sites.insert(n.id);
                    }
                }
                None => {}
            }
        }

        // Broadcast live-update pings. Clients refetch and rebuild the graph on
        // request. The SSE handler filters by user site_ids before forwarding.
        for site_id in &affected_sites {
            let _ = self.live_update_tx.send(*site_id).inspect_err(|e| {
                tracing::debug!(
                    site_id = %site_id,
                    "Live-update broadcast skipped (no receivers): {e}"
                )
            });
        }

        Ok(())
    }

    fn debounce_window_ms(&self) -> u64 {
        200
    }
}

inventory::submit!(SubscriberRegistration::new::<
    TopologyService,
    EntityOperation,
>());
