//! Daemon API keys and user API keys.

use super::*;

pub(super) fn generate_api_keys(daemons: &[Daemon], now: DateTime<Utc>) -> Vec<DaemonApiKey> {
    // One key per daemon, bound 1:1 as server-side provisioning binds them: the key names its
    // daemon, and `bind_daemon_api_keys` points each daemon back at its key.
    daemons
        .iter()
        .map(|daemon| {
            let (_plaintext, hashed) = generate_api_key_for_storage(ApiKeyType::Daemon);
            DaemonApiKey {
                id: Uuid::new_v4(),
                created_at: now,
                updated_at: now,
                base: DaemonApiKeyBase {
                    key: hashed,
                    name: format!("{} API Key", daemon.base.name),
                    last_used: Some(now),
                    expires_at: None,
                    site_id: daemon.base.site_id,
                    is_enabled: true,
                    tags: vec![],
                    daemon_id: Some(daemon.id),
                    // Both demo daemons poll the server, so the server holds no plaintext.
                    plaintext: None,
                },
            }
        })
        .collect()
}

/// Point each daemon at the key bound to it.
pub(super) fn bind_daemon_api_keys(daemons: &mut [Daemon], api_keys: &[DaemonApiKey]) {
    for daemon in daemons {
        daemon.base.api_key_id = api_keys
            .iter()
            .find(|key| key.base.daemon_id == Some(daemon.id))
            .map(|key| key.id);
    }
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
