use super::*;

// ============================================================================
// Shares
// ============================================================================

pub(super) fn generate_shares(
    topologies: &[Topology],
    networks: &[Network],
    user_id: Uuid,
    now: DateTime<Utc>,
) -> Vec<Share> {
    let hq_network = networks.iter().find(|n| n.base.name == "Headquarters");
    let hq_topology =
        hq_network.and_then(|net| topologies.iter().find(|t| t.base.network_id == net.id));

    let mut shares = Vec::new();

    if let (Some(network), Some(topology)) = (hq_network, hq_topology) {
        if let Ok(id) = Uuid::parse_str("a1b2c3d4-e5f6-7890-abcd-ef1234567890") {
            shares.push(Share {
                // Fixed UUID for demo share — used in onboarding embed
                id,
                created_at: now,
                updated_at: now,
                base: ShareBase {
                    topology_id: topology.id,
                    network_id: network.id,
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
                    network_id: network.id,
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
                    },
                    enabled_views: None,
                },
            });
        }
    }

    shares
}
