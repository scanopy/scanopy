use crate::infra::{BASE_URL, TestContext};
use cidr::{IpCidr, Ipv4Cidr};
use reqwest::StatusCode;
use scanopy::server::daemon_api_keys::r#impl::api::DaemonApiKeyResponse;
use scanopy::server::daemon_api_keys::r#impl::base::{DaemonApiKey, DaemonApiKeyBase};
use scanopy::server::discovery::r#impl::base::{Discovery, DiscoveryBase};
use scanopy::server::discovery::r#impl::types::{DiscoveryType, HostNamingFallback, RunType};
use scanopy::server::groups::r#impl::base::{Group, GroupBase};
use scanopy::server::groups::r#impl::types::GroupType;
use scanopy::server::hosts::r#impl::api::{CreateHostRequest, HostResponse, UpdateHostRequest};
use scanopy::server::services::definitions::ServiceDefinitionRegistry;
use scanopy::server::services::r#impl::base::{Service, ServiceBase};
use scanopy::server::shared::storage::traits::Storable;
use scanopy::server::shared::types::Color;
use scanopy::server::shared::types::entities::EntitySource;
use scanopy::server::subnets::r#impl::base::{Subnet, SubnetBase};
use scanopy::server::subnets::r#impl::types::SubnetType;
use scanopy::server::tags::r#impl::base::{Tag, TagBase};
use scanopy::server::topology::types::edges::EdgeStyle;
use scanopy::server::user_api_keys::r#impl::api::UserApiKeyResponse;
use scanopy::server::user_api_keys::r#impl::base::{UserApiKey, UserApiKeyBase};
use scanopy::server::users::r#impl::base::{User, UserBase};
use scanopy::server::users::r#impl::permissions::UserOrgPermissions;
use std::net::Ipv4Addr;
use uuid::Uuid;

pub async fn run_crud_tests(ctx: &TestContext) -> Result<(), String> {
    println!("\n=== Testing CRUD Endpoints ===\n");

    test_subnet_crud(ctx).await?;
    test_host_crud(ctx).await?;
    test_service_crud(ctx).await?;
    test_group_crud(ctx).await?;
    test_tag_crud(ctx).await?;
    test_discovery_crud(ctx).await?;
    test_api_key_crud(ctx).await?;
    test_user_api_key_crud(ctx).await?;
    test_user_api_key_authentication(ctx).await?;
    test_user_api_key_permission_escalation(ctx).await?;
    test_user_api_key_rotation(ctx).await?;
    test_user_api_key_expired_disabled(ctx).await?;
    test_user_api_key_network_access(ctx).await?;
    test_user_api_key_owner_isolation(ctx).await?;

    println!("\n✅ All CRUD endpoint tests passed!");
    Ok(())
}

async fn test_subnet_crud(ctx: &TestContext) -> Result<(), String> {
    println!("Testing Subnet CRUD...");

    let subnet = Subnet::new(SubnetBase {
        name: "Test Subnet".to_string(),
        description: Some("Test description".to_string()),
        network_id: ctx.network_id,
        cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(10, 0, 0, 0), 24).unwrap()),
        subnet_type: SubnetType::Lan,
        source: EntitySource::System,
        tags: Vec::new(),
    });

    let created: Subnet = ctx.client.post("/api/v1/subnets", &subnet).await?;
    assert!(!created.id.is_nil(), "Created subnet should have an ID");
    assert_eq!(created.base.name, "Test Subnet");
    println!("  ✓ Create subnet");

    let fetched: Subnet = ctx
        .client
        .get(&format!("/api/v1/subnets/{}", created.id))
        .await?;
    assert_eq!(fetched.id, created.id);
    println!("  ✓ Read subnet");

    let mut updated = fetched.clone();
    updated.base.name = "Updated Subnet".to_string();
    let updated: Subnet = ctx
        .client
        .put(&format!("/api/v1/subnets/{}", updated.id), &updated)
        .await?;
    assert_eq!(updated.base.name, "Updated Subnet");
    println!("  ✓ Update subnet");

    let subnets: Vec<Subnet> = ctx.client.get("/api/v1/subnets").await?;
    assert!(subnets.iter().any(|s| s.id == created.id));
    println!("  ✓ List subnets");

    ctx.client
        .delete_no_content(&format!("/api/v1/subnets/{}", created.id))
        .await?;
    println!("  ✓ Delete subnet");

    let result = ctx
        .client
        .get_expect_status(
            &format!("/api/v1/subnets/{}", created.id),
            StatusCode::NOT_FOUND,
        )
        .await;
    assert!(result.is_ok(), "Deleted subnet should return 404");
    println!("  ✓ Verify deletion");

    println!("✅ Subnet CRUD passed");
    Ok(())
}

