use scanopy::server::bindings::r#impl::base::Binding;
use scanopy::server::daemon_api_keys::r#impl::base::DaemonApiKey;
use scanopy::server::daemons::r#impl::base::Daemon;
use scanopy::server::discovery::r#impl::base::Discovery;
use scanopy::server::groups::r#impl::base::Group;
use scanopy::server::hosts::r#impl::base::Host;
use scanopy::server::if_entries::r#impl::base::IfEntry;
use scanopy::server::interfaces::r#impl::base::Interface;
use scanopy::server::invites::r#impl::base::Invite;
use scanopy::server::networks::r#impl::Network;
use scanopy::server::organizations::r#impl::base::Organization;
use scanopy::server::ports::r#impl::base::Port;
use scanopy::server::services::definitions::ServiceDefinitionRegistry;
use scanopy::server::services::r#impl::base::Service;
use scanopy::server::services::r#impl::definitions::{ServiceDefinition, ServiceDefinitionExt};
use scanopy::server::shared::entity_metadata::EntityCategory;
use scanopy::server::shared::fixtures::generate_ui_data_fixtures;
use scanopy::server::shared::storage::traits::{Entity, Storable};
use scanopy::server::shared::types::metadata::EntityMetadataProvider;
use scanopy::server::shares::r#impl::base::Share;
use scanopy::server::subnets::r#impl::base::Subnet;
use scanopy::server::tags::r#impl::base::Tag;
use scanopy::server::topology::types::base::Topology;
use scanopy::server::user_api_keys::r#impl::base::UserApiKey;
use scanopy::server::users::r#impl::base::User;
use serde::Serialize;

/// Generate all fixtures (requires Docker containers to be running, except OpenAPI)
pub async fn generate_fixtures() {
    generate_db_fixture()
        .await
        .expect("Failed to generate db fixture");

    generate_daemon_config_fixture()
        .await
        .expect("Failed to generate daemon config fixture");

    generate_services_json()
        .await
        .expect("Failed to generate services json");

    // Generate all UI data fixtures (billing plans, features, credential types, etc.)
    let ui_data_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to get parent directory")
        .join("ui/src/lib/data");
    generate_ui_data_fixtures(&ui_data_dir);

    generate_schema_mermaid()
        .await
        .expect("Failed to generate schema mermaid");

    generate_entity_metadata_json()
        .await
        .expect("Failed to generate entity metadata json");

    merge_daemon_fixtures()
        .await
        .expect("Failed to merge daemon fixtures");

    // OpenAPI generation - public spec only (excludes internal endpoints)
    let openapi_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to get parent directory")
        .join("ui/static/openapi-public.json");
    super::openapi_gen::generate_public(&openapi_path).expect("Failed to generate OpenAPI spec");

    println!("✅ Generated test fixtures");
}

