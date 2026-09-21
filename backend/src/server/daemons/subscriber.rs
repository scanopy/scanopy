//! Event subscriber implementation for DaemonService.
//!
//! Subscribes to discovery events with `Started` / `Cancelled` phases and
//! handles them for ServerPoll-mode daemons.

use async_trait::async_trait;
use uuid::Uuid;

use crate::daemon::discovery::types::base::{DiscoveryPhase, DiscoveryPhaseDiscriminants};
use crate::server::daemons::r#impl::base::DaemonMode;
use crate::server::daemons::service::{DaemonHttpError, DaemonService};
use crate::server::shared::events::registry::SubscriberRegistration;
use crate::server::shared::events::traits::{Event, EventFilter, Subscriber};
use crate::server::shared::services::traits::CrudService;

/// What happened when the server tried to tell a daemon to stop a session.
#[derive(Debug)]
pub(crate) enum CancelDelivery {
    /// The daemon accepted the cancellation.
    Delivered,
    /// A DaemonPoll daemon: it reads the cancellation when it next polls for work.
    PulledByDaemon,
    /// There is no usable daemon to tell: its row is gone, it is marked unreachable, or it has no
    /// usable API key.
    Undeliverable(String),
    /// The daemon answered that it is not running the session. It may have just finished it.
    NotRunning,
    /// The request failed after its retries.
    Failed(anyhow::Error),
}

impl CancelDelivery {
    /// A bounded label for logs.
    pub(crate) fn outcome(&self) -> &'static str {
        match self {
            CancelDelivery::Delivered => "delivered",
            CancelDelivery::PulledByDaemon => "pulled_by_daemon",
            CancelDelivery::Undeliverable(_) => "undeliverable",
            CancelDelivery::NotRunning => "not_running",
            CancelDelivery::Failed(_) => "failed",
        }
    }

    pub(crate) fn detail(&self) -> Option<String> {
        match self {
            CancelDelivery::Undeliverable(why) => Some(why.clone()),
            CancelDelivery::Failed(e) => Some(format!("{e:#}")),
            _ => None,
        }
    }
}

impl DaemonService {
    /// Tell a ServerPoll daemon to stop a session. DaemonPoll daemons are not contacted; they pick
    /// cancellations up from the server when they poll.
    pub(crate) async fn deliver_cancellation(
        &self,
        daemon_id: Uuid,
        session_id: Uuid,
    ) -> CancelDelivery {
        let daemon = match self.get_by_id(&daemon_id).await {
            Ok(Some(daemon)) => daemon,
            Ok(None) => {
                return CancelDelivery::Undeliverable("the daemon no longer exists".to_string());
            }
            Err(e) => return CancelDelivery::Failed(e),
        };

        if daemon.base.mode != DaemonMode::ServerPoll {
            return CancelDelivery::PulledByDaemon;
        }
        if daemon.base.is_unreachable {
            return CancelDelivery::Undeliverable("the daemon is marked unreachable".to_string());
        }

        // Legacy daemons (< v0.14.0) take the request without auth.
        let api_key = if daemon.supports_full_server_poll() {
            match self.get_daemon_api_key(&daemon).await {
                Ok(key) => Some(key),
                Err(e) => {
                    return CancelDelivery::Undeliverable(format!(
                        "the daemon's API key is unusable: {e}"
                    ));
                }
            }
        } else {
            None
        };

        match self
            .send_discovery_cancellation_to_daemon(&daemon, api_key.as_deref(), session_id)
            .await
        {
            Ok(()) => CancelDelivery::Delivered,
            Err(e)
                if e.downcast_ref::<DaemonHttpError>()
                    .is_some_and(|h| h.status == reqwest::StatusCode::CONFLICT) =>
            {
                CancelDelivery::NotRunning
            }
            Err(e) => CancelDelivery::Failed(e),
        }
    }
}

#[async_trait]
impl Subscriber<DiscoveryPhase> for DaemonService {
    fn filter(&self) -> EventFilter<DiscoveryPhase> {
        EventFilter::ops(vec![
            DiscoveryPhaseDiscriminants::Started,
            DiscoveryPhaseDiscriminants::Cancelled,
        ])
    }

    async fn handle(&self, events: Vec<Event<DiscoveryPhase>>) -> Result<(), anyhow::Error> {
        for event in events {
            let daemon_id = event.scope.daemon_id;
            let session_id = event.scope.session_id;

            match event.operation {
                DiscoveryPhase::Started => {
                    tracing::debug!(
                        daemon_id = %daemon_id,
                        session_id = %session_id,
                        "Discovery session queued — will dispatch when daemon reports ready"
                    );
                }
                DiscoveryPhase::Cancelled => {
                    let delivery = self.deliver_cancellation(daemon_id, session_id).await;
                    match &delivery {
                        CancelDelivery::Delivered | CancelDelivery::PulledByDaemon => {
                            tracing::info!(
                                daemon_id = %daemon_id,
                                session_id = %session_id,
                                delivery = delivery.outcome(),
                                "Cancellation passed to the daemon"
                            );
                        }
                        _ => {
                            tracing::warn!(
                                daemon_id = %daemon_id,
                                session_id = %session_id,
                                delivery = delivery.outcome(),
                                detail = delivery.detail(),
                                "Cancellation could not be passed to the daemon"
                            );
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<DaemonService, DiscoveryPhase>());