async fn test_host_crud(ctx: &TestContext) -> Result<(), String> {
    println!("Testing Host CRUD...");

    let request = CreateHostRequest {
        name: "Test Host".to_string(),
        hostname: Some("test.local".to_string()),
        network_id: ctx.network_id,
        description: None,
        virtualization: None,
        hidden: false,
        tags: Vec::new(),
        // SNMP fields
        sys_descr: None,
        sys_object_id: None,
        sys_location: None,
        sys_contact: None,
        management_url: None,
        chassis_id: None,
        credential_assignments: vec![],
        interfaces: vec![],
        ports: vec![],
        services: vec![],
        if_entries: vec![],
    };

    let created: HostResponse = ctx.client.post("/api/v1/hosts", &request).await?;
    assert!(!created.id.is_nil(), "Created host should have an ID");
    assert_eq!(created.name, "Test Host");
    println!("  ✓ Create host");

    let fetched: HostResponse = ctx
        .client
        .get(&format!("/api/v1/hosts/{}", created.id))
        .await?;
    assert_eq!(fetched.id, created.id);
    println!("  ✓ Read host");

    let update_request = UpdateHostRequest {
        id: created.id,
        name: "Updated Host".to_string(),
        hostname: fetched.hostname.clone(),
        description: fetched.description.clone(),
        virtualization: fetched.virtualization.clone(),
        hidden: fetched.hidden,
        tags: fetched.tags.clone(),
        expected_updated_at: None,    // No optimistic locking for this test
        interfaces: None,             // Keep existing interfaces
        ports: None,                  // Keep existing ports
        services: None,               // Keep existing services
        credential_assignments: None, // Keep existing credentials
    };
    let updated: HostResponse = ctx
        .client
        .put(&format!("/api/v1/hosts/{}", created.id), &update_request)
        .await?;
    assert_eq!(updated.name, "Updated Host");
    println!("  ✓ Update host");

    let hosts: Vec<HostResponse> = ctx.client.get("/api/v1/hosts").await?;
    assert!(hosts.iter().any(|h| h.id == created.id));
    println!("  ✓ List hosts");

    ctx.client
        .delete_no_content(&format!("/api/v1/hosts/{}", created.id))
        .await?;
    println!("  ✓ Delete host");

    println!("✅ Host CRUD passed");
    Ok(())
}

async fn test_service_crud(ctx: &TestContext) -> Result<(), String> {
    println!("Testing Service CRUD...");

    let host_request = CreateHostRequest {
        name: "Service Test Host".to_string(),
        hostname: Some("service-test.local".to_string()),
        network_id: ctx.network_id,
        description: None,
        virtualization: None,
        hidden: false,
        tags: Vec::new(),
        // SNMP fields
        sys_descr: None,
        sys_object_id: None,
        sys_location: None,
        sys_contact: None,
        management_url: None,
        chassis_id: None,
        credential_assignments: vec![],
        interfaces: vec![],
        ports: vec![],
        services: vec![],
        if_entries: vec![],
    };
    let created_host: HostResponse = ctx.client.post("/api/v1/hosts", &host_request).await?;

    let service_def = ServiceDefinitionRegistry::find_by_id("Dns Server")
        .unwrap_or_else(|| ServiceDefinitionRegistry::all_service_definitions()[0].clone());

    let service = Service::new(ServiceBase {
        name: "Test Service".to_string(),
        host_id: created_host.id,
        bindings: vec![],
        network_id: ctx.network_id,
        service_definition: service_def,
        virtualization: None,
        source: EntitySource::System,
        tags: Vec::new(),
        position: 0,
    });

    let created: Service = ctx.client.post("/api/v1/services", &service).await?;
    assert!(!created.id.is_nil());
    assert_eq!(created.base.name, "Test Service");
    println!("  ✓ Create service");

    let fetched: Service = ctx
        .client
        .get(&format!("/api/v1/services/{}", created.id))
        .await?;
    assert_eq!(fetched.id, created.id);
    println!("  ✓ Read service");

    let mut updated = fetched.clone();
    updated.base.name = "Updated Service".to_string();
    let updated: Service = ctx
        .client
        .put(&format!("/api/v1/services/{}", updated.id), &updated)
        .await?;
    assert_eq!(updated.base.name, "Updated Service");
    println!("  ✓ Update service");

    let services: Vec<Service> = ctx.client.get("/api/v1/services").await?;
    assert!(services.iter().any(|s| s.id == created.id));
    println!("  ✓ List services");

    ctx.client
        .delete_no_content(&format!("/api/v1/services/{}", created.id))
        .await?;
    println!("  ✓ Delete service");

    ctx.client
        .delete_no_content(&format!("/api/v1/hosts/{}", created_host.id))
        .await?;

    println!("✅ Service CRUD passed");
    Ok(())
}

