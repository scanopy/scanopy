//! Shared fixture generation logic used by both the `generate-fixtures` binary
//! and integration tests.

use crate::daemon::discovery::types::base::{DiscoveryPhase, DiscoveryTerminalReason};
use crate::daemon::discovery::types::warnings::{
    ClaimSource, DiscoveryWarningCode, MalformedNeighbourConsequence, SnmpWalkGroup, WarningRemedy,
};
use crate::server::billing::plans::get_website_fixture_plans;
use crate::server::billing::types::base::{BillingPlan, CancelReason, PlanStatus, SaveOffer};
use crate::server::billing::types::features::Feature;
use crate::server::credentials::r#impl::integrations::all_integrations;
use crate::server::credentials::r#impl::mapping::CredentialQueryPayloadDiscriminants;
use crate::server::credentials::r#impl::types::CredentialTypeDiscriminants;
use crate::server::dependencies::r#impl::types::DependencyType;
use crate::server::discovery::r#impl::scan_settings::ScanSettings;
use crate::server::discovery::r#impl::types::DiscoveryType;
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::ServiceDefinitionRegistry;
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::shared::attribution::{AttributeMethod, AttributeSourceDiscriminants};
use crate::server::shared::concepts::Concept;
use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::types::entities::EntitySourceDiscriminants;
use crate::server::shared::types::metadata::{EntityMetadata, MetadataProvider, TypeMetadata};
use crate::server::subnets::r#impl::types::SubnetType;
use crate::server::topology::types::edges::EdgeType;
use crate::server::topology::types::grouping::{ContainerRule, ElementRule};
use crate::server::topology::types::nodes::ContainerType;
use crate::server::topology::types::views::TopologyView;
use crate::server::users::r#impl::permissions::UserOrgPermissions;
use std::fs;
use std::path::Path;
use strum::{IntoDiscriminant, IntoEnumIterator};

