//! Shared fixture generation logic used by both the `generate-fixtures` binary
//! and integration tests.

use crate::server::billing::plans::get_website_fixture_plans;
use crate::server::billing::types::base::BillingPlan;
use crate::server::billing::types::features::Feature;
use crate::server::credentials::r#impl::types::CredentialTypeDiscriminants;
use crate::server::discovery::r#impl::scan_settings::ScanSettings;
use crate::server::discovery::r#impl::types::DiscoveryType;
use crate::server::groups::r#impl::types::GroupType;
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::ServiceDefinitionRegistry;
use crate::server::shared::concepts::Concept;
use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::types::metadata::{EntityMetadata, MetadataProvider, TypeMetadata};
use crate::server::subnets::r#impl::types::SubnetType;
use crate::server::topology::types::edges::EdgeType;
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

    let service_defs: Vec<TypeMetadata> = ServiceDefinitionRegistry::all_service_definitions()
        .iter()
        .map(|t| t.to_metadata())
        .collect();
    write_fixture(&service_defs, output_dir, "service-definitions.json");

    let subnet_types: Vec<TypeMetadata> = SubnetType::iter().map(|t| t.to_metadata()).collect();
    write_fixture(&subnet_types, output_dir, "subnet-types.json");

    let edge_types: Vec<TypeMetadata> = EdgeType::iter().map(|t| t.to_metadata()).collect();
    write_fixture(&edge_types, output_dir, "edge-types.json");

    let group_types: Vec<TypeMetadata> = GroupType::iter()
        .map(|t| t.discriminant().to_metadata())
        .collect();
    write_fixture(&group_types, output_dir, "group-types.json");

    let ports: Vec<TypeMetadata> = PortType::iter().map(|p| p.to_metadata()).collect();
    write_fixture(&ports, output_dir, "ports.json");

    let discovery_types: Vec<TypeMetadata> =
        DiscoveryType::iter().map(|d| d.to_metadata()).collect();
    write_fixture(&discovery_types, output_dir, "discovery-types.json");

    let permissions: Vec<TypeMetadata> = UserOrgPermissions::iter()
        .map(|p| p.to_metadata())
        .collect();
    write_fixture(&permissions, output_dir, "permissions.json");

    // EntityMetadata categories
    let entities: Vec<EntityMetadata> = EntityDiscriminants::iter()
        .map(|e| e.to_metadata())
        .collect();
    write_fixture(&entities, output_dir, "entities.json");

    let concepts: Vec<EntityMetadata> = Concept::iter().map(|e| e.to_metadata()).collect();
    write_fixture(&concepts, output_dir, "concepts.json");

    let credential_types: Vec<TypeMetadata> = CredentialTypeDiscriminants::iter()
        .map(|d| d.to_metadata())
        .collect();
    write_fixture(&credential_types, output_dir, "credential-types.json");

    let scan_settings_fields = ScanSettings::field_definitions();
    write_fixture(&scan_settings_fields, output_dir, "scan-settings.json");

    println!("Done! Generated all metadata fixtures.");
}

fn write_fixture<T: serde::Serialize>(items: &[T], output_dir: &Path, filename: &str) {
    let json = serde_json::to_string_pretty(items).expect("Failed to serialize");
    let path = output_dir.join(filename);
    fs::write(&path, json).unwrap_or_else(|_| panic!("Failed to write {filename}"));
    println!("  {}", filename);
}
