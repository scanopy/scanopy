use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::base::{Share, ShareOptions};
use crate::server::topology::types::{api::TopologyData, base::Topology, views::TopologyView};

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateUpdateShareRequest {
    /// The share to create or replace.
    pub share: Share,
}

/// Public share metadata (returned without authentication)
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PublicShareMetadata {
    /// Server-assigned unique identifier.
    pub id: Uuid,
    /// Human-facing name for this share.
    pub name: String,
    /// Whether a password must be supplied before the topology is returned.
    pub requires_password: bool,
    /// What the viewer can see and do.
    pub options: ShareOptions,
    /// Resolved list of available topology views for this share.
    /// Filtered by both share configuration and data availability.
    /// First element is the default view.
    pub enabled_views: Vec<TopologyView>,
}

impl PublicShareMetadata {
    pub fn new(share: &Share, enabled_views: Vec<TopologyView>) -> Self {
        Self {
            id: share.id,
            name: share.base.name.clone(),
            requires_password: share.requires_password(),
            options: share.base.options.clone(),
            enabled_views,
        }
    }
}

/// Export feature flags derived from the share creator's billing plan
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ExportFeatures {
    /// Viewer may export the diagram as PNG.
    pub png_export: bool,
    /// Viewer may export the diagram as SVG.
    pub svg_export: bool,
    /// Viewer may export the diagram as Mermaid.
    pub mermaid_export: bool,
    /// Viewer may export the diagram for Confluence.
    pub confluence_export: bool,
    /// Viewer may export the diagram as PDF.
    pub pdf_export: bool,
    /// Viewer may export the diagram as standalone HTML.
    pub html_export: bool,
    /// Exports omit the Scanopy attribution line.
    pub remove_created_with: bool,
}

/// Share with topology data (returned after authentication/verification).
///
/// Returns the slim topology row (`{ id, site_id, options }`) plus the
/// `TopologyData` bundle (entities + the per-view graph built on request). The
/// share viewer composes these with the same `toRenderableTopology` the app
/// uses — no server-side merge.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ShareWithTopology {
    /// Public metadata for the share itself.
    pub share: PublicShareMetadata,
    /// The shared topology record.
    pub topology: Topology,
    /// Entities and graph for the requested view.
    pub data: TopologyData,
    /// Which exports the share creator's plan allows.
    pub export_features: ExportFeatures,
}

/// Access token returned after successful password verification.
///
/// The token is an HS256 JWT tied to the share's `password_hash` — changing
/// the share password implicitly invalidates all outstanding tokens.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ShareAccessTokenResponse {
    /// Bearer token granting access to this share for the rest of the session.
    pub access_token: String,
    /// When this record stops being valid.
    pub expires_at: DateTime<Utc>,
}
