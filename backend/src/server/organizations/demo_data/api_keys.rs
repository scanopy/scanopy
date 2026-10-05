//! Daemon API keys and user API keys.

use super::*;

pub(super) fn generate_api_keys(sites: &[Site], now: DateTime<Utc>) -> Vec<DaemonApiKey> {
    let find_site = |name: &str| sites.iter().find(|n| n.base.name.contains(name)).unwrap();

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
                site_id: find_site("Headquarters").id,
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
                site_id: find_site("Data Center").id,
                is_enabled: true,
                tags: vec![],
                daemon_id: None,
                plaintext: None,
            },
        },
    ]
}

pub(super) fn generate_user_api_keys(
    sites: &[Site],
    organization_id: Uuid,
    now: DateTime<Utc>,
) -> Vec<(UserApiKey, Vec<Uuid>)> {
    use super::super::handlers::DEMO_USER_ID;

    let site_ids: Vec<Uuid> = sites.iter().map(|n| n.id).collect();
    let hq_id = sites
        .iter()
        .find(|n| n.base.name.contains("Headquarters"))
        .map(|n| n.id)
        .unwrap();
    let (_plaintext, hashed) = generate_api_key_for_storage(ApiKeyType::User);
    let (_plaintext, asset_export_hashed) = generate_api_key_for_storage(ApiKeyType::User);

    vec![
        // Read-only, and limited to HQ: the office asset register only tracks HQ equipment.
        (
            UserApiKey {
                id: Uuid::new_v4(),
                created_at: now,
                updated_at: now,
                base: UserApiKeyBase {
                    key: asset_export_hashed,
                    name: "HQ Asset Register Export".to_string(),
                    user_id: DEMO_USER_ID,
                    organization_id,
                    permissions: UserOrgPermissions::Viewer,
                    last_used: Some(now - Duration::days(1)),
                    expires_at: Some(now + Duration::days(180)),
                    is_enabled: true,
                    tags: vec![],
                    site_ids: vec![], // hydrated by create_with_sites
                },
            },
            vec![hq_id],
        ),
        (
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
                    site_ids: vec![], // hydrated by create_with_sites
                },
            },
            site_ids,
        ),
    ]
}
