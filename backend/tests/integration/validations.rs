//! Handler validation tests.

use crate::infra::{BASE_URL, TestContext};
use reqwest::StatusCode;
use scanopy::server::daemons::r#impl::base::Daemon;
use scanopy::server::hosts::r#impl::api::{CreateHostRequest, HostResponse};
use scanopy::server::networks::r#impl::{Network, NetworkBase};
use scanopy::server::services::definitions::ServiceDefinitionRegistry;
use scanopy::server::services::r#impl::base::{Service, ServiceBase};
use scanopy::server::shared::storage::traits::Storable;
use scanopy::server::shared::types::entities::EntitySource;
use scanopy::server::tags::r#impl::base::{Tag, TagBase};

pub async fn run_validation_tests(ctx: &TestContext) -> Result<(), String> {
    println!("\n=== Testing Handler Validations ===\n");

    test_service_network_validation(ctx).await?;
    test_host_daemon_deletion_prevention(ctx).await?;
    test_bulk_delete_validation(ctx).await?;

    println!("\n✅ All handler validation tests passed!");
    Ok(())
}

async fn test_service_network_validation(ctx: &TestContext) -> Result<(), String> {
    println!("Testing: Service must be on same network as host...");

    // Create a second network that the user has access to
    let second_network = Network::new(NetworkBase {
        name: "Validation Test Network".to_string(),
        organization_id: ctx.organization_id,
        ..Default::default()
    });
    let second_network: Network = ctx.client.post("/api/v1/networks", &second_network).await?;

    // Create host on the first network
    let host_request = CreateHostRequest {
        name: "Validation Test Host".to_string(),
        hostname: Some("validation.local".to_string()),
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

    // Try to create a service on the second network that references the host on the first network
    let service_def = ServiceDefinitionRegistry::all_service_definitions()[0].clone();
    let service = Service::new(ServiceBase {
        name: "Wrong Network Service".to_string(),
        host_id: created_host.id,
        bindings: vec![],
        network_id: second_network.id, // Different network than host!
        service_definition: service_def,
        virtualization: None,
        source: EntitySource::System,
        tags: Vec::new(),
        position: 0,
    });

    let result = ctx
        .client
        .post_expect_status("/api/v1/services", &service, StatusCode::BAD_REQUEST)
        .await;
    assert!(
        result.is_ok(),
        "Service on different network should return 400: {:?}",
        result
    );
    println!("  ✓ Service with different network_id than host returns 400");

    // Cleanup
    ctx.client
        .delete_no_content(&format!("/api/v1/hosts/{}", created_host.id))
        .await?;
    ctx.client
        .delete_no_content(&format!("/api/v1/networks/{}", second_network.id))
        .await?;

    Ok(())
}

async fn test_host_daemon_deletion_prevention(ctx: &TestContext) -> Result<(), String> {
    println!("Testing: Cannot delete host with associated daemon...");

    let daemons: Vec<Daemon> = ctx.client.get("/api/v1/daemons").await?;

    if let Some(daemon) = daemons.first() {
        let result = ctx
            .client
            .get_expect_status(
                &format!("/api/v1/hosts/{}", daemon.base.host_id),
                StatusCode::OK,
            )
            .await;

        if result.is_ok() {
            let response = ctx
                .client
                .client
                .delete(format!("{}/api/v1/hosts/{}", BASE_URL, daemon.base.host_id))
                .send()
                .await
                .map_err(|e| format!("Delete request failed: {}", e))?;

            assert_eq!(
                response.status(),
                StatusCode::CONFLICT,
                "Deleting host with daemon should return 409 Conflict"
            );
            println!("  ✓ Deleting host with daemon returns 409 Conflict");
        }
    } else {
        println!("  ⚠ No daemon found to test host-daemon deletion");
    }

    Ok(())
}

async fn test_bulk_delete_validation(ctx: &TestContext) -> Result<(), String> {
    println!("Testing: Bulk delete validates all items...");

    let mut tag1 = Tag::new(TagBase::default());
    tag1.base.organization_id = ctx.organization_id;
    tag1.base.name = "Bulk Test Tag 1".to_string();
    let created1: Tag = ctx.client.post("/api/v1/tags", &tag1).await?;

    let mut tag2 = Tag::new(TagBase::default());
    tag2.base.organization_id = ctx.organization_id;
    tag2.base.name = "Bulk Test Tag 2".to_string();
    let created2: Tag = ctx.client.post("/api/v1/tags", &tag2).await?;

    let ids = vec![created1.id, created2.id];
    let result: serde_json::Value = ctx.client.post("/api/v1/tags/bulk-delete", &ids).await?;

    assert!(
        result.get("deleted").is_some() || result.get("deleted_count").is_some(),
        "Bulk delete should return deleted count"
    );
    println!("  ✓ Bulk delete returns success with count");

    let result1 = ctx
        .client
        .get_expect_status(
            &format!("/api/v1/tags/{}", created1.id),
            StatusCode::NOT_FOUND,
        )
        .await;
    let result2 = ctx
        .client
        .get_expect_status(
            &format!("/api/v1/tags/{}", created2.id),
            StatusCode::NOT_FOUND,
        )
        .await;

    assert!(result1.is_ok(), "First tag should be deleted");
    assert!(result2.is_ok(), "Second tag should be deleted");
    println!("  ✓ All bulk deleted items are removed");

    Ok(())
}