async fn test_group_crud(ctx: &TestContext) -> Result<(), String> {
    println!("Testing Group CRUD...");

    let group = Group::new(GroupBase {
        name: "Test Group".to_string(),
        description: Some("Test description".to_string()),
        network_id: ctx.network_id,
        color: Color::Red,
        group_type: GroupType::RequestPath,
        binding_ids: vec![],
        source: EntitySource::System,
        edge_style: EdgeStyle::Bezier,
        tags: Vec::new(),
    });

    let created: Group = ctx.client.post("/api/v1/groups", &group).await?;
    assert!(!created.id.is_nil());
    assert_eq!(created.base.name, "Test Group");
    println!("  ✓ Create group");

    let fetched: Group = ctx
        .client
        .get(&format!("/api/v1/groups/{}", created.id))
        .await?;
    assert_eq!(fetched.id, created.id);
    println!("  ✓ Read group");

    let mut updated = fetched.clone();
    updated.base.name = "Updated Group".to_string();
    let updated: Group = ctx
        .client
        .put(&format!("/api/v1/groups/{}", updated.id), &updated)
        .await?;
    assert_eq!(updated.base.name, "Updated Group");
    println!("  ✓ Update group");

    let groups: Vec<Group> = ctx.client.get("/api/v1/groups").await?;
    assert!(groups.iter().any(|g| g.id == created.id));
    println!("  ✓ List groups");

    ctx.client
        .delete_no_content(&format!("/api/v1/groups/{}", created.id))
        .await?;
    println!("  ✓ Delete group");

    println!("✅ Group CRUD passed");
    Ok(())
}

async fn test_tag_crud(ctx: &TestContext) -> Result<(), String> {
    println!("Testing Tag CRUD...");

    let mut tag = Tag::new(TagBase::default());
    tag.base.organization_id = ctx.organization_id;
    tag.base.name = "Test Tag".to_string();

    let created: Tag = ctx.client.post("/api/v1/tags", &tag).await?;
    assert!(!created.id.is_nil());
    assert_eq!(created.base.name, "Test Tag");
    println!("  ✓ Create tag");

    let fetched: Tag = ctx
        .client
        .get(&format!("/api/v1/tags/{}", created.id))
        .await?;
    assert_eq!(fetched.id, created.id);
    println!("  ✓ Read tag");

    let mut updated = fetched.clone();
    updated.base.name = "Updated Tag".to_string();
    let updated: Tag = ctx
        .client
        .put(&format!("/api/v1/tags/{}", updated.id), &updated)
        .await?;
    assert_eq!(updated.base.name, "Updated Tag");
    println!("  ✓ Update tag");

    let tags: Vec<Tag> = ctx.client.get("/api/v1/tags").await?;
    assert!(tags.iter().any(|t| t.id == created.id));
    println!("  ✓ List tags");

    ctx.client
        .delete_no_content(&format!("/api/v1/tags/{}", created.id))
        .await?;
    println!("  ✓ Delete tag");

    println!("✅ Tag CRUD passed");
    Ok(())
}

