use crate::server::{
    auth::middleware::permissions::{Authorized, InOrganization},
    config::AppState,
    live_updates::types::{LiveTopologyUpdate, LiveUpdate},
    shared::{
        services::traits::CrudService,
        sse::until_shutdown,
        types::api::{ApiError, ApiResult},
    },
};
use axum::{
    extract::State,
    response::{
        Sse,
        sse::{Event, KeepAlive},
    },
    routing::get,
};
use futures::Stream;
use std::{convert::Infallible, sync::Arc};
use tokio::sync::broadcast::error::RecvError;
use utoipa_axum::router::OpenApiRouter;

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    // SSE endpoint (not well-supported by OpenAPI). `LiveUpdate` is registered in the spec's
    // components so the UI types its events from it.
    OpenApiRouter::new().route("/stream", get(live_updates_stream))
}

/// SSE stream of `LiveUpdate` events for the caller's sites: discovery progress, daemon check-ins
/// that change a version or health, and topology changes when the plan includes the topology view. The plan is read at connect, so the app
/// reconnects when the org's topology access changes. The caller's own access is read per event,
/// so a revoked site stops at once, and the stream ends when the caller is gone.
async fn live_updates_stream(
    State(state): State<Arc<AppState>>,
    auth: Authorized<InOrganization>,
) -> ApiResult<Sse<impl Stream<Item = Result<Event, Infallible>>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::internal_error("No organization"))?;
    let organization = state
        .services
        .organization_service
        .get_by_id(&organization_id)
        .await?
        .ok_or_else(|| ApiError::forbidden("Organization not found"))?;
    let topology_on_plan = !state.config.enforces_plan_limits()
        || organization.base.plan.features().includes_topology();

    let mut discovery_rx = state.services.discovery_service.subscribe();
    let mut topology_rx = state
        .services
        .topology_service
        .subscribe_live_topology_updates();
    let mut daemon_rx = state
        .services
        .daemon_service
        .subscribe_live_daemon_updates();
    let caller = auth.into_entity();
    let events_state = state.clone();

    let stream = async_stream::stream! {
        loop {
            let update = tokio::select! {
                received = discovery_rx.recv() => match received {
                    Ok(update) => LiveUpdate::Discovery(Box::new(update)),
                    Err(RecvError::Lagged(n)) => {
                        tracing::warn!("Live-update stream lagged by {} discovery messages", n);
                        continue;
                    }
                    Err(RecvError::Closed) => break,
                },
                received = topology_rx.recv() => match received {
                    Ok(site_id) => LiveUpdate::Topology(LiveTopologyUpdate { site_id }),
                    Err(RecvError::Lagged(n)) => {
                        tracing::warn!("Live-update stream lagged by {} topology messages", n);
                        continue;
                    }
                    Err(RecvError::Closed) => break,
                },
                received = daemon_rx.recv() => match received {
                    Ok(update) => LiveUpdate::Daemon(update),
                    Err(RecvError::Lagged(n)) => {
                        tracing::warn!("Live-update stream lagged by {} daemon messages", n);
                        continue;
                    }
                    Err(RecvError::Closed) => break,
                },
            };
            let access = match caller.current_access(&events_state.services).await {
                Ok(Some(access)) => access,
                Ok(None) => break,
                Err(e) => {
                    tracing::warn!("Skipped a live update: {e}");
                    continue;
                }
            };
            if update.is_visible_to(&access, topology_on_plan) {
                let json = serde_json::to_string(&update).unwrap_or_default();
                yield Ok(Event::default().data(json));
            }
        }
    };

    Ok(Sse::new(until_shutdown(stream, &state.shutdown)).keep_alive(KeepAlive::default()))
}
