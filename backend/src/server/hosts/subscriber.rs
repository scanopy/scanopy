use async_trait::async_trait;

use crate::daemon::discovery::types::base::DiscoveryPhase;
use crate::server::hosts::service::HostService;
use crate::server::shared::events::bus::{EventFilter, EventSubscriber};
use crate::server::shared::events::types::Event;

#[async_trait]
impl EventSubscriber for HostService {
    fn event_filter(&self) -> EventFilter {
        EventFilter::discovery_only(Some(vec![DiscoveryPhase::Complete]))
    }

    async fn handle_events(&self, events: Vec<Event>) -> Result<(), anyhow::Error> {
        for event in events {
            if let Event::Discovery(discovery_event) = event
                && discovery_event.phase == DiscoveryPhase::Complete
            {
                // Resolve LLDP/CDP neighbor links — purely server-side DB operation,
                // works for all daemon modes (DaemonPoll and ServerPoll).
                if let Err(e) = self.resolve_lldp_links(discovery_event.network_id).await {
                    tracing::warn!(
                        session_id = %discovery_event.session_id,
                        network_id = %discovery_event.network_id,
                        error = %e,
                        "Failed to resolve LLDP links after discovery completion"
                    );
                }
                // Resolve FDB single-MAC ports after LLDP/CDP (lower priority)
                if let Err(e) = self.resolve_fdb_links(discovery_event.network_id).await {
                    tracing::warn!(
                        session_id = %discovery_event.session_id,
                        network_id = %discovery_event.network_id,
                        error = %e,
                        "Failed to resolve FDB links after discovery completion"
                    );
                }
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "host-discovery-events"
    }
}