/// Generate all UI data fixture JSON files into the given output directory.
///
/// This is called by:
/// - `generate-fixtures` binary (local dev, Docker builds)
/// - Integration tests (CI release workflow)
pub fn generate_ui_data_fixtures(output_dir: &Path) {
    println!(
        "Generating metadata fixtures to {}...",
        output_dir.display()
    );

    fs::create_dir_all(output_dir).expect("Failed to create output directory");

    // Billing fixtures — website plans (curated, for billing modal)
    let plan_metadata: Vec<TypeMetadata> = get_website_fixture_plans()
        .iter()
        .map(|p| p.to_metadata())
        .collect();
    write_fixture(&plan_metadata, output_dir, "billing-plans.json");

    // All billing plan variants (for metadata store, matches old /api/metadata)
    let all_plan_metadata: Vec<TypeMetadata> =
        BillingPlan::iter().map(|p| p.to_metadata()).collect();
    write_fixture(&all_plan_metadata, output_dir, "billing-plans-all.json");

    let feature_metadata: Vec<TypeMetadata> = Feature::iter().map(|f| f.to_metadata()).collect();
    write_fixture(&feature_metadata, output_dir, "features.json");

    let all_services = ServiceDefinitionRegistry::all_service_definitions();
    let service_defs: Vec<TypeMetadata> = all_services.iter().map(|t| t.to_metadata()).collect();
    write_fixture(&service_defs, output_dir, "service-definitions.json");

    // Download service logos to static directory for local serving
    // output_dir is ui/src/lib/data, static dir is ui/static/logos/services
    let static_dir = output_dir.join("../../../static/logos/services");
    download_service_logos(&all_services, &static_dir);

    let subnet_types: Vec<TypeMetadata> = SubnetType::iter().map(|t| t.to_metadata()).collect();
    write_fixture(&subnet_types, output_dir, "subnet-types.json");

    let edge_types: Vec<TypeMetadata> = EdgeType::iter().map(|t| t.to_metadata()).collect();
    write_fixture(&edge_types, output_dir, "edge-types.json");

    let dependency_types: Vec<TypeMetadata> = DependencyType::iter()
        .map(|t| t.discriminant().to_metadata())
        .collect();
    write_fixture(&dependency_types, output_dir, "dependency-types.json");

    let ports: Vec<TypeMetadata> = PortType::iter().map(|p| p.to_metadata()).collect();
    write_fixture(&ports, output_dir, "ports.json");

    let discovery_types: Vec<TypeMetadata> =
        DiscoveryType::iter().map(|d| d.to_metadata()).collect();
    write_fixture(&discovery_types, output_dir, "discovery-types.json");

    let discovery_phases: Vec<TypeMetadata> =
        DiscoveryPhase::iter().map(|p| p.to_metadata()).collect();
    write_fixture(&discovery_phases, output_dir, "discovery-phases.json");

    let discovery_terminal_reasons: Vec<TypeMetadata> = DiscoveryTerminalReason::iter()
        .map(|r| r.to_metadata())
        .collect();
    write_fixture(
        &discovery_terminal_reasons,
        output_dir,
        "discovery-terminal-reasons.json",
    );

    // Scan warnings: the code carries the sentence template, and the three enums after it carry
    // the values that fill its slots. All four have to reach the UI, or the English the codes
    // replaced simply moves one level down and crosses the wire as a slot value instead.
    let warning_codes: Vec<TypeMetadata> = DiscoveryWarningCode::iter()
        .map(|c| c.to_metadata())
        .collect();
    write_fixture(&warning_codes, output_dir, "warning-codes.json");

    // The rungs the codes above are filed under, in the order the warning list renders them. Each
    // code names one through its `category`; this is where that name resolves to a heading.
    let warning_remedies: Vec<TypeMetadata> =
        WarningRemedy::iter().map(|r| r.to_metadata()).collect();
    write_fixture(&warning_remedies, output_dir, "warning-remedies.json");

    let snmp_walk_groups: Vec<TypeMetadata> =
        SnmpWalkGroup::iter().map(|g| g.to_metadata()).collect();
    write_fixture(&snmp_walk_groups, output_dir, "snmp-walk-groups.json");

    let claim_sources: Vec<TypeMetadata> = ClaimSource::iter().map(|c| c.to_metadata()).collect();
    write_fixture(&claim_sources, output_dir, "claim-sources.json");

    // Keyed by `CredentialQueryPayloadDiscriminants`, which is what a coded warning carries.
    // Neither `integrations.json` (keyed by display name) nor `credential-types.json` (keyed by
    // `CredentialType`) can resolve those eight values.
    let discovery_integrations: Vec<TypeMetadata> = CredentialQueryPayloadDiscriminants::iter()
        .map(|i| i.to_metadata())
        .collect();
    write_fixture(
        &discovery_integrations,
        output_dir,
        "discovery-integrations.json",
    );

    let malformed_neighbour_consequences: Vec<TypeMetadata> = MalformedNeighbourConsequence::iter()
        .map(|c| c.to_metadata())
        .collect();
    write_fixture(
        &malformed_neighbour_consequences,
        output_dir,
        "malformed-neighbour-consequences.json",
    );

    let permissions: Vec<TypeMetadata> = UserOrgPermissions::iter()
        .map(|p| p.to_metadata())
        .collect();
    write_fixture(&permissions, output_dir, "permissions.json");

    // Entity metadata (uses TypeMetadata for the metadata field with parent_entity)
    let entities: Vec<TypeMetadata> = EntityDiscriminants::iter()
        .map(|e: EntityDiscriminants| {
            <EntityDiscriminants as MetadataProvider<TypeMetadata>>::to_metadata(&e)
        })
        .collect();
    write_fixture(&entities, output_dir, "entities.json");

    // How an entity came to exist. Keyed by the `source.type` tag every entity carries, so the
    // UI labels, colours and filters on it without restating the variants.
    let entity_sources: Vec<TypeMetadata> = EntitySourceDiscriminants::iter()
        .map(|s| s.to_metadata())
        .collect();
    write_fixture(&entity_sources, output_dir, "entity-sources.json");

    let concepts: Vec<EntityMetadata> = Concept::iter().map(|e| e.to_metadata()).collect();
    write_fixture(&concepts, output_dir, "concepts.json");

    let credential_types: Vec<TypeMetadata> = CredentialTypeDiscriminants::iter()
        .map(|d| d.to_metadata())
        .collect();
    write_fixture(&credential_types, output_dir, "credential-types.json");

    // Integrations: a service joined with the credential transports that target
    // it and one canonical discovery description (for the website integrations
    // page). Synced like service-definitions.json.
    let integrations = all_integrations();
    write_fixture(&integrations, output_dir, "integrations.json");

    let scan_settings_fields = ScanSettings::field_definitions();
    write_fixture(&scan_settings_fields, output_dir, "scan-settings.json");

    let container_rule_types: Vec<TypeMetadata> =
        ContainerRule::iter().map(|r| r.to_metadata()).collect();
    write_fixture(
        &container_rule_types,
        output_dir,
        "container-rule-types.json",
    );

    let element_rule_types: Vec<TypeMetadata> =
        ElementRule::iter().map(|r| r.to_metadata()).collect();
    write_fixture(&element_rule_types, output_dir, "element-rule-types.json");

    let container_types: Vec<TypeMetadata> =
        ContainerType::iter().map(|r| r.to_metadata()).collect();
    write_fixture(&container_types, output_dir, "container-types.json");

    let views: Vec<TypeMetadata> = TopologyView::iter().map(|v| v.to_metadata()).collect();
    write_fixture(&views, output_dir, "views.json");

    let service_categories: Vec<TypeMetadata> =
        ServiceCategory::iter().map(|c| c.to_metadata()).collect();
    write_fixture(&service_categories, output_dir, "service-categories.json");

    let cancel_reasons: Vec<TypeMetadata> = CancelReason::iter().map(|r| r.to_metadata()).collect();
    write_fixture(&cancel_reasons, output_dir, "cancel-reasons.json");

    let save_offers: Vec<TypeMetadata> = SaveOffer::iter().map(|o| o.to_metadata()).collect();
    write_fixture(&save_offers, output_dir, "save-offers.json");

    let plan_statuses: Vec<TypeMetadata> = PlanStatus::iter().map(|s| s.to_metadata()).collect();
    write_fixture(&plan_statuses, output_dir, "plan-statuses.json");

    // How much a discovered value is worth, and which sources sit at each tier. The tier is what
    // an operator reads ("assumed", "read from the device"); the source list under it is the
    // lookup the UI needs to get from a value's recorded source to that label, inverted from
    // `AttributeSource::method()` so it cannot disagree with the ordering the applier uses.
    let attribute_methods: Vec<TypeMetadata> =
        AttributeMethod::iter().map(|m| m.to_metadata()).collect();
    write_fixture(&attribute_methods, output_dir, "attribute-methods.json");

    // Where one value came from, by name. Keyed by the source's discriminant: `Probe` and
    // `Authored` are templates whose `{probe}` slot is filled from `client-probes.json`.
    let attribute_sources: Vec<TypeMetadata> = AttributeSourceDiscriminants::iter()
        .map(|s| s.to_metadata())
        .collect();
    write_fixture(&attribute_sources, output_dir, "attribute-sources.json");

    let client_probes: Vec<TypeMetadata> = ClientProbe::iter().map(|p| p.to_metadata()).collect();
    write_fixture(&client_probes, output_dir, "client-probes.json");

    println!("Done! Generated all metadata fixtures.");
}