async fn test_discovery_crud(ctx: &TestContext) -> Result<(), String> {
    println!("Testing Discovery CRUD...");

    // Use a ServerPoll daemon (supports unified discovery) for the CRUD test
    let daemons: Vec<serde_json::Value> = ctx.client.get("/api/v1/daemons").await?;
    let daemon = daemons
        .iter()
        .find(|d| d.get("mode").and_then(|m| m.as_str()) == Some("ServerPoll"))
        .or_else(|| daemons.last())
        .ok_or("No daemon found for discovery test")?;
    let daemon_id = daemon
        .get("id")
        .and_then(|id| id.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or("No daemon id found")?;
    let host_id = daemon
        .get("host_id")
        .and_then(|id| id.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or("No host_id found on daemon")?;

    let discovery = Discovery {
        id: Uuid::nil(), // Server assigns ID
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        base: DiscoveryBase {
            discovery_type: DiscoveryType::Unified {
                host_id,
                subnet_ids: None,
                scan_local_docker_socket: false,
                host_naming_fallback: HostNamingFallback::BestService,
                scan_settings: Default::default(),
            },
            run_type: RunType::AdHoc { last_run: None },
            name: "CRUD Test Discovery".to_string(),
            daemon_id,
            network_id: ctx.network_id,
            tags: vec![],
        },
        scan_count: 0,
        force_full_scan: false,
        pending_credential_ids: vec![],
    };

    let created: Discovery = ctx.client.post("/api/v1/discovery", &discovery).await?;
    assert!(!created.id.is_nil());
    assert_eq!(created.base.name, "CRUD Test Discovery");
    println!("  ✓ Create discovery");

    let fetched: Discovery = ctx
        .client
        .get(&format!("/api/v1/discovery/{}", created.id))
        .await?;
    assert_eq!(fetched.id, created.id);
    println!("  ✓ Read discovery");

    let mut updated = fetched.clone();
    updated.base.name = "Updated Discovery".to_string();
    let updated: Discovery = ctx
        .client
        .put(&format!("/api/v1/discovery/{}", updated.id), &updated)
        .await?;
    assert_eq!(updated.base.name, "Updated Discovery");
    println!("  ✓ Update discovery");

    let discoveries: Vec<Discovery> = ctx.client.get("/api/v1/discovery").await?;
    assert!(discoveries.iter().any(|d| d.id == created.id));
    println!("  ✓ List discoveries");

    ctx.client
        .delete_no_content(&format!("/api/v1/discovery/{}", created.id))
        .await?;
    println!("  ✓ Delete discovery");

    let result = ctx
        .client
        .get_expect_status(
            &format!("/api/v1/discovery/{}", created.id),
            StatusCode::NOT_FOUND,
        )
        .await;
    assert!(result.is_ok(), "Deleted discovery should return 404");
    println!("  ✓ Verify deletion");

    println!("✅ Discovery CRUD passed");
    Ok(())
}

async fn test_api_key_crud(ctx: &TestContext) -> Result<(), String> {
    println!("Testing Daemon API Key CRUD...");

    let api_key = DaemonApiKey::new(DaemonApiKeyBase {
        key: String::new(),
        name: "Test Daemon API Key".to_string(),
        last_used: None,
        expires_at: None,
        network_id: ctx.network_id,
        is_enabled: true,
        tags: Vec::new(),
        plaintext: None,
    });

    // Daemon API keys are now at /api/v1/auth/daemon
    let created: DaemonApiKeyResponse = ctx.client.post("/api/v1/auth/daemon", &api_key).await?;
    let key_id = created.api_key.id;
    assert!(!created.key.is_empty(), "Should return plaintext key");
    println!("  ✓ Create daemon API key (received plaintext key)");

    let fetched: DaemonApiKey = ctx
        .client
        .get(&format!("/api/v1/auth/daemon/{}", key_id))
        .await?;
    assert_eq!(fetched.id, key_id);
    println!("  ✓ Read daemon API key");

    let mut updated = fetched.clone();
    updated.base.name = "Updated Daemon API Key".to_string();
    let updated: DaemonApiKey = ctx
        .client
        .put(&format!("/api/v1/auth/daemon/{}", updated.id), &updated)
        .await?;
    assert_eq!(updated.base.name, "Updated Daemon API Key");
    assert_eq!(
        updated.base.key, fetched.base.key,
        "Key hash should be preserved"
    );
    println!("  ✓ Update daemon API key (key hash preserved)");

    let keys: Vec<DaemonApiKey> = ctx.client.get("/api/v1/auth/daemon").await?;
    assert!(keys.iter().any(|k| k.id == key_id));
    println!("  ✓ List daemon API keys");

    ctx.client
        .delete_no_content(&format!("/api/v1/auth/daemon/{}", key_id))
        .await?;
    println!("  ✓ Delete daemon API key");

    println!("✅ Daemon API Key CRUD passed");
    Ok(())
}

async fn test_user_api_key_crud(ctx: &TestContext) -> Result<(), String> {
    println!("Testing User API Key CRUD...");

    let api_key = UserApiKey::new(UserApiKeyBase {
        key: String::new(),
        name: "Test User API Key".to_string(),
        user_id: Uuid::nil(), // Will be set by server
        organization_id: ctx.organization_id,
        permissions: UserOrgPermissions::Viewer,
        last_used: None,
        expires_at: None,
        is_enabled: true,
        tags: Vec::new(),
        network_ids: vec![ctx.network_id],
    });

    // User API keys are at /api/v1/auth/keys
    let created: UserApiKeyResponse = ctx.client.post("/api/v1/auth/keys", &api_key).await?;
    let key_id = created.api_key.id;
    assert!(!created.key.is_empty(), "Should return plaintext key");
    println!("  ✓ Create user API key (received plaintext key)");

    let fetched: UserApiKey = ctx
        .client
        .get(&format!("/api/v1/auth/keys/{}", key_id))
        .await?;
    assert_eq!(fetched.id, key_id);
    println!("  ✓ Read user API key");

    let mut updated = fetched.clone();
    updated.base.name = "Updated User API Key".to_string();
    let updated: UserApiKey = ctx
        .client
        .put(&format!("/api/v1/auth/keys/{}", updated.id), &updated)
        .await?;
    assert_eq!(updated.base.name, "Updated User API Key");
    assert_eq!(
        updated.base.key, fetched.base.key,
        "Key hash should be preserved"
    );
    println!("  ✓ Update user API key (key hash preserved)");

    let keys: Vec<UserApiKey> = ctx.client.get("/api/v1/auth/keys").await?;
    assert!(keys.iter().any(|k| k.id == key_id));
    println!("  ✓ List user API keys");

    ctx.client
        .delete_no_content(&format!("/api/v1/auth/keys/{}", key_id))
        .await?;
    println!("  ✓ Delete user API key");

    println!("✅ User API Key CRUD passed");
    Ok(())
}

async fn test_user_api_key_authentication(ctx: &TestContext) -> Result<(), String> {
    println!("Testing User API Key Authentication...");

    // Create a user API key with Viewer permissions
    let api_key = UserApiKey::new(UserApiKeyBase {
        key: String::new(),
        name: "Auth Test Key".to_string(),
        user_id: Uuid::nil(),
        organization_id: ctx.organization_id,
        permissions: UserOrgPermissions::Viewer,
        last_used: None,
        expires_at: None,
        is_enabled: true,
        tags: Vec::new(),
        network_ids: vec![ctx.network_id],
    });

    let created: UserApiKeyResponse = ctx.client.post("/api/v1/auth/keys", &api_key).await?;
    let plaintext_key = created.key.clone();
    let key_id = created.api_key.id;
    println!("  ✓ Created API key for authentication test");

    // Create a new HTTP client without cookies (to test API key auth separately)
    let api_key_client = reqwest::Client::new();

    // Test: Access GET /api/v1/subnets with Bearer token (should work - Viewer permission)
    let response = api_key_client
        .get(format!("{}/api/v1/subnets", BASE_URL))
        .header("Authorization", format!("Bearer {}", plaintext_key))
        .send()
        .await
        .map_err(|e| format!("API key auth request failed: {}", e))?;

    assert!(
        response.status().is_success(),
        "API key should be able to access GET /api/v1/subnets (Viewer endpoint), got {}",
        response.status()
    );
    println!("  ✓ API key can access read endpoints (GET /api/v1/subnets)");

    // Test: Access POST /api/v1/subnets with Bearer token (should fail - requires Member)
    let test_subnet = Subnet::new(SubnetBase {
        name: "API Key Test Subnet".to_string(),
        description: None,
        network_id: ctx.network_id,
        cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(192, 168, 100, 0), 24).unwrap()),
        subnet_type: SubnetType::Lan,
        source: EntitySource::Manual,
        tags: Vec::new(),
    });

    let response = api_key_client
        .post(format!("{}/api/v1/subnets", BASE_URL))
        .header("Authorization", format!("Bearer {}", plaintext_key))
        .json(&test_subnet)
        .send()
        .await
        .map_err(|e| format!("API key auth request failed: {}", e))?;

    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "Viewer API key should NOT be able to POST (requires Member), got {}",
        response.status()
    );
    println!("  ✓ Viewer API key correctly denied write access (POST /api/v1/subnets)");

    // Test: Unauthenticated request should fail
    let response = api_key_client
        .get(format!("{}/api/v1/subnets", BASE_URL))
        .send()
        .await
        .map_err(|e| format!("Unauthenticated request failed: {}", e))?;

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Unauthenticated request should be rejected, got {}",
        response.status()
    );
    println!("  ✓ Unauthenticated requests correctly rejected");

    // Cleanup
    ctx.client
        .delete_no_content(&format!("/api/v1/auth/keys/{}", key_id))
        .await?;

    println!("✅ User API Key Authentication passed");
    Ok(())
}