/// Merge mode-specific daemon fixture files into the canonical server_to_daemon.json.
///
/// Each daemon container (daemon_poll, server_poll) writes to its own file to avoid
/// race conditions on the shared volume. This function merges them, deduplicating
/// by method+path, preferring server_poll exchanges (the interesting ServerPoll protocol).
async fn merge_daemon_fixtures() -> Result<(), Box<dyn std::error::Error>> {
    use serde::{Deserialize, Serialize};
    use std::collections::HashSet;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct CapturedExchange {
        method: String,
        path: String,
        request_body: serde_json::Value,
        response_status: u16,
        response_body: serde_json::Value,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct FixtureManifest {
        version: String,
        exchanges: Vec<CapturedExchange>,
    }

    let fixtures_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/integration/compat/fixtures");

    if !fixtures_dir.exists() {
        return Ok(());
    }

    // Fix permissions on fixture files written by Docker containers (which run as root)
    let _ = std::process::Command::new("docker")
        .args([
            "exec",
            "scanopy-server-1",
            "chmod",
            "-R",
            "a+rw",
            "/app/tests/integration/compat/fixtures",
        ])
        .output();

    // Find all version directories
    for entry in std::fs::read_dir(&fixtures_dir)? {
        let entry = entry?;
        if !entry.path().is_dir() {
            continue;
        }

        let version_dir = entry.path();
        let server_poll_path = version_dir.join("server_to_server_poll.json");
        let daemon_poll_path = version_dir.join("server_to_daemon_poll.json");

        // Skip versions that don't have mode-specific files
        if !server_poll_path.exists() && !daemon_poll_path.exists() {
            continue;
        }

        let version_name = entry
            .file_name()
            .into_string()
            .unwrap_or_default()
            .trim_start_matches('v')
            .to_string();

        // Start with server_poll exchanges (preferred — these are the full ServerPoll protocol)
        let mut merged: Vec<CapturedExchange> = Vec::new();
        let mut seen: HashSet<(String, String)> = HashSet::new();

        for path in [&server_poll_path, &daemon_poll_path] {
            if !path.exists() {
                continue;
            }

            let content = std::fs::read_to_string(path)?;
            if let Ok(manifest) = serde_json::from_str::<FixtureManifest>(&content) {
                for exchange in manifest.exchanges {
                    let key = (exchange.method.clone(), exchange.path.clone());
                    if seen.insert(key) {
                        merged.push(exchange);
                    }
                }
            }
        }

        if merged.is_empty() {
            continue;
        }

        let manifest = FixtureManifest {
            version: version_name.clone(),
            exchanges: merged,
        };

        let merged_path = version_dir.join("server_to_daemon.json");
        let json = serde_json::to_string_pretty(&manifest)?;
        std::fs::write(&merged_path, json)?;

        // Clean up mode-specific files
        let _ = std::fs::remove_file(&server_poll_path);
        let _ = std::fs::remove_file(&daemon_poll_path);

        println!(
            "✅ Merged daemon fixtures for v{} ({} exchanges)",
            version_name,
            manifest.exchanges.len()
        );
    }

    Ok(())
}

async fn generate_db_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::process::Command::new("docker")
        .args([
            "exec",
            "scanopy-postgres-dev-1",
            "pg_dump",
            "-U",
            "postgres",
            "-d",
            "scanopy",
            "--clean",
            "--if-exists",
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "pg_dump failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let fixture_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tests/scanopy.sql");
    std::fs::write(&fixture_path, output.stdout)?;

    println!("✅ Generated scanopy.sql from test data");
    Ok(())
}

async fn generate_daemon_config_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let find_output = std::process::Command::new("docker")
        .args([
            "exec",
            "scanopy-daemon-1",
            "find",
            "/root/.config",
            "-name",
            "config.json",
            "-type",
            "f",
        ])
        .output()?;

    if !find_output.status.success() {
        return Err(format!(
            "Failed to find daemon config: {}",
            String::from_utf8_lossy(&find_output.stderr)
        )
        .into());
    }

    let config_path = String::from_utf8_lossy(&find_output.stdout)
        .trim()
        .to_string();

    if config_path.is_empty() {
        return Err("No config.json found in container".into());
    }

    println!("Found daemon config at: {}", config_path);

    let output = std::process::Command::new("docker")
        .args(["exec", "scanopy-daemon-1", "cat", &config_path])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to read daemon config: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let fixture_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tests/daemon_config.json");
    std::fs::write(&fixture_path, output.stdout)?;

    println!("✅ Generated daemon_config.json from test daemon");
    Ok(())
}

async fn generate_services_json() -> Result<(), Box<dyn std::error::Error>> {
    let services: Vec<serde_json::Value> = ServiceDefinitionRegistry::all_service_definitions()
        .iter()
        .filter_map(|s| {
            if s.can_be_manually_added() {
                Some(serde_json::json!({
                    "logo_url": s.logo_url(),
                    "name": s.name(),
                    "description": s.description(),
                    "discovery_pattern": s.discovery_pattern().to_string(),
                    "category": s.category(),
                    "color": s.color(),
                    "logo_needs_white_background": s.logo_needs_white_background()
                }))
            } else {
                None
            }
        })
        .collect();

    let json_string = serde_json::to_string_pretty(&services)?;
    let json_path = std::path::Path::new("../ui/static/services.json");
    tokio::fs::write(json_path, json_string).await?;

    Ok(())
}

/// Entity metadata entry for documentation generation
#[derive(Serialize)]
struct EntityMetadataEntry {
    /// Unique identifier (e.g., "host")
    id: &'static str,
    /// Singular name (e.g., "Host")
    name_singular: &'static str,
    /// Plural name (e.g., "Hosts")
    name_plural: &'static str,
    /// Description for documentation
    description: &'static str,
    /// Category key (e.g., "network_infrastructure")
    category: &'static str,
    /// Human-readable category name (e.g., "Network Infrastructure")
    category_display: &'static str,
    /// Database table name (e.g., "hosts")
    table_name: &'static str,
}

impl EntityMetadataEntry {
    fn new<E: Entity + Storable>(id: &'static str) -> Self {
        let category = E::entity_category();
        Self {
            id,
            name_singular: E::ENTITY_NAME_SINGULAR,
            name_plural: E::ENTITY_NAME_PLURAL,
            description: E::ENTITY_DESCRIPTION,
            category: category_to_snake_case(category),
            category_display: category.display_name(),
            table_name: E::table_name(),
        }
    }
}

fn category_to_snake_case(category: EntityCategory) -> &'static str {
    match category {
        EntityCategory::OrganizationsAndUsers => "organizations_and_users",
        EntityCategory::NetworkInfrastructure => "network_infrastructure",
        EntityCategory::DiscoveryAndDaemons => "discovery_and_daemons",
        EntityCategory::Visualization => "visualization",
        EntityCategory::Metadata => "metadata",
    }
}

