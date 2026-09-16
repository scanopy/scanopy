//! Networks

use super::*;

pub(super) fn generate_networks(
    organization_id: Uuid,
    tags: &[Tag],
    _credentials: &[Credential],
    now: DateTime<Utc>,
) -> Vec<Network> {
    let production_tag = tags
        .iter()
        .find(|t| t.base.name == "Production")
        .map(|t| t.id);

    // Note: credential_ids are hydrated from junction tables, not stored on the network.
    // Network-credential associations would be created via credential_service.set_network_credentials().

    // Stagger timestamps so networks sort in predictable order (Headquarters first)
    vec![
        Network {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: NetworkBase {
                name: "Headquarters".to_string(),
                organization_id,
                tags: production_tag.into_iter().collect(),
                credential_ids: vec![],
                stale_after_hours: Some(48),
            },
            effective_stale_after_hours: 48,
        },
        Network {
            id: Uuid::new_v4(),
            created_at: now + chrono::Duration::seconds(1),
            updated_at: now + chrono::Duration::seconds(1),
            base: NetworkBase {
                name: "Data Center".to_string(),
                organization_id,
                tags: production_tag.into_iter().collect(),
                credential_ids: vec![],
                stale_after_hours: None,
            },
            effective_stale_after_hours: DEFAULT_STALE_AFTER_HOURS,
        },
    ]
}