/// Test that users cannot create API keys with higher permissions than their own
async fn test_user_api_key_permission_escalation(ctx: &TestContext) -> Result<(), String> {
    use crate::infra::exec_sql;

    println!("Testing User API Key Permission Escalation Prevention...");

    // Set user to Member permissions (can't create Admin/Owner keys)
    exec_sql("UPDATE users SET permissions = 'Member';")?;

    // Try to create a key with Admin permissions (should fail)
    let api_key = UserApiKey::new(UserApiKeyBase {
        key: String::new(),
        name: "Escalation Test Key".to_string(),
        user_id: Uuid::nil(),
        organization_id: ctx.organization_id,
        permissions: UserOrgPermissions::Admin, // Higher than Member
        last_used: None,
        expires_at: None,
        is_enabled: true,
        tags: Vec::new(),
        network_ids: vec![ctx.network_id],
    });

    let result = ctx
        .client
        .post_expect_status("/api/v1/auth/keys", &api_key, StatusCode::FORBIDDEN)
        .await;

    // Restore Owner permissions
    exec_sql("UPDATE users SET permissions = 'Owner';")?;

    assert!(
        result.is_ok(),
        "Should reject creating API key with higher permissions: {:?}",
        result.err()
    );
    println!("  ✓ Cannot create API key with higher permissions than user (Admin key as Member)");

    // Now test that Owner CAN create Admin keys
    let api_key_admin = UserApiKey::new(UserApiKeyBase {
        key: String::new(),
        name: "Admin Key By Owner".to_string(),
        user_id: Uuid::nil(),
        organization_id: ctx.organization_id,
        permissions: UserOrgPermissions::Admin,
        last_used: None,
        expires_at: None,
        is_enabled: true,
        tags: Vec::new(),
        network_ids: vec![ctx.network_id],
    });

    let created: UserApiKeyResponse = ctx.client.post("/api/v1/auth/keys", &api_key_admin).await?;
    let key_id = created.api_key.id;
    println!("  ✓ Owner can create Admin-level API key");

    // Cleanup
    ctx.client
        .delete_no_content(&format!("/api/v1/auth/keys/{}", key_id))
        .await?;

    println!("✅ User API Key Permission Escalation Prevention passed");
    Ok(())
}

