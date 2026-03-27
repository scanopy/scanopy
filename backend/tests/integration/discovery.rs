//! Discovery and integration flow tests.

use crate::infra::{BASE_URL, TestClient, retry};
use scanopy::server::daemons::r#impl::api::DiscoveryUpdatePayload;
use scanopy::server::discovery::r#impl::base::{Discovery, DiscoveryBase};
use scanopy::server::discovery::r#impl::types::{DiscoveryType, HostNamingFallback, RunType};
use scanopy::server::groups::r#impl::base::{Group, GroupBase};
use scanopy::server::services::definitions::home_assistant::HomeAssistant;
use scanopy::server::services::r#impl::base::Service;
use scanopy::server::shared::entities::EntityDiscriminants;
use scanopy::server::shared::storage::traits::Storable;
use scanopy::server::shared::types::metadata::HasId;
use scanopy::server::tags::handlers::BulkTagRequest;
use scanopy::server::tags::r#impl::base::{Tag, TagBase};
use uuid::Uuid;

/// Trigger discovery for a specific daemon by creating a Discovery record and starting the session.
/// Returns the session_id so callers can track the specific session.
pub async fn trigger_discovery(
    client: &TestClient,
    daemon_id: Uuid,
    host_id: Uuid,
    network_id: Uuid,
) -> Result<Uuid, String> {
    println!("\n=== Creating Discovery for ServerPoll Daemon ===");

    // Create a Unified Discovery record
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
            name: "ServerPoll Integration Test Discovery".to_string(),
            daemon_id,
            network_id,
            tags: vec![],
        },
        scan_count: 0,
        force_full_scan: false,
        pending_credential_ids: vec![],
    };

    let created_discovery: Discovery = client.post("/api/v1/discovery", &discovery).await?;
    println!("✅ Created Discovery record: {}", created_discovery.id);

    // Start the discovery session
    let update: DiscoveryUpdatePayload = client
        .post("/api/v1/discovery/start-session", &created_discovery.id)
        .await?;
    println!("✅ Started discovery session: {}", update.session_id);

    Ok(update.session_id)
}

/// Wait for a discovery session to complete via SSE stream.
/// If session_id is provided, filters to only that specific session.
/// If None, accepts any Network discovery update (useful for DaemonPoll auto-discovery).
pub async fn run_discovery(client: &TestClient, session_id: Option<Uuid>) -> Result<(), String> {
    if let Some(sid) = session_id {
        println!("🔌 Connecting to SSE stream for session {}...", sid);
    } else {
        println!("🔌 Connecting to SSE stream (waiting for any Network discovery)...");
    }

    let mut event_source = client
        .client
        .get(format!("{}/api/v1/discovery/stream", BASE_URL))
        .send()
        .await
        .map_err(|e| format!("Failed to connect to SSE: {}", e))?;

    // 15 minutes to allow for deep port scanning (65535 ports × multiple hosts)
    // With adaptive batch sizing, constrained hosts use smaller batches (64 ports)
    // which means 1024+ batches per host. This timeout accommodates slow CI environments.
    let timeout = tokio::time::sleep(tokio::time::Duration::from_secs(900));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            _ = &mut timeout => {
                return Err("Discovery timed out after 15 minutes".to_string());
            }
            chunk = event_source.chunk() => {
                match chunk {
                    Ok(Some(bytes)) => {
                        let text = String::from_utf8_lossy(&bytes);

                        for line in text.lines() {
                            if let Some(data) = line.strip_prefix("data: ")
                                && let Ok(update) = serde_json::from_str::<DiscoveryUpdatePayload>(data)
                            {
                                // If filtering by session_id, skip other sessions
                                if let Some(expected_sid) = session_id {
                                    if update.session_id != expected_sid {
                                        continue;
                                    }
                                } else {
                                    // No session_id filter - only accept Network or Unified discoveries
                                    if !matches!(update.discovery_type, DiscoveryType::Network { .. } | DiscoveryType::Unified { .. }) {
                                        continue;
                                    }
                                }

                                println!(
                                    "📊 Discovery [{}]: {} - {}%",
                                    update.session_id,
                                    update.phase,
                                    update.progress,
                                );

                                if update.finished_at.is_some() {
                                    if let Some(error) = &update.error {
                                        return Err(format!("Discovery failed: {}", error));
                                    }
                                    println!("✅ Discovery completed!");
                                    return Ok(());
                                }
                            }
                        }
                    }
                    Ok(None) => return Err("SSE stream ended unexpectedly".to_string()),
                    Err(e) => return Err(format!("Error reading SSE: {}", e)),
                }
            }
        }
    }
}

pub async fn verify_home_assistant_discovered(client: &TestClient) -> Result<Service, String> {
    println!("\n=== Verifying Home Assistant Discovery ===");

    retry("find Home Assistant service", 10, 2, || async {
        let services: Vec<Service> = client.get("/api/v1/services").await?;

        if services.is_empty() {
            return Err("No services found yet".to_string());
        }

        println!("✅ Found {} service(s):", services.len());
        for service in &services {
            println!(
                "   - {} ({})",
                service.base.name,
                service.base.service_definition.id()
            );
        }

        services
            .into_iter()
            .find(|s| s.base.service_definition.id() == HomeAssistant.id())
            .ok_or_else(|| "Home Assistant service not found".to_string())
    })
    .await
}

pub async fn create_group(client: &TestClient, network_id: Uuid) -> Result<Group, String> {
    println!("\n=== Creating Group ===");

    let mut group = Group::new(GroupBase::default());
    group.base.network_id = network_id;

    retry("create Group", 10, 3, || async {
        let created_group: Group = client.post("/api/v1/groups", &group).await?;
        println!("✅ Created group");
        Ok(created_group)
    })
    .await
}

pub async fn create_tag(client: &TestClient, organization_id: Uuid) -> Result<Tag, String> {
    println!("\n=== Creating Tag ===");

    let mut tag = Tag::new(TagBase::default());
    tag.base.organization_id = organization_id;
    tag.base.name = "Integration Test Tag".to_string();

    retry("create Tag", 10, 3, || async {
        let created_tag: Tag = client.post("/api/v1/tags", &tag).await?;
        println!("✅ Created Tag: {}", created_tag.base.name);
        Ok(created_tag)
    })
    .await
}

/// Apply a tag to a service using the bulk-add endpoint.
pub async fn apply_tag_to_service(
    client: &TestClient,
    tag_id: Uuid,
    service_id: Uuid,
) -> Result<(), String> {
    println!("\n=== Applying Tag to Discovered Service ===");

    let request = BulkTagRequest {
        entity_type: EntityDiscriminants::Service,
        entity_ids: vec![service_id],
        tag_id,
    };

    retry("apply tag to service", 5, 2, || async {
        let _response: serde_json::Value = client
            .post("/api/v1/tags/assign/bulk-add", &request)
            .await?;
        println!("✅ Applied tag to service");
        Ok(())
    })
    .await
}