async fn generate_entity_metadata_json() -> Result<(), Box<dyn std::error::Error>> {
    let metadata: Vec<EntityMetadataEntry> = vec![
        // Organizations & Users
        EntityMetadataEntry::new::<Organization>("organization"),
        EntityMetadataEntry::new::<User>("user"),
        EntityMetadataEntry::new::<Invite>("invite"),
        EntityMetadataEntry::new::<UserApiKey>("user_api_key"),
        // Network Infrastructure
        EntityMetadataEntry::new::<Network>("network"),
        EntityMetadataEntry::new::<Host>("host"),
        EntityMetadataEntry::new::<Subnet>("subnet"),
        EntityMetadataEntry::new::<Interface>("interface"),
        EntityMetadataEntry::new::<Port>("port"),
        EntityMetadataEntry::new::<Service>("service"),
        EntityMetadataEntry::new::<Binding>("binding"),
        EntityMetadataEntry::new::<IfEntry>("if_entry"),
        // Discovery & Daemons
        EntityMetadataEntry::new::<Daemon>("daemon"),
        EntityMetadataEntry::new::<DaemonApiKey>("daemon_api_key"),
        EntityMetadataEntry::new::<Discovery>("discovery"),
        // Visualization
        EntityMetadataEntry::new::<Group>("group"),
        EntityMetadataEntry::new::<Topology>("topology"),
        EntityMetadataEntry::new::<Share>("share"),
        // Metadata
        EntityMetadataEntry::new::<Tag>("tag"),
    ];

    let json_string = serde_json::to_string_pretty(&metadata)?;
    let json_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to get parent directory")
        .join("ui/static/entity-metadata.json");
    tokio::fs::write(&json_path, json_string).await?;

    println!("✅ Generated entity-metadata.json");
    Ok(())
}

async fn generate_schema_mermaid() -> Result<(), Box<dyn std::error::Error>> {
    // Check if tbls is available (graceful skip for local dev without tbls)
    let which = std::process::Command::new("which").arg("tbls").output();
    if which.is_err() || !which.unwrap().status.success() {
        println!("⚠️  tbls not found, skipping schema generation");
        return Ok(());
    }

    let temp_dir = std::env::temp_dir().join("tbls-schema");
    let _ = std::fs::remove_dir_all(&temp_dir);

    // tbls runs on host, connects to exposed port 5435
    let output = std::process::Command::new("tbls")
        .args([
            "doc",
            "postgres://postgres:password@localhost:5435/scanopy?sslmode=disable",
            temp_dir.to_str().unwrap(),
            "--er-format",
            "mermaid",
            "--exclude",
            "sqlx_migrations",
            "--force",
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!("tbls failed: {}", String::from_utf8_lossy(&output.stderr)).into());
    }

    // Extract mermaid block from README.md
    let readme_path = temp_dir.join("README.md");
    let readme_content = std::fs::read_to_string(&readme_path)?;

    let mermaid = readme_content
        .lines()
        .skip_while(|line| *line != "```mermaid")
        .skip(1) // skip the ```mermaid line
        .take_while(|line| *line != "```")
        .collect::<Vec<_>>()
        .join("\n");

    let _ = std::fs::remove_dir_all(&temp_dir);

    // Full schema with all columns
    let schema_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to get parent directory")
        .join("ui/static/schema.mermaid");
    std::fs::write(&schema_path, &mermaid)?;
    println!("✅ Generated schema.mermaid");

    // Simplified ER diagram (relationships only, no columns)
    let simplified_er = generate_simplified_er(&mermaid);
    let er_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to get parent directory")
        .join("ui/static/schema-er.mermaid");
    std::fs::write(&er_path, simplified_er)?;
    println!("✅ Generated schema-er.mermaid");

    Ok(())
}

/// Generate a simplified ER diagram from the full tbls mermaid output.
/// This strips the attribute blocks (columns) and keeps only table names and relationships.
fn generate_simplified_er(full_mermaid: &str) -> String {
    let mut result = Vec::new();
    let mut in_table_block = false;

    for line in full_mermaid.lines() {
        let trimmed = line.trim();

        // Keep the erDiagram declaration
        if trimmed == "erDiagram" {
            result.push(line.to_string());
            continue;
        }

        // Detect start of table block (table name followed by {)
        if trimmed.ends_with('{') {
            in_table_block = true;
            // Extract table name and add it without attributes
            let table_name = trimmed.trim_end_matches('{').trim();
            result.push(format!("  {}", table_name));
            continue;
        }

        // End of table block
        if trimmed == "}" {
            in_table_block = false;
            continue;
        }

        // Skip attribute lines inside table blocks
        if in_table_block {
            continue;
        }

        // Keep relationship lines (contain || or }o or |{ etc)
        if trimmed.contains("||")
            || trimmed.contains("}o")
            || trimmed.contains("|{")
            || trimmed.contains("o{")
        {
            result.push(line.to_string());
        }
    }

    result.join("\n")
}