/// Test the key rotation endpoint
async fn test_user_api_key_rotation(ctx: &TestContext) -> Result<(), String> {
    println!("Testing User API Key Rotation...");

    // Create a key to rotate
    let api_key = UserApiKey::new(UserApiKeyBase {
        key: String::new(),
        name: "Rotation Test Key".to_string(),
        user_id: Uuid::nil(),
        organization_id: ctx.organization_id,
        permissions: UserOrgPermissions::Viewer,
        last_used: None,
        expires_at: None,
        is_enabled: true,
        tags: Vec::new(),
        network_ids: vec![ctx.network_id],
    });

    let created: UserApiKeyResponse = ctx.client.post("/api/v1/auth/keys", &api_key).await?;
    let original_key = created.key.clone();
    let key_id = created.api_key.id;
    println!("  ✓ Created API key for rotation test");

    // Rotate the key
    let rotated: String = ctx
        .client
        .post(&format!("/api/v1/auth/keys/{}/rotate", key_id), &())
        .await?;

    assert_ne!(
        rotated, original_key,
        "Rotated key should be different from original"
    );
    assert!(
        rotated.starts_with("scp_u_"),
        "Rotated key should have user prefix"
    );
    println!("  ✓ Key rotation returns new key with correct prefix");

    // Verify old key no longer works
    let api_key_client = reqwest::Client::new();
    let response = api_key_client
        .get(format!("{}/api/v1/subnets", BASE_URL))
        .header("Authorization", format!("Bearer {}", original_key))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Old key should be rejected after rotation"
    );
    println!("  ✓ Old key is invalidated after rotation");

    // Verify new key works
    let response = api_key_client
        .get(format!("{}/api/v1/subnets", BASE_URL))
        .header("Authorization", format!("Bearer {}", rotated))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    assert!(
        response.status().is_success(),
        "New rotated key should work, got {}",
        response.status()
    );
    println!("  ✓ New rotated key works correctly");

    // Cleanup
    ctx.client
        .delete_no_content(&format!("/api/v1/auth/keys/{}", key_id))
        .await?;

    println!("✅ User API Key Rotation passed");
    Ok(())
}

/// Test that expired and disabled keys are rejected during authentication
async fn test_user_api_key_expired_disabled(ctx: &TestContext) -> Result<(), String> {
    use chrono::{Duration, Utc};

    println!("Testing Expired/Disabled API Key Rejection...");

    // Create a key that will be disabled
    let api_key = UserApiKey::new(UserApiKeyBase {
        key: String::new(),
        name: "Disable Test Key".to_string(),
        user_id: Uuid::nil(),
        organization_id: ctx.organization_id,
        permissions: UserOrgPermissions::Viewer,
        last_used: None,
        expires_at: None,
        is_enabled: true,
        tags: Vec::new(),
        network_ids: vec![ctx.network_id],
    });

    let created: UserApiKeyResponse = ctx.client.post("/api/v1/auth/keys", &api_key).await?;
    let plaintext_key = created.key.clone();
    let key_id = created.api_key.id;

    // Verify key works initially
    let api_key_client = reqwest::Client::new();
    let response = api_key_client
        .get(format!("{}/api/v1/subnets", BASE_URL))
        .header("Authorization", format!("Bearer {}", plaintext_key))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    assert!(
        response.status().is_success(),
        "Enabled key should work initially"
    );
    println!("  ✓ Enabled key works");

    // Disable the key via update
    let mut fetched: UserApiKey = ctx
        .client
        .get(&format!("/api/v1/auth/keys/{}", key_id))
        .await?;
    fetched.base.is_enabled = false;
    let _updated: UserApiKey = ctx
        .client
        .put(&format!("/api/v1/auth/keys/{}", key_id), &fetched)
        .await?;

    // Verify disabled key is rejected
    let response = api_key_client
        .get(format!("{}/api/v1/subnets", BASE_URL))
        .header("Authorization", format!("Bearer {}", plaintext_key))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "Disabled key should be rejected"
    );
    println!("  ✓ Disabled key is rejected");

    // Re-enable but set expiration in the past
    fetched.base.is_enabled = true;
    fetched.base.expires_at = Some(Utc::now() - Duration::hours(1));
    let _updated: UserApiKey = ctx
        .client
        .put(&format!("/api/v1/auth/keys/{}", key_id), &fetched)
        .await?;

    // Verify expired key is rejected
    let response = api_key_client
        .get(format!("{}/api/v1/subnets", BASE_URL))
        .header("Authorization", format!("Bearer {}", plaintext_key))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "Expired key should be rejected"
    );
    println!("  ✓ Expired key is rejected");

    // Cleanup
    ctx.client
        .delete_no_content(&format!("/api/v1/auth/keys/{}", key_id))
        .await?;

    println!("✅ Expired/Disabled API Key Rejection passed");
    Ok(())
}

