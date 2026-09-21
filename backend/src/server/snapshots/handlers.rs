//! Snapshot HTTP handlers.
//!
//! `POST /snapshots` is custom: it acquires the discovery snapshot lock,
//! creates the snapshot row via the standard generic create path (which emits
//! `EntityOperation::Created` — the topology subscriber listens for this and
//! inserts the snapshot's topology row), runs `run_close_and_clone` to stamp
//! every Snapshotable entity row with `snapshot_id` + close them, and releases
//! the lock on every path.
//!
//! `GET /snapshots?network_id=X` and `DELETE /snapshots/{id}` use the standard
//! generic CRUD handlers; the cascade FKs on closed entity rows + topology rows
//! reap everything tied to the deleted snapshots automatically.

use std::sync::Arc;

use axum::extract::State;
use axum::response::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::server::{
    auth::middleware::{
        features::{RequireFeature, TakeSnapshotFeature},
        permissions::{Authorized, Member},
    },
    config::AppState,
    shared::events::{
        traits::{Event, OrgScope},
        types::{OnboardingOperation, OnboardingOperationDiscriminants},
    },
    shared::{
        services::traits::CrudService,
        storage::traits::Entity,
        types::api::{ApiError, ApiErrorResponse, ApiResponse, ApiResult},
    },
    snapshots::types::base::{Snapshot, SnapshotBase},
};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateSnapshotRequest {
    /// The network this entity belongs to.
    pub network_id: Uuid,
}

// Generated handlers for generic CRUD operations. `create` is hand-rolled below
// because it needs the discovery lock + close-and-clone + onboarding event.
mod generated {
    use super::*;
    crate::crud_get_all_handler!(Snapshot);
    crate::crud_get_by_id_handler!(Snapshot);
    crate::crud_delete_handler!(Snapshot);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(generated::get_all, create_snapshot))
        .routes(routes!(generated::get_by_id, generated::delete))
}

/// Take a snapshot of the current live topology + entity state for a network.
/// Acquires the discovery snapshot lock, creates the snapshots row, runs
/// close-and-clone to stamp every Snapshotable entity row with `snapshot_id`
/// and close them. The topology subscriber inserts the snapshot's topology
/// row off the back of the `Snapshot::Created` event.
#[utoipa::path(
    post,
    path = "",
    tag = Snapshot::ENTITY_NAME_PLURAL,
    request_body = CreateSnapshotRequest,
    responses(
        (status = 200, description = "Snapshot created", body = ApiResponse<Snapshot>),
        (status = 402, description = "Snapshots not available on plan", body = ApiErrorResponse),
        (status = 409, description = "Network is busy with discovery; retry shortly", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn create_snapshot(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    RequireFeature { .. }: RequireFeature<TakeSnapshotFeature>,
    Json(req): Json<CreateSnapshotRequest>,
) -> ApiResult<Json<ApiResponse<Snapshot>>> {
    // Tenant isolation: require the network to be in the caller's set.
    let network_ids = auth.network_ids();
    if !network_ids.contains(&req.network_id) {
        return Err(ApiError::forbidden(
            "Network not accessible to the current user",
        ));
    }

    // Discovery lock: blocks queueing of new discoveries on this network for
    // the duration of close-and-clone, and rejects this request if a scan is
    // currently in-flight. The reservation releases the network even if this
    // request is abandoned before the release below.
    let Some(reservation) = state
        .services
        .discovery_service
        .try_reserve_network_for_snapshot(req.network_id)
        .await
    else {
        return Err(ApiError::conflict(
            "Network is busy with an in-flight discovery; retry shortly.",
        ));
    };

    let result = async {
        let snapshot = Snapshot {
            base: SnapshotBase::new(req.network_id, Utc::now(), auth.user_id()),
            ..Default::default()
        };

        // INSERT the snapshots row.
        let created = state
            .services
            .snapshot_service
            .create(snapshot, auth.entity.clone())
            .await
            .map_err(ApiError::from)?;

        // Close-and-clone the live entity set, stamping `snapshot_id` on each
        // closed copy. Single transaction; rolls back on any failure. No
        // topology row is built — the snapshot's graph is built on request from
        // these closed copies by the read path.
        if let Err(e) = state
            .services
            .snapshot_service
            .run_close_and_clone(created.base.network_id, created.base.taken_at, created.id)
            .await
        {
            // The response carries the message, but a failure here rolls the
            // clone transaction back and leaves nothing behind to diagnose from
            // — GH #687 was reported with no server-side record of it at all.
            tracing::error!(
                snapshot_id = %created.id,
                network_id = %created.base.network_id,
                error = ?e,
                "Snapshot close-and-clone failed; no rows were captured",
            );

            // The snapshots row is INSERTed before close-and-clone and is not
            // part of its transaction, so a failed clone otherwise leaves a
            // snapshot that captured nothing — selectable in the picker and
            // rendering an empty topology, which reads as "the network was
            // empty at that time" rather than as the failure it was.
            if let Err(cleanup) = state
                .services
                .snapshot_service
                .delete(&created.id, auth.entity.clone())
                .await
            {
                tracing::error!(
                    snapshot_id = %created.id,
                    error = ?cleanup,
                    "Could not remove the snapshot row after a failed close-and-clone; it captured \
                     no entities and should be deleted by hand",
                );
            }

            return Err(ApiError::internal_error(&e.to_string()));
        }

        Ok::<Snapshot, ApiError>(created)
    }
    .await;

    reservation.release().await;

    let created = result?;

    // First-snapshot onboarding event. Best-effort: failures here don't fail
    // the request — same fire-and-forget pattern as SecondNetworkCreated in
    // networks/handlers.rs.
    if let Some(organization_id) = auth.organization_id()
        && let Ok(Some(org)) = state
            .services
            .organization_service
            .get_by_id(&organization_id)
            .await
        && org.not_onboarded(&OnboardingOperationDiscriminants::FirstSnapshotCreated)
    {
        let _ = state
            .services
            .event_bus
            .publish(Event::new(
                OrgScope { organization_id },
                OnboardingOperation::FirstSnapshotCreated {
                    snapshot_id: created.id,
                    network_id: created.base.network_id,
                },
                auth.entity.clone(),
            ))
            .await;
    }

    Ok(Json(ApiResponse::success(created)))
}