fn write_fixture<T: serde::Serialize>(items: &[T], output_dir: &Path, filename: &str) {
    let json = serde_json::to_string_pretty(items).expect("Failed to serialize");
    let path = output_dir.join(filename);
    fs::write(&path, json).unwrap_or_else(|_| panic!("Failed to write {filename}"));
    println!("  {}", filename);
}

/// Convert a service name to a URL-safe filename slug.
/// "Home Assistant" → "home-assistant", "Docker Container" → "docker-container"
pub fn logo_slug(name: &str) -> String {
    name.to_lowercase().replace(' ', "-")
}

/// Derive file extension from a logo URL.
/// "https://cdn.jsdelivr.net/.../docker.svg" → "svg"
fn logo_ext(url: &str) -> &str {
    url.rsplit('.')
        .next()
        .and_then(|e| e.split('?').next())
        .filter(|e| matches!(*e, "svg" | "png" | "webp"))
        .unwrap_or("svg")
}

/// Download service logos from CDN URLs to local static directory.
/// Files are saved with extensions so the static server sets correct Content-Type.
fn download_service_logos(services: &[Box<dyn ServiceDefinition>], static_dir: &Path) {
    println!("Downloading service logos to {}...", static_dir.display());
    fs::create_dir_all(static_dir).expect("Failed to create logos directory");

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client");

    let mut downloaded = 0;
    let mut failed = 0;

    for service in services {
        let url = service.logo_url();
        if url.is_empty() || url.starts_with('/') {
            continue;
        }

        let ext = logo_ext(url);
        let filename = format!("{}.{}", logo_slug(service.name()), ext);
        let path = static_dir.join(&filename);

        match client.get(url).send().and_then(|r| r.error_for_status()) {
            Ok(resp) => match resp.bytes() {
                Ok(bytes) => {
                    fs::write(&path, &bytes)
                        .unwrap_or_else(|_| panic!("Failed to write logo for {}", service.name()));
                    downloaded += 1;
                }
                Err(e) => {
                    eprintln!("  ⚠ {} — failed to read response: {}", service.name(), e);
                    failed += 1;
                }
            },
            Err(e) => {
                eprintln!("  ⚠ {} — {}", service.name(), e);
                failed += 1;
            }
        }
    }

    println!("  Logos: {} downloaded, {} failed", downloaded, failed);
}