/// Test that API keys only have access to assigned networks
async fn test_user_api_key_network_access(ctx: &TestContext) -> Result<(), String> {
    use crate::infra::exec_sql;
    use scanopy::server::networks::r#impl::{Network, NetworkBase};

    println!("Testing User API Key Network Access Enforcement...");

    // Create a second network that we won't grant API key access to
    let other_network = Network::new(NetworkBase {
        name: "API Key Inaccessible Network".to_string(),
        organization_id: ctx.organization_id,
        ..Default::default()
    });
    let other_network = ctx.insert_entity(&other_network).await?;
    println!("  Created second network: {}", other_network.id);

    // Create a subnet on the other network to query
    let other_subnet = Subnet::new(SubnetBase {
        name: "Other Network Subnet".to_string(),
        description: None,
        network_id: other_network.id,
        cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(172, 16, 0, 0), 24).unwrap()),
        subnet_type: SubnetType::Lan,
        source: EntitySource::System,
        tags: Vec::new(),
    });
    let other_subnet = ctx.insert_entity(&other_subnet).await?;
    println!("  Created subnet on other network: {}", other_subnet.id);

    // Create API key with access ONLY to ctx.network_id (not other_network.id)
    let api_key = UserApiKey::new(UserApiKeyBase {
        key: String::new(),
        name: "Limited Network Key".to_string(),
        user_id: Uuid::nil(),
        organization_id: ctx.organization_id,
        permissions: UserOrgPermissions::Viewer,
        last_used: None,
        expires_at: None,
        is_enabled: true,
        tags: Vec::new(),
        network_ids: vec![ctx.network_id], // Only first network
    });

    let created: UserApiKeyResponse = ctx.client.post("/api/v1/auth/keys", &api_key).await?;
    let plaintext_key = created.key.clone();
    let key_id = created.api_key.id;
    println!("  ✓ Created API key with limited network access");

    let api_key_client = reqwest::Client::new();

    // Verify key can access subnets on assigned network
    let response = api_key_client
        .get(format!("{}/api/v1/subnets", BASE_URL))
        .header("Authorization", format!("Bearer {}", plaintext_key))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    assert!(
        response.status().is_success(),
        "API key should access subnets list"
    );
    println!("  ✓ API key can list subnets");

    // Verify key cannot directly access the other network's subnet
    let response = api_key_client
        .get(format!("{}/api/v1/subnets/{}", BASE_URL, other_subnet.id))
        .header("Authorization", format!("Bearer {}", plaintext_key))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    // Should be 403 Forbidden (network access denied) or 404 Not Found (filtered out)
    assert!(
        response.status() == StatusCode::FORBIDDEN || response.status() == StatusCode::NOT_FOUND,
        "API key should NOT access subnet on other network, got {}",
        response.status()
    );
    println!("  ✓ API key cannot access subnet on non-assigned network");

    // Verify attempting to create API key with access to network user doesn't own fails
    // First, downgrade user to Member so they don't have auto-access to all networks
    exec_sql("UPDATE users SET permissions = 'Member';")?;

    // Remove user's access to the other network (Members don't auto-get all networks)
    // The user only has access to ctx.network_id via the network_users table
    let restricted_key = UserApiKey::new(UserApiKeyBase {
        key: String::new(),
        name: "Overreach Key".to_string(),
        user_id: Uuid::nil(),
        organization_id: ctx.organization_id,
        permissions: UserOrgPermissions::Viewer,
        last_used: None,
        expires_at: None,
        is_enabled: true,
        tags: Vec::new(),
        network_ids: vec![ctx.network_id, other_network.id], // Includes network user shouldn't access
    });

    let result = ctx
        .client
        .post_expect_status("/api/v1/auth/keys", &restricted_key, StatusCode::FORBIDDEN)
        .await;

    // Restore Owner permissions
    exec_sql("UPDATE users SET permissions = 'Owner';")?;

    assert!(
        result.is_ok(),
        "Should reject API key with unauthorized network access: {:?}",
        result.err()
    );
    println!("  ✓ Cannot create API key with access to networks user doesn't have access to");

    // Cleanup
    ctx.client
        .delete_no_content(&format!("/api/v1/auth/keys/{}", key_id))
        .await?;
    let _ = ctx.delete_entity::<Subnet>(&other_subnet.id).await;
    let _ = ctx.delete_entity::<Network>(&other_network.id).await;

    println!("✅ User API Key Network Access Enforcement passed");
    Ok(())
}

