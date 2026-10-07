//! Permission and access control tests.
//!
//! Tests that users cannot access resources on sites/organizations they don't have access to.

use crate::infra::{TestContext, exec_sql};
use cidr::{IpCidr, Ipv4Cidr};
use reqwest::StatusCode;
use scanopy::server::hosts::r#impl::api::CreateHostRequest;
use scanopy::server::hosts::r#impl::base::{Host, HostBase};
use scanopy::server::hosts::r#impl::name::HostNameSources;
use scanopy::server::shared::attribution::AttributeSource;
use scanopy::server::shared::storage::traits::Storable;
use scanopy::server::shared::types::entities::EntitySource;
use scanopy::server::sites::r#impl::{Site, SiteBase};
use scanopy::server::subnets::r#impl::base::{Subnet, SubnetBase};
use scanopy::server::subnets::r#impl::base::{SubnetCidr, SubnetCidrValue};
use scanopy::server::subnets::r#impl::types::SubnetType;
use std::net::Ipv4Addr;
use uuid::Uuid;

pub async fn run_permission_tests(ctx: &TestContext) -> Result<(), String> {
    println!("\n=== Testing Permissions & Access Control ===\n");

    // Create a second site in the same org that we won't grant access to
    let other_site_id = create_inaccessible_site(ctx).await?;

    // Downgrade user to Member so they don't auto-get access to all sites
    // (Owners/Admins automatically have access to all sites in their org)
    set_user_permissions("Member")?;

    test_cannot_read_host_on_other_site(ctx, other_site_id).await?;
    test_cannot_create_host_on_other_site(ctx, other_site_id).await?;
    test_cannot_create_subnet_on_other_site(ctx, other_site_id).await?;

    // Restore Owner permissions
    set_user_permissions("Owner")?;

    // Clean up
    cleanup_inaccessible_site(ctx, other_site_id).await?;

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

/// Creates a site in the same organization that we won't grant the user access to.
/// Combined with downgrading user to Member, this tests intra-org site permissions.
async fn create_inaccessible_site(ctx: &TestContext) -> Result<Uuid, String> {
    let site = Site::new(SiteBase {
        name: "Inaccessible Site".to_string(),
        organization_id: ctx.organization_id,
        ..Default::default()
    });
    let created = ctx.insert_entity(&site).await?;

    println!("  Created inaccessible site: {}", created.id);
    Ok(created.id)
}

async fn cleanup_inaccessible_site(ctx: &TestContext, site_id: Uuid) -> Result<(), String> {
    // Clean up in reverse order of dependencies using exec_sql for bulk deletes
    let _ = exec_sql(&format!(
        "DELETE FROM subnets WHERE site_id = '{}';",
        site_id
    ));
    let _ = exec_sql(&format!("DELETE FROM hosts WHERE site_id = '{}';", site_id));
    // Delete the site
    let _ = ctx.delete_entity::<Site>(&site_id).await;
    Ok(())
}

/// Test that user cannot read hosts on a site they don't have access to
async fn test_cannot_read_host_on_other_site(
    ctx: &TestContext,
    other_site_id: Uuid,
) -> Result<(), String> {
    println!("Testing: Cannot read hosts on inaccessible site...");

    // Create a host directly in the database on the other site
    let host = Host::new(HostBase {
        name: scanopy::server::hosts::r#impl::name::HostName::manual("Secret Host".to_string()),
        site_id: other_site_id,
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
        "Should not be able to read host on inaccessible site: {:?}",
        result.err()
    );
    println!("  ✓ Cannot read host on inaccessible site (returns 404)");

    Ok(())
}

/// Test that user cannot create hosts on a site they don't have access to
async fn test_cannot_create_host_on_other_site(
    ctx: &TestContext,
    other_site_id: Uuid,
) -> Result<(), String> {
    println!("Testing: Cannot create host on inaccessible site...");

    let host_request = CreateHostRequest {
        name: "Unauthorized Host".to_string(),
        hostname: None,
        site_id: other_site_id, // Site user doesn't have access to
        description: None,
        virtualization_metadata: None,
        virtualization_service_id: None,
        virtualization_interface_id: None,
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
        ip_addresses: vec![],
        ports: vec![],
        services: vec![],
        interfaces: vec![],
    };

    // Should get 401 Unauthorized (or 403 Forbidden)
    let result = ctx
        .client
        .post_expect_status("/api/v1/hosts", &host_request, StatusCode::FORBIDDEN)
        .await;

    assert!(
        result.is_ok(),
        "Should not be able to create host on inaccessible site: {:?}",
        result.err()
    );
    println!("  ✓ Cannot create host on inaccessible site (returns 401)");

    Ok(())
}

/// Test that user cannot create subnets on a site they don't have access to
async fn test_cannot_create_subnet_on_other_site(
    ctx: &TestContext,
    other_site_id: Uuid,
) -> Result<(), String> {
    println!("Testing: Cannot create subnet on inaccessible site...");

    let subnet = Subnet::new(SubnetBase {
        name: "Unauthorized Subnet".to_string(),
        description: None,
        site_id: other_site_id, // Site user doesn't have access to
        cidr: SubnetCidr::new(
            SubnetCidrValue(IpCidr::V4(
                Ipv4Cidr::new(Ipv4Addr::new(10, 0, 0, 0), 24).unwrap(),
            )),
            AttributeSource::DaemonSelfReport,
        ),
        subnet_type: SubnetType::Lan,
        source: EntitySource::System,
        tags: Vec::new(),
        virtualization_service_id: None,
    });

    // Should get 401 Unauthorized
    let result = ctx
        .client
        .post_expect_status("/api/v1/subnets", &subnet, StatusCode::FORBIDDEN)
        .await;

    assert!(
        result.is_ok(),
        "Should not be able to create subnet on inaccessible site: {:?}",
        result.err()
    );
    println!("  ✓ Cannot create subnet on inaccessible site (returns 401)");

    Ok(())
}
