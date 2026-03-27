//! Integration test suite for Scanopy.
//!
//! This module runs all integration tests in a single test function to share
//! Docker containers across test categories, significantly reducing test time.
//!
//! Test categories:
//! - Full integration flow (auth, discovery, entity creation)
//! - CRUD endpoint tests
//! - Billing middleware tests
//! - Handler validation tests

mod billing;
mod billing_fixtures;
mod compat;
mod crud;
mod discovery;
#[cfg(feature = "generate-fixtures")]
mod fixtures;
mod infra;
mod openapi_gen;
mod permissions;
mod validations;

use infra::{
    ContainerManager, TestClient, TestContext, clear_discovery_data, create_test_db_pool,
    provision_serverpoll_daemon, setup_authenticated_user, wait_for_daemon, wait_for_network,
    wait_for_organization,
};

/// Single integration test that runs all test categories with shared containers.
///
/// This avoids spinning up/down containers for each test category, which saves
/// significant time (each container cycle takes ~30-60 seconds).
#[tokio::test]
async fn integration_tests() {
    let mut container_manager = ContainerManager::new();

    // Start containers once
    container_manager
        .start()
        .expect("Failed to start containers");

    let client = TestClient::new();

    // =========================================================================
    // Phase 1: Minimal Setup for Compat Tests
    // =========================================================================
    println!("\n============================================================");
    println!("Phase 1: Minimal Setup");
    println!("============================================================\n");

    let user = setup_authenticated_user(&client)
        .await
        .expect("Failed to authenticate user");
    println!("✅ Authenticated as: {}", user.base.email);

    println!("\n=== Waiting for Organization ===");
    let organization = wait_for_organization(&client)
        .await
        .expect("Failed to find organization");
    println!("✅ Organization: {}", organization.base.name);

    println!("\n=== Waiting for Network ===");
    let network = wait_for_network(&client)
        .await
        .expect("Failed to find network");
    println!("✅ Network: {}", network.base.name);

    // =========================================================================
    // Phase 2: DaemonPoll Discovery (creates subnets needed for compat tests)
    // =========================================================================
    println!("\n============================================================");
    println!("Phase 2: DaemonPoll Discovery");
    println!("============================================================\n");

    println!("\n=== Waiting for DaemonPoll Daemon ===");
    let daemon = wait_for_daemon(&client)
        .await
        .expect("Failed to find daemon");
    println!("✅ DaemonPoll daemon registered: {}", daemon.id);

    // Run discovery - this creates subnets that compat tests need
    // Pass None for session_id since DaemonPoll auto-starts discovery
    discovery::run_discovery(&client, None)
        .await
        .expect("DaemonPoll discovery failed");

    println!("\n✅ DaemonPoll discovery completed!");

    // =========================================================================
    // Phase 3: ServerPoll Provisioning
    // =========================================================================
    println!("\n============================================================");
    println!("Phase 3: ServerPoll Provisioning");
    println!("============================================================\n");

    // Clear discovery data but keep network structure
    clear_discovery_data().expect("Failed to clear discovery data");

    // Provision ServerPoll daemon (needed for compat tests)
    let serverpoll_provision = provision_serverpoll_daemon(&client, network.id)
        .await
        .expect("Failed to provision ServerPoll daemon");
    let serverpoll_daemon_id = serverpoll_provision.daemon.id;
    let serverpoll_api_key = serverpoll_provision.daemon_api_key.clone();
    println!("✅ ServerPoll daemon provisioned: {}", serverpoll_daemon_id);

    // =========================================================================
    // Phase 4: ServerPoll Discovery
    // =========================================================================
    // Run discovery BEFORE compat tests to ensure the daemon has clean state.
    // Compat tests send fixture requests that can leave stale sessions in the
    // server's daemon_sessions map, causing discovery to skip publishing events.
    println!("\n============================================================");
    println!("Phase 4: ServerPoll Discovery");
    println!("============================================================\n");

    // Clear any leftover discovery data from previous test runs
    clear_discovery_data().expect("Failed to clear discovery data");

    // Trigger discovery for the ServerPoll daemon and get the session_id
    let serverpoll_host_id = serverpoll_provision.daemon.base.host_id;
    let session_id = discovery::trigger_discovery(
        &client,
        serverpoll_daemon_id,
        serverpoll_host_id,
        network.id,
    )
    .await
    .expect("Failed to trigger discovery for ServerPoll daemon");

    // Wait for this specific session to complete via SSE stream
    // (filters by session_id to avoid catching stalled sessions from other daemons)
    discovery::run_discovery(&client, Some(session_id))
        .await
        .expect("ServerPoll discovery failed");

    // Verify service discovered
    let service = discovery::verify_home_assistant_discovered(&client)
        .await
        .expect("Failed to find Home Assistant after ServerPoll discovery");

    // Test creating user entities that reference discovered data
    let tag = discovery::create_tag(&client, organization.id)
        .await
        .expect("Failed to create tag");

    // Apply the tag to the discovered service
    discovery::apply_tag_to_service(&client, tag.id, service.id)
        .await
        .expect("Failed to apply tag to discovered service");

    let _group = discovery::create_group(&client, network.id)
        .await
        .expect("Failed to create group");

    println!("\n✅ ServerPoll discovery completed!");

    // =========================================================================
    // Phase 5: API Compatibility Tests
    // =========================================================================
    // Runs after discovery so any stale sessions left by fixture playback
    // don't affect the real discovery flow.
    println!("\n============================================================");
    println!("Phase 5: API Compatibility Tests");
    println!("============================================================");

    compat::run_compat_tests(
        daemon.id, // Use DaemonPoll daemon ID (like original)
        network.id,
        organization.id,
        user.id,
        &serverpoll_api_key,
    )
    .await
    .expect("Compatibility tests failed");

    // =========================================================================
    // Phase 6: CRUD Endpoint Tests
    // =========================================================================
    println!("\n============================================================");
    println!("Phase 6: CRUD Endpoint Tests");
    println!("============================================================");

    let db_pool = create_test_db_pool()
        .await
        .expect("Failed to create test database pool");

    let ctx = TestContext {
        client: TestClient::new(),
        network_id: network.id,
        organization_id: organization.id,
        db_pool,
    };

    // Re-authenticate for CRUD tests
    let _ = setup_authenticated_user(&ctx.client)
        .await
        .expect("Failed to re-authenticate");

    crud::run_crud_tests(&ctx).await.expect("CRUD tests failed");

    // =========================================================================
    // Phase 7: Billing Middleware Tests
    // =========================================================================
    println!("\n============================================================");
    println!("Phase 7: Billing Middleware Tests");
    println!("============================================================");

    billing::run_billing_tests(&ctx)
        .await
        .expect("Billing tests failed");

    // =========================================================================
    // Phase 8: Handler Validation Tests
    // =========================================================================
    println!("\n============================================================");
    println!("Phase 8: Handler Validation Tests");
    println!("============================================================");

    validations::run_validation_tests(&ctx)
        .await
        .expect("Validation tests failed");

    // =========================================================================
    // Phase 9: Permission & Access Control Tests
    // =========================================================================
    println!("\n============================================================");
    println!("Phase 9: Permission & Access Control Tests");
    println!("============================================================");

    permissions::run_permission_tests(&ctx)
        .await
        .expect("Permission tests failed");

    // =========================================================================
    // Phase 10: Generate Fixtures (optional)
    // =========================================================================
    #[cfg(feature = "generate-fixtures")]
    {
        println!("\n============================================================");
        println!("Phase 10: Generating Fixtures");
        println!("============================================================");

        fixtures::generate_fixtures().await;
    }

    // =========================================================================
    // Summary
    // =========================================================================
    println!("\n============================================================");
    println!("ALL INTEGRATION TESTS PASSED!");
    println!("============================================================");
    println!("   - DaemonPoll integration flow (discovery)");
    println!("   - ServerPoll integration flow (discovery)");
    println!("   - CRUD endpoint tests");
    println!("   - Billing middleware tests");
    println!("   - Handler validation tests");
    println!("   - Permission & access control tests");
    #[cfg(feature = "generate-fixtures")]
    println!("   - Fixture generation");
    println!("   - API compatibility tests");
}