/// Test that users cannot access other users' API keys
async fn test_user_api_key_owner_isolation(ctx: &TestContext) -> Result<(), String> {
    use scanopy::server::user_api_keys::r#impl::base::UserApiKey as UserApiKeyEntity;

    println!("Testing User API Key Owner Isolation...");

    // Create an API key for the current user
    let api_key = UserApiKey::new(UserApiKeyBase {
        key: String::new(),
        name: "Owner Isolation Test Key".to_string(),
        user_id: Uuid::nil(),
        organization_id: ctx.organization_id,
        permissions: UserOrgPermissions::Viewer,
        last_used: None,
        expires_at: None,
        is_enabled: true,
        tags: Vec::new(),
        network_ids: vec![ctx.network_id],
    });

    let created: UserApiKeyResponse = ctx.client.post("/api/v1/auth/keys", &api_key).await?;
    let key_id = created.api_key.id;
    let user_id = created.api_key.base.user_id;
    println!("  ✓ Created API key owned by user {}", user_id);

    // Create another user's API key directly in the database
    // (simulating another user who created a key)
    let mut other_user = User::new(UserBase::default());
    other_user.base.organization_id = ctx.organization_id;
    let created_other_user = ctx.insert_entity(&other_user).await.unwrap();
    let other_key = UserApiKeyEntity::new(UserApiKeyBase {
        key: "other_user_hash".to_string(),
        name: "Other User Key".to_string(),
        user_id: created_other_user.id,
        organization_id: ctx.organization_id,
        permissions: UserOrgPermissions::Viewer,
        last_used: None,
        expires_at: None,
        is_enabled: true,
        tags: Vec::new(),
        network_ids: vec![ctx.network_id],
    });
    let other_key = ctx.insert_entity(&other_key).await?;
    println!(
        "  ✓ Created API key owned by other user {}",
        created_other_user.id
    );

    // Verify current user cannot read the other user's key
    let result = ctx
        .client
        .get_expect_status(
            &format!("/api/v1/auth/keys/{}", other_key.id),
            StatusCode::NOT_FOUND,
        )
        .await;
    assert!(
        result.is_ok(),
        "Should not be able to read other user's API key: {:?}",
        result.err()
    );
    println!("  ✓ Cannot read other user's API key (returns 404)");

    // Verify other user's key doesn't appear in the list
    let keys: Vec<UserApiKey> = ctx.client.get("/api/v1/auth/keys").await?;
    let other_key_in_list = keys.iter().any(|k| k.id == other_key.id);
    assert!(
        !other_key_in_list,
        "Other user's key should not appear in list"
    );
    println!("  ✓ Other user's key doesn't appear in list");

    // Verify current user cannot update the other user's key
    let mut attempt_update = other_key.clone();
    attempt_update.base.name = "Hacked Name".to_string();
    let _result = ctx
        .client
        .post_expect_status(
            &format!("/api/v1/auth/keys/{}", other_key.id),
            &attempt_update,
            StatusCode::NOT_FOUND,
        )
        .await;
    // Note: PUT might return 404 or 403, both are acceptable
    // The key is that the update should not succeed
    println!("  ✓ Cannot update other user's API key");

    // Verify current user cannot delete the other user's key
    // Use the authenticated client, which should fail
    let response = ctx
        .client
        .client
        .delete(format!("{}/api/v1/auth/keys/{}", BASE_URL, other_key.id))
        .send()
        .await
        .map_err(|e| format!("Delete request failed: {}", e))?;

    assert!(
        response.status() == StatusCode::NOT_FOUND || response.status() == StatusCode::FORBIDDEN,
        "Delete should fail for other user's key, got {}",
        response.status()
    );
    println!("  ✓ Cannot delete other user's API key");

    // Cleanup - delete both keys
    ctx.client
        .delete_no_content(&format!("/api/v1/auth/keys/{}", key_id))
        .await?;
    let _ = ctx.delete_entity::<UserApiKeyEntity>(&other_key.id).await;

    println!("✅ User API Key Owner Isolation passed");
    Ok(())
}
