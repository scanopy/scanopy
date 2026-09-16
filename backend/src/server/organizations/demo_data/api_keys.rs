//! Daemon API keys and user API keys.

use super::*;

pub(super) fn generate_api_keys(networks: &[Network], now: DateTime<Utc>) -> Vec<DaemonApiKey> {
    let find_network = |name: &str| {
        networks
            .iter()
            .find(|n| n.base.name.contains(name))
            .unwrap()
    };

    vec![
        DaemonApiKey {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: DaemonApiKeyBase {
                key: format!("demo_hq_{}", Uuid::new_v4().simple()),
                name: "HQ Daemon Key".to_string(),
                last_used: Some(now),
                expires_at: None,
                network_id: find_network("Headquarters").id,
                is_enabled: true,
                tags: vec![],
                daemon_id: None,
                plaintext: None,
            },
        },
        DaemonApiKey {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: DaemonApiKeyBase {
                key: format!("demo_dc_{}", Uuid::new_v4().simple()),
                name: "DC Daemon Key".to_string(),
                last_used: Some(now),
                expires_at: None,
                network_id: find_network("Data Center").id,
                is_enabled: true,
                tags: vec![],
                daemon_id: None,
                plaintext: None,
            },
        },
    ]
}

pub(super) fn generate_user_api_keys(
    networks: &[Network],
    organization_id: Uuid,
    now: DateTime<Utc>,
) -> Vec<(UserApiKey, Vec<Uuid>)> {
    use super::super::handlers::DEMO_USER_ID;

    let network_ids: Vec<Uuid> = networks.iter().map(|n| n.id).collect();
    let (_plaintext, hashed) = generate_api_key_for_storage(ApiKeyType::User);

    vec![(
        UserApiKey {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: UserApiKeyBase {
                key: hashed,
                name: "Monitoring Integration Key".to_string(),
                user_id: DEMO_USER_ID,
                organization_id,
                permissions: UserOrgPermissions::Member,
                last_used: None,
                expires_at: None,
                is_enabled: true,
                tags: vec![],
                network_ids: vec![], // hydrated by create_with_networks
            },
        },
        network_ids,
    )]
}
