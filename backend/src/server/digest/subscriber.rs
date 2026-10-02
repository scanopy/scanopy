//! Subscriber on the historical-Discovery `EntityOperation::Created` event.
//! Filters to terminal-Complete sessions (the foundation worker publishes the
//! same event for `Failed` and `Cancelled`; v1 of the digest only emails on
//! successful completion). Calls `DiscoveryDigestService::compute_and_publish`
//! which fans out to the email subscriber via a separate event.

use std::collections::HashMap;

use async_trait::async_trait;

use crate::daemon::discovery::types::base::DiscoveryPhase;
use crate::server::daemons::r#impl::api::ScannedEntityIds;
use crate::server::digest::service::DiscoveryDigestService;
use crate::server::shared::entities::{Entity, EntityDiscriminants};
use crate::server::shared::events::registry::SubscriberRegistration;
use crate::server::shared::events::traits::{EntityEventFilter, Event, NonRetryable, Subscriber};
use crate::server::shared::events::types::{EntityOperation, EntityOperationDiscriminants};

#[async_trait]
impl Subscriber<EntityOperation> for DiscoveryDigestService {
    fn filter(&self) -> EntityEventFilter {
        EntityEventFilter::by_entity(HashMap::from([(
            EntityDiscriminants::Discovery,
            Some(vec![EntityOperationDiscriminants::Created]),
        )]))
    }

    async fn handle(&self, events: Vec<Event<EntityOperation>>) -> anyhow::Result<()> {
        let mut failures = Vec::new();
        for event in events {
            let Entity::Discovery(discovery) = event.scope.entity_type() else {
                continue;
            };
            let Some(results) = discovery.base.run_type.historical_results() else {
                continue;
            };
            if results.phase != DiscoveryPhase::Complete {
                continue;
            }
            // A rescan is user-initiated and watched live in the UI; mailing a
            // digest about one host is noise. Scheduled sweeps are what the
            // digest summarises.
            if results.discovery_type.rescan_target_host_id().is_some() {
                continue;
            }
            // ScannedEntityIds carries the daemon-reported, server-resolved
            // canonical IDs of every entity touched this session. We use it
            // to detect "removed" children (live on a scanned host but absent
            // from the scan response). It's `Some` only while the in-memory
            // event scope is live — by the time the historical Discovery row
            // is read back from the DB, `scanned` has been stripped.
            let empty = ScannedEntityIds::default();
            let scanned = results.scanned.as_ref().unwrap_or(&empty);
            if let Err(e) = self.compute_and_publish(results, scanned).await {
                failures.push(anyhow::anyhow!(
                    "discovery digest for session {}: {e:#}",
                    results.session_id
                ));
            }
        }
        // `compute_and_publish` may have published the digest event (and so sent its emails)
        // before failing; a re-run would send them again.
        NonRetryable::from_failures(failures).map_or(Ok(()), |e| Err(e.into()))
    }
}

inventory::submit!(SubscriberRegistration::new::<
    DiscoveryDigestService,
    EntityOperation,
>());
