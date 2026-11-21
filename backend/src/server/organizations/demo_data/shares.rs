use super::*;
use crate::server::topology::types::grouping::EdgeColoring;

// ============================================================================
// Shares
// ============================================================================

pub(super) fn generate_shares(
    topologies: &[Topology],
    sites: &[Site],
    user_id: Uuid,
    now: DateTime<Utc>,
) -> Vec<Share> {
    let hq_site = sites.iter().find(|n| n.base.name == "Headquarters");
    let hq_topology = hq_site.and_then(|net| topologies.iter().find(|t| t.base.site_id == net.id));

    let mut shares = Vec::new();

    if let (Some(site), Some(topology)) = (hq_site, hq_topology) {
        if let Ok(id) = Uuid::parse_str("a1b2c3d4-e5f6-7890-abcd-ef1234567890") {
            shares.push(Share {
                // Fixed UUID for demo share — used in onboarding embed
                id,
                created_at: now,
                updated_at: now,
                base: ShareBase {
                    topology_id: topology.id,
                    site_id: site.id,
                    created_by: user_id,
                    name: "HQ Public View".to_string(),
                    is_enabled: true,
                    expires_at: None,
                    password_hash: None,
                    password: None,
                    allowed_domains: None,
                    options: ShareOptions {
                        show_inspect_panel: false,
                        show_zoom_controls: false,
                        show_export_button: false,
                        show_minimap: false,
                        edge_coloring: None,
                    },
                    enabled_views: None,
                },
            });
        }

        if let Ok(id) = Uuid::parse_str("b1b2c3d4-e5f6-7890-abcd-ef1234567890") {
            shares.push(Share {
                // Fixed UUID for demo share — used on website
                id,
                created_at: now,
                updated_at: now,
                base: ShareBase {
                    topology_id: topology.id,
                    site_id: site.id,
                    created_by: user_id,
                    name: "HQ Public View - With Inspect Panel".to_string(),
                    is_enabled: true,
                    expires_at: None,
                    password_hash: None,
                    password: None,
                    allowed_domains: None,
                    options: ShareOptions {
                        show_inspect_panel: true,
                        show_zoom_controls: false,
                        show_export_button: false,
                        show_minimap: false,
                        edge_coloring: None,
                    },
                    enabled_views: None,
                },
            });
        }
    }

    shares
}

/// Fixed UUID of the Water Utility's L2 share, so its link stays the same across demo resets.
pub const WATER_UTILITY_L2_SHARE_ID: Uuid = Uuid::from_u128(0xc1b2c3d4_e5f6_7890_abcd_ef1234567890);

/// A share of the Water Utility site's L2 view alone, with the inspect panel and zoom controls,
/// so a link opens straight onto the rings and each link can be traced.
pub(super) fn water_utility_l2_share(
    topology: &Topology,
    user_id: Uuid,
    now: DateTime<Utc>,
) -> Share {
    use crate::server::topology::types::views::TopologyView;
    Share {
        id: WATER_UTILITY_L2_SHARE_ID,
        created_at: now,
        updated_at: now,
        base: ShareBase {
            topology_id: topology.id,
            site_id: topology.base.site_id,
            created_by: user_id,
            name: "Water Utility L2 Rings".to_string(),
            is_enabled: true,
            expires_at: None,
            password_hash: None,
            password: None,
            allowed_domains: None,
            options: ShareOptions {
                show_inspect_panel: true,
                show_zoom_controls: true,
                show_export_button: false,
                show_minimap: true,
                edge_coloring: Some(EdgeColoring::Speed),
            },
            enabled_views: Some(vec![TopologyView::L2Physical]),
        },
    }
}
