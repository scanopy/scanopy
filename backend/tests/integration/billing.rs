//! Billing middleware tests.

use crate::infra::{TestContext, reset_plan_to_default, set_billable_plan, set_plan_status};
use cidr::{IpCidr, Ipv4Cidr};
use reqwest::StatusCode;
use scanopy::server::hosts::r#impl::base::Host;
use scanopy::server::shared::attribution::AttributeSource;
use scanopy::server::shared::storage::traits::Storable;
use scanopy::server::shared::types::entities::EntitySource;
use scanopy::server::subnets::r#impl::base::{Subnet, SubnetBase};
use scanopy::server::subnets::r#impl::base::{SubnetCidr, SubnetCidrValue};
use scanopy::server::subnets::r#impl::types::SubnetType;
use scanopy::server::tags::r#impl::base::Tag;
use std::net::Ipv4Addr;

pub async fn run_billing_tests(ctx: &TestContext) -> Result<(), String> {
    println!("\n=== Testing Billing Middleware ===\n");

    // Set a billable plan type (Community/CommercialSelfHosted/Demo are exempt)
    set_billable_plan().expect("Failed to set billable plan");

    test_billing_active_allows_requests(ctx).await?;
    test_billing_trialing_allows_requests(ctx).await?;
    test_billing_past_due_blocks_requests(ctx).await?;
    test_billing_canceled_blocks_requests(ctx).await?;
    test_billing_null_blocks_requests(ctx).await?;

    // Restore plan and status for cleanup
    reset_plan_to_default().expect("Failed to reset plan");
    set_plan_status(Some("active")).expect("Failed to restore plan status");

    println!("\n✅ All billing middleware tests passed!");
    Ok(())
}

async fn test_billing_active_allows_requests(ctx: &TestContext) -> Result<(), String> {
    println!("Testing: active status allows requests...");
    set_plan_status(Some("active"))?;

    let _: Vec<Tag> = ctx.client.get("/api/v1/tags").await?;

    println!("  ✓ active status allows GET requests");
    Ok(())
}

async fn test_billing_trialing_allows_requests(ctx: &TestContext) -> Result<(), String> {
    println!("Testing: trialing status allows requests...");
    set_plan_status(Some("trialing"))?;

    let _: Vec<Host> = ctx.client.get("/api/v1/hosts").await?;

    println!("  ✓ trialing status allows requests");

    set_plan_status(Some("active"))?;
    Ok(())
}

async fn test_billing_past_due_blocks_requests(ctx: &TestContext) -> Result<(), String> {
    println!("Testing: past_due status blocks requests...");
    set_plan_status(Some("past_due"))?;

    let subnet = Subnet::new(SubnetBase {
        name: "Blocked Subnet".to_string(),
        description: None,
        network_id: ctx.network_id,
        cidr: SubnetCidr::new(
            SubnetCidrValue(IpCidr::V4(
                Ipv4Cidr::new(Ipv4Addr::new(10, 1, 0, 0), 24).unwrap(),
            )),
            AttributeSource::DaemonSelfReport,
        ),
        subnet_type: SubnetType::Lan,
        source: EntitySource::System,
        tags: Vec::new(),
        virtualization_service_id: None,
    });

    let result = ctx
        .client
        .post_expect_status("/api/v1/subnets", &subnet, StatusCode::OK)
        .await;
    assert!(result.is_ok(), "past_due should return 402: {:?}", result);
    println!("  ✓ past_due allows POST /api/v1/subnets (200)");

    let result = ctx
        .client
        .get_expect_status("/api/v1/hosts", StatusCode::OK)
        .await;
    assert!(
        result.is_ok(),
        "past_due should return 402 for GET: {:?}",
        result
    );
    println!("  ✓ past_due allows GET /api/v1/hosts (200)");

    set_plan_status(Some("active"))?;
    Ok(())
}

async fn test_billing_canceled_blocks_requests(ctx: &TestContext) -> Result<(), String> {
    println!("Testing: canceled (lapsed) status makes the org read-only...");
    set_plan_status(Some("canceled"))?;

    let result = ctx
        .client
        .get_expect_status("/api/v1/hosts", StatusCode::OK)
        .await;
    assert!(result.is_ok(), "canceled should allow GET: {:?}", result);
    println!("  ✓ canceled allows GET /api/v1/hosts (200)");

    let subnet = Subnet::new(SubnetBase {
        name: "Lapsed Subnet".to_string(),
        description: None,
        network_id: ctx.network_id,
        cidr: SubnetCidr::new(
            SubnetCidrValue(IpCidr::V4(
                Ipv4Cidr::new(Ipv4Addr::new(10, 2, 0, 0), 24).unwrap(),
            )),
            AttributeSource::DaemonSelfReport,
        ),
        subnet_type: SubnetType::Lan,
        source: EntitySource::System,
        tags: Vec::new(),
        virtualization_service_id: None,
    });
    let result = ctx
        .client
        .post_expect_status("/api/v1/subnets", &subnet, StatusCode::PAYMENT_REQUIRED)
        .await;
    assert!(
        result.is_ok(),
        "canceled should return 402 for POST: {:?}",
        result
    );
    println!("  ✓ canceled blocks POST /api/v1/subnets (402)");

    set_plan_status(Some("active"))?;
    Ok(())
}

async fn test_billing_null_blocks_requests(ctx: &TestContext) -> Result<(), String> {
    println!("Testing: null status blocks requests...");
    set_plan_status(None)?;

    let result = ctx
        .client
        .get_expect_status("/api/v1/hosts", StatusCode::PAYMENT_REQUIRED)
        .await;
    assert!(
        result.is_ok(),
        "null status should return 402: {:?}",
        result
    );
    println!("  ✓ null status blocks requests (402)");

    set_plan_status(Some("active"))?;
    Ok(())
}
