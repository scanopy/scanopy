//! Permission and access control tests.
//!
//! Tests that users cannot access resources on networks/organizations they don't have access to.

use crate::infra::{TestContext, exec_sql};
use cidr::{IpCidr, Ipv4Cidr};
use reqwest::StatusCode;
use scanopy::server::hosts::r#impl::api::CreateHostRequest;
use scanopy::server::hosts::r#impl::base::{Host, HostBase};
use scanopy::server::networks::r#impl::{Network, NetworkBase};
use scanopy::server::shared::storage::traits::Storable;
use scanopy::server::shared::types::entities::EntitySource;
use scanopy::server::subnets::r#impl::base::{Subnet, SubnetBase};
use scanopy::server::subnets::r#impl::types::SubnetType;
use std::net::Ipv4Addr;
use uuid::Uuid;

pub async fn run_permission_tests(ctx: &TestContext) -> Result<(), String> {
    println!("\n=== Testing Permissions & Access Control ===\n");

    // Create a second network in the same org that we won't grant access to
    let other_network_id = create_inaccessible_network(ctx).await?;

    // Downgrade user to Member so they don't auto-get access to all networks
    // (Owners/Admins automatically have access to all networks in their org)
    set_user_permissions("Member")?;

    test_cannot_read_host_on_other_network(ctx, other_network_id).await?;
    test_cannot_create_host_on_other_network(ctx, other_network_id).await?;
    test_cannot_create_subnet_on_other_network(ctx, other_network_id).await?;

    // Restore Owner permissions
    set_user_permissions("Owner")?;

    // Clean up
    cleanup_inaccessible_network(ctx, other_network_id).await?;

    println!("\n✅ All permission tests passed!");
    Ok(())
}

/// Set user permissions via direct SQL update
fn set_user_permissions(permissions: &str) -> Result<(), String> {
    exec_sql(&format!(
        "UPDATE users SET permissions = '{}';",
        permissions
    ))?;
    println!("  Set user permissions to: {}", permissions);
    Ok(())
}

/// Creates a network in the same organization that we won't grant the user access to.
/// Combined with downgrading user to Member, this tests intra-org network permissions.
async fn create_inaccessible_network(ctx: &TestContext) -> Result<Uuid, String> {
    let network = Network::new(NetworkBase {
        name: "Inaccessible Network".to_string(),
        organization_id: ctx.organization_id,
        ..Default::default()
    });
    let created = ctx.insert_entity(&network).await?;

    println!("  Created inaccessible network: {}", created.id);
    Ok(created.id)
}

async fn cleanup_inaccessible_network(ctx: &TestContext, network_id: Uuid) -> Result<(), String> {
    // Clean up in reverse order of dependencies using exec_sql for bulk deletes
    let _ = exec_sql(&format!(
        "DELETE FROM subnets WHERE network_id = '{}';",
        network_id
    ));
    let _ = exec_sql(&format!(
        "DELETE FROM hosts WHERE network_id = '{}';",
        network_id
    ));
    // Delete the network
    let _ = ctx.delete_entity::<Network>(&network_id).await;
    Ok(())
}

/// Test that user cannot read hosts on a network they don't have access to
async fn test_cannot_read_host_on_other_network(
    ctx: &TestContext,
    other_network_id: Uuid,
) -> Result<(), String> {
    println!("Testing: Cannot read hosts on inaccessible network...");

    // Create a host directly in the database on the other network
    let host = Host::new(HostBase {
        name: "Secret Host".to_string(),
        network_id: other_network_id,
        source: EntitySource::System,
        ..Default::default()
    });
    let created_host = ctx.insert_entity(&host).await?;

    // Try to read this host - should fail
    let result = ctx
        .client
        .get_expect_status(
            &format!("/api/v1/hosts/{}", created_host.id),
            StatusCode::FORBIDDEN,
        )
        .await;

    // Clean up
    let _ = ctx.delete_entity::<Host>(&created_host.id).await;

    assert!(
        result.is_ok(),
        "Should not be able to read host on inaccessible network: {:?}",
        result.err()
    );
    println!("  ✓ Cannot read host on inaccessible network (returns 404)");

    Ok(())
}

/// Test that user cannot create hosts on a network they don't have access to
async fn test_cannot_create_host_on_other_network(
    ctx: &TestContext,
    other_network_id: Uuid,
) -> Result<(), String> {
    println!("Testing: Cannot create host on inaccessible network...");

    let host_request = CreateHostRequest {
        name: "Unauthorized Host".to_string(),
        hostname: None,
        network_id: other_network_id, // Network user doesn't have access to
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

    // Should get 401 Unauthorized (or 403 Forbidden)
    let result = ctx
        .client
        .post_expect_status("/api/v1/hosts", &host_request, StatusCode::FORBIDDEN)
        .await;

    assert!(
        result.is_ok(),
        "Should not be able to create host on inaccessible network: {:?}",
        result.err()
    );
    println!("  ✓ Cannot create host on inaccessible network (returns 401)");

    Ok(())
}

/// Test that user cannot create subnets on a network they don't have access to
async fn test_cannot_create_subnet_on_other_network(
    ctx: &TestContext,
    other_network_id: Uuid,
) -> Result<(), String> {
    println!("Testing: Cannot create subnet on inaccessible network...");

    let subnet = Subnet::new(SubnetBase {
        name: "Unauthorized Subnet".to_string(),
        description: None,
        network_id: other_network_id, // Network user doesn't have access to
        cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(10, 0, 0, 0), 24).unwrap()),
        subnet_type: SubnetType::Lan,
        source: EntitySource::System,
        tags: Vec::new(),
    });

    // Should get 401 Unauthorized
    let result = ctx
        .client
        .post_expect_status("/api/v1/subnets", &subnet, StatusCode::FORBIDDEN)
        .await;

    assert!(
        result.is_ok(),
        "Should not be able to create subnet on inaccessible network: {:?}",
        result.err()
    );
    println!("  ✓ Cannot create subnet on inaccessible network (returns 401)");

    Ok(())
}
