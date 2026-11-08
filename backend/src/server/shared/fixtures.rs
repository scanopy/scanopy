//! Shared fixture generation logic used by both the `generate-fixtures` binary
//! and integration tests.

use crate::daemon::discovery::types::base::{DiscoveryPhase, DiscoveryTerminalReason};
use crate::daemon::discovery::types::warnings::{
    ClaimSource, DesktopVm, DiscoveryWarningCode, MalformedNeighbourConsequence, SnmpWalkCause,
    SnmpWalkGroup, WarningRemedy,
};
use crate::server::auth::access::Resource;
use crate::server::auth::r#impl::oidc::IdentityProviderLogo;
use crate::server::billing::plans::get_website_fixture_plans;
use crate::server::billing::types::base::{BillingPlan, CancelReason, PlanStatus, SaveOffer};
use crate::server::billing::types::features::Feature;
use crate::server::billing::types::limits::PlanLimit;
use crate::server::credentials::r#impl::integrations::all_integrations;
use crate::server::credentials::r#impl::mapping::CredentialQueryPayloadDiscriminants;
use crate::server::credentials::r#impl::types::{
    CredentialIntegration, CredentialTypeDiscriminants, SnmpV3AuthProtocol, SnmpV3PrivProtocol,
};
use crate::server::dependencies::r#impl::types::DependencyType;
use crate::server::discovery::r#impl::collection::all_collected_data;
use crate::server::discovery::r#impl::scan_settings::ScanSettings;
use crate::server::discovery::r#impl::types::DiscoveryType;
use crate::server::email::messages::EmailCategory;
use crate::server::hosts::r#impl::device_type::DeviceType;
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::ServiceDefinitionRegistry;
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::{ServiceDefinition, ServiceDefinitionExt};
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::shared::attribution::{AttributeMethod, AttributeSourceDiscriminants};
use crate::server::shared::concepts::Concept;
use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::logos::{Logo, Theming, logo_credits, logo_entries};
use crate::server::shared::product_area::ProductArea;
use crate::server::shared::types::entities::EntitySourceDiscriminants;
use crate::server::shared::types::entity_field::FieldGroup;
use crate::server::shared::types::metadata::{
    EntityMetadata, EntityMetadataProvider, MetadataProvider, TypeMetadata,
};
use crate::server::subnets::r#impl::types::SubnetType;
use crate::server::tags::r#impl::base::TagIcon;
use crate::server::topology::types::edges::EdgeType;
use crate::server::topology::types::grouping::{ContainerRule, ElementRule, ElementSort};
use crate::server::topology::types::nodes::ContainerType;
use crate::server::topology::types::views::{InspectorSection, TopologyView};
use crate::server::users::r#impl::permissions::UserOrgPermissions;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use strum::{IntoDiscriminant, IntoEnumIterator};

/// Generate all UI data fixture JSON files into the given output directory.
///
/// This is called by:
/// - `generate-fixtures` binary (local dev, Docker builds)
/// - Integration tests (CI release workflow)
pub fn generate_ui_data_fixtures(output_dir: &Path, force_logos: bool) {
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

    write_fixture(
        &crate::server::daemons::r#impl::version::UiVersionGates::current(),
        output_dir,
        "daemon-version-gates.json",
    );

    // The registry yields definitions in link order, which shifts between builds. Sorting keeps
    // the two service fixtures identical until a definition actually changes.
    let mut all_services = ServiceDefinitionRegistry::all_service_definitions();
    all_services.sort_by_key(|s| s.id());
    let service_defs: Vec<TypeMetadata> = all_services.iter().map(|t| t.to_metadata()).collect();
    write_fixture(&service_defs, output_dir, "service-definitions.json");

    // The website's services catalog: every definition a user can add, with the docs page of the
    // integration whose service it is.
    let services_catalog: Vec<serde_json::Value> = all_services
        .iter()
        .filter(|s| s.can_be_manually_added())
        .map(|s| {
            serde_json::json!({
                "logo": s.logo(),
                "name": s.name(),
                "description": s.description(),
                "discovery_pattern": s.discovery_pattern().to_string(),
                "category": s.category(),
                "color": s.color(),
                "icon": s.icon(),
                "integration_docs_path": CredentialIntegration::for_service(&**s)
                    .map(|i| i.docs_path()),
            })
        })
        .collect();
    write_fixture(&services_catalog, output_dir, "services.json");

    // Every logo, once, into ui/static/logos/catalog/<id>.<ext> (output_dir is ui/src/lib/data),
    // a `Paired` logo's dark-theme file beside it as <id>.dark.<ext>, and logos.json, which every
    // logo consumer looks a logo id up in.
    let urls: Vec<(String, String)> = Logo::iter()
        .filter(Logo::is_downloaded)
        .flat_map(|logo| {
            let id: &'static str = logo.into();
            std::iter::once((id.to_string(), logo.url()))
                .chain(logo.dark_url().map(|url| (format!("{id}.dark"), url)))
        })
        .collect();
    let wanted: Vec<(String, &str)> = urls
        .iter()
        .map(|(stem, url)| (stem.clone(), url.as_str()))
        .collect();
    let catalog_dir = output_dir.join("../../../static/logos/catalog");
    download_logos(&wanted, &catalog_dir, force_logos);
    // A `TextColor` logo's markup goes into logos.json, so the UI draws it inline in the text
    // colour as it draws its own icons. That only works while the file uses `currentColor`.
    let mut entries = logo_entries();
    for entry in entries
        .iter_mut()
        .filter(|e| e.theming == Theming::TextColor)
    {
        let file = catalog_dir.join(entry.id.file_name());
        let Ok(svg) = fs::read_to_string(&file) else {
            continue;
        };
        assert!(
            svg.contains("currentColor") && !svg.contains("<script"),
            "{} is drawn in the text colour, so it must use currentColor and carry no script",
            file.display()
        );
        entry.markup = Some(svg);
    }
    write_fixture(&entries, output_dir, "logos.json");

    // The website's OIDC guide lists these from identity-provider-logos.json, as paths oidc.toml
    // can name for a provider's logo.
    let identity_provider_logo_paths: Vec<serde_json::Value> = IdentityProviderLogo::iter()
        .map(|p| {
            let id: &'static str = p.into();
            serde_json::json!({
                "id": id,
                "name": p.name(),
                "path": p.logo().path(),
            })
        })
        .collect();
    write_fixture(
        &identity_provider_logo_paths,
        output_dir,
        "identity-provider-logos.json",
    );

    // Where each logo comes from and on what terms, for Settings > System.
    write_fixture(&logo_credits(), output_dir, "logo-credits.json");

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

    let snmp_walk_causes: Vec<TypeMetadata> =
        SnmpWalkCause::iter().map(|c| c.to_metadata()).collect();
    write_fixture(&snmp_walk_causes, output_dir, "snmp-walk-causes.json");

    let claim_sources: Vec<TypeMetadata> = ClaimSource::iter().map(|c| c.to_metadata()).collect();
    write_fixture(&claim_sources, output_dir, "claim-sources.json");

    // The VM a daemon was found running in, named in the DaemonInDesktopVm warning.
    let desktop_vms: Vec<TypeMetadata> = DesktopVm::iter().map(|v| v.to_metadata()).collect();
    write_fixture(&desktop_vms, output_dir, "desktop-vms.json");

    // The SNMPv3 protocols, with the names net-snmp takes for them, so a scan warning can hand an
    // operator the command that checks it with the stored credential's options.
    let snmp_auth_protocols: Vec<TypeMetadata> = SnmpV3AuthProtocol::iter()
        .map(|p| p.to_metadata())
        .collect();
    write_fixture(&snmp_auth_protocols, output_dir, "snmp-auth-protocols.json");

    let snmp_priv_protocols: Vec<TypeMetadata> = SnmpV3PrivProtocol::iter()
        .map(|p| p.to_metadata())
        .collect();
    write_fixture(&snmp_priv_protocols, output_dir, "snmp-priv-protocols.json");

    // Install OS → credential OS family, for the create-daemon modal and the daemon OS gate.
    let daemon_os: Vec<TypeMetadata> = crate::server::daemons::r#impl::base::DaemonOs::iter()
        .map(|o| o.to_metadata())
        .collect();
    write_fixture(&daemon_os, output_dir, "daemon-os.json");

    // A host OS family's label, for an OS whose source named no product.
    let host_os_families: Vec<TypeMetadata> =
        crate::server::hosts::r#impl::os::HostOsFamily::iter()
            .map(|f| f.to_metadata())
            .collect();
    write_fixture(&host_os_families, output_dir, "host-os-families.json");

    // A service match's confidence: its label and colour, and the services filter's options.
    let match_confidences: Vec<TypeMetadata> =
        crate::server::services::r#impl::patterns::MatchConfidence::iter()
            .map(|c| c.to_metadata())
            .collect();
    write_fixture(&match_confidences, output_dir, "match-confidences.json");

    // Which hypervisor platform a guest runs on, keyed by `virtualization_metadata.type`.
    let host_virtualizations: Vec<TypeMetadata> =
        crate::server::hosts::r#impl::virtualization::HostVirtualizationDiscriminants::iter()
            .map(|v| v.to_metadata())
            .collect();
    write_fixture(
        &host_virtualizations,
        output_dir,
        "host-virtualizations.json",
    );

    // Which container runtime a service runs in, keyed by a service's `virtualization_metadata.type`.
    let service_virtualizations: Vec<TypeMetadata> =
        crate::server::services::r#impl::virtualization::ServiceVirtualizationDiscriminants::iter()
            .map(|v| v.to_metadata())
            .collect();
    write_fixture(
        &service_virtualizations,
        output_dir,
        "service-virtualizations.json",
    );

    // The neighbor protocol that reported a physical link, keyed by `DiscoveryProtocol`.
    let discovery_protocols: Vec<TypeMetadata> =
        crate::server::topology::types::edges::DiscoveryProtocol::iter()
            .map(|p| p.to_metadata())
            .collect();
    write_fixture(&discovery_protocols, output_dir, "discovery-protocols.json");

    // A Proxmox guest's type (VM or LXC container), keyed by `ProxmoxGuestType`.
    let proxmox_guest_types: Vec<TypeMetadata> =
        crate::server::hosts::r#impl::virtualization::ProxmoxGuestType::iter()
            .map(|t| t.to_metadata())
            .collect();
    write_fixture(&proxmox_guest_types, output_dir, "proxmox-guest-types.json");

    // A container host's LAN network driver (macvlan or ipvlan), keyed by `ContainerNetworkType`.
    let container_network_types: Vec<TypeMetadata> =
        crate::server::hosts::r#impl::virtualization::ContainerNetworkType::iter()
            .map(|t| t.to_metadata())
            .collect();
    write_fixture(
        &container_network_types,
        output_dir,
        "container-network-types.json",
    );

    // A container's or VM's run state at the last scan, keyed by `GuestState`.
    let guest_states: Vec<TypeMetadata> =
        crate::server::shared::types::guest_state::GuestState::iter()
            .map(|s| s.to_metadata())
            .collect();
    write_fixture(&guest_states, output_dir, "guest-states.json");

    // A port's ifOperStatus, keyed by `IfOperStatus`: the inspector's status tag colour.
    let if_oper_statuses: Vec<TypeMetadata> =
        crate::server::interfaces::r#impl::base::IfOperStatus::iter()
            .map(|s| s.to_metadata())
            .collect();
    write_fixture(&if_oper_statuses, output_dir, "if-oper-statuses.json");

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

    // What the role comparison lists: each resource with its level under every role.
    let resources: Vec<TypeMetadata> = Resource::iter().map(|r| r.to_metadata()).collect();
    write_fixture(&resources, output_dir, "resources.json");

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

    // The parts of the product entities and plan features belong to, in display order: the
    // sidebar's sections, the global search's group order and the plan cards' headings.
    let product_areas: Vec<TypeMetadata> = ProductArea::iter().map(|a| a.to_metadata()).collect();
    write_fixture(&product_areas, output_dir, "product-areas.json");

    // Headings a field is listed under wherever fields are picked, in display order. Each field
    // in `entities.json` and `edge-types.json` names one in its `group`.
    let field_groups: Vec<TypeMetadata> = FieldGroup::iter().map(|g| g.to_metadata()).collect();
    write_fixture(&field_groups, output_dir, "field-groups.json");

    let concepts: Vec<EntityMetadata> = Concept::iter().map(|e| e.to_metadata()).collect();
    write_fixture(&concepts, output_dir, "concepts.json");

    let credential_types: Vec<TypeMetadata> = CredentialTypeDiscriminants::iter()
        .map(|d| d.to_metadata())
        .collect();
    write_fixture(&credential_types, output_dir, "credential-types.json");

    // The integration each credential type belongs to (`metadata.integration` on a type), for
    // the app's integration-first credential picker.
    let credential_integrations: Vec<TypeMetadata> = CredentialIntegration::iter()
        .map(|i| i.to_metadata())
        .collect();
    write_fixture(
        &credential_integrations,
        output_dir,
        "credential-integrations.json",
    );

    // Integrations: a service joined with the credential transports that target
    // it and one canonical discovery description (for the website integrations
    // page). Synced like service-definitions.json.
    let integrations = all_integrations();
    write_fixture(&integrations, output_dir, "integrations.json");

    // Each category of collected data with its credential-free methods and the integrations that
    // collect it, for the website security page. Synced like integrations.json.
    let collected_data = all_collected_data();
    write_fixture(&collected_data, output_dir, "collected-data.json");

    let scan_settings_fields = ScanSettings::field_definitions();
    write_fixture(&scan_settings_fields, output_dir, "scan-settings.json");

    // The SSH script contract: every key a script may print and the host field it fills. Rendered
    // by the website's SSH integration guide, so the docs table is the parser's own list.
    let ssh_script_fields =
        crate::server::credentials::r#impl::types::ssh_script::ssh_script_fields();
    write_fixture(&ssh_script_fields, output_dir, "ssh-script-fields.json");

    // Server flags, environment variables, defaults and descriptions, read from the server's own
    // CLI and config. Rendered by the website's server configuration reference.
    let server_config_fields = crate::server::config::ServerConfig::docs_fields();
    write_fixture(&server_config_fields, output_dir, "server-config.json");

    // SNMP versions, v3 protocols, ports, limits and every MIB object the daemon requests, from
    // the daemon's own constants. Rendered by the website's SNMP reference.
    let snmp_reference = crate::daemon::discovery::integration::snmp::catalog::snmp_reference();
    write_fixture(&snmp_reference, output_dir, "snmp.json");

    // The OS a credential's files and scripts are for: picker labels and file-path placeholders.
    let os_families: Vec<_> = crate::server::credentials::r#impl::types::OsFamily::iter()
        .map(|os| os.metadata())
        .collect();
    write_fixture(&os_families, output_dir, "os-families.json");

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

    let element_sorts: Vec<TypeMetadata> = ElementSort::iter().map(|s| s.to_metadata()).collect();
    write_fixture(&element_sorts, output_dir, "element-sorts.json");

    let device_types: Vec<TypeMetadata> = DeviceType::iter().map(|r| r.to_metadata()).collect();
    write_fixture(&device_types, output_dir, "device-types.json");

    let container_types: Vec<TypeMetadata> =
        ContainerType::iter().map(|r| r.to_metadata()).collect();
    write_fixture(&container_types, output_dir, "container-types.json");

    // Inspector section headings: name, one-line description, icon and colour.
    let inspector_sections: Vec<TypeMetadata> =
        InspectorSection::iter().map(|s| s.to_metadata()).collect();
    write_fixture(&inspector_sections, output_dir, "inspector-sections.json");

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

    let plan_limits: Vec<TypeMetadata> = PlanLimit::iter().map(|l| l.to_metadata()).collect();
    write_fixture(&plan_limits, output_dir, "plan-limits.json");

    let email_categories: Vec<TypeMetadata> =
        EmailCategory::iter().map(|c| c.to_metadata()).collect();
    write_fixture(&email_categories, output_dir, "email-categories.json");

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

    // Every icon a tag may carry, by the name the API stores. From the backend's lucide build so
    // the picker can offer nothing the server would refuse.
    let tag_icons: Vec<TagIcon> = TagIcon::all().collect();
    write_fixture(&tag_icons, output_dir, "tag-icons.json");

    println!("Done! Generated all metadata fixtures.");
}

/// Writes the fixture only when its content changed, so unchanged files keep their mtime and the
/// Vite dev server doesn't reload for nothing.
fn write_fixture<T: serde::Serialize + ?Sized>(items: &T, output_dir: &Path, filename: &str) {
    let json = serde_json::to_string_pretty(items).expect("Failed to serialize");
    let path = output_dir.join(filename);
    if fs::read(&path).is_ok_and(|existing| existing == json.as_bytes()) {
        return;
    }
    fs::write(&path, json).unwrap_or_else(|_| panic!("Failed to write {filename}"));
    println!("  {}", filename);
}

/// The repo README, whose `<!-- generated:<key> -->` spans `update_readme` fills.
fn readme_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../README.md")
}

/// The README's generated spans, by key. Each value matches what the website derives from the
/// same fixtures: `getServiceCountLabel` and `getIntegrationsLabel` in the website's `schemas.ts`.
fn readme_spans() -> Vec<(&'static str, String)> {
    // The `services.json` catalog: definitions a user can add, floored to ten ("300+").
    let services = ServiceDefinitionRegistry::all_service_definitions()
        .iter()
        .filter(|s| s.can_be_manually_added())
        .count();
    let integrations: Vec<String> = all_integrations().into_iter().map(|i| i.name).collect();
    vec![
        ("service-count", format!("{}+", services / 10 * 10)),
        ("integrations", list_label(&integrations)),
    ]
}

/// "A, B, and C", as the website joins names.
fn list_label(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [a, b] => format!("{a} and {b}"),
        [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
    }
}

/// `readme` with every generated span replaced by its current value. Panics on a missing marker,
/// so a README edit that drops one fails instead of leaving a stale value.
fn render_readme(readme: &str) -> String {
    const END: &str = "<!-- /generated -->";
    let mut out = readme.to_string();
    for (key, value) in readme_spans() {
        let start = format!("<!-- generated:{key} -->");
        let open = out
            .find(&start)
            .unwrap_or_else(|| panic!("README.md has no {start} marker"))
            + start.len();
        let close = out[open..]
            .find(END)
            .unwrap_or_else(|| panic!("README.md marker {start} has no {END}"))
            + open;
        out.replace_range(open..close, &value);
    }
    out
}

/// Rewrite the README's generated spans from the current definitions.
pub fn update_readme() {
    let path = readme_path();
    let readme = fs::read_to_string(&path).expect("Failed to read README.md");
    // Show what was read when no marker is present at all: in the v0.19.3 release a tool's
    // archive had overwritten README.md, and the marker panic alone didn't show it.
    if !readme.contains("<!-- generated:") {
        let start: String = readme.chars().take(400).collect();
        panic!(
            "README.md at {} ({} bytes) has no generated markers. It starts:\n{start}",
            path.display(),
            readme.len()
        );
    }
    let rendered = render_readme(&readme);
    if rendered != readme {
        fs::write(&path, rendered).expect("Failed to write README.md");
        println!("  README.md");
    }
}

/// Convert a service name to a URL-safe filename slug.
/// "Home Assistant" → "home-assistant", "Docker Container" → "docker-container"
pub fn logo_slug(name: &str) -> String {
    name.to_lowercase().replace(' ', "-")
}

/// Derive file extension from a logo URL.
/// "https://cdn.jsdelivr.net/.../docker.svg" → "svg"
pub(crate) fn logo_ext(url: &str) -> &str {
    url.rsplit('.')
        .next()
        .and_then(|e| e.split('?').next())
        .filter(|e| matches!(*e, "svg" | "png" | "webp"))
        .unwrap_or("svg")
}

/// Record of the URL each logo file in a directory was downloaded from, kept beside the logos.
const LOGO_CACHE_FILE: &str = ".logo-cache.json";

/// Sent on every logo request. Wikimedia refuses requests without a User-Agent, and CDNs
/// fronting Simple Icons refuse some anonymous requests from CI runners.
pub(crate) const LOGO_FETCH_USER_AGENT: &str = "scanopy-fixtures (https://scanopy.net)";

/// Whether `url` was already downloaded into `catalog_dir` as `<stem>.<ext>`: the file is there and
/// the cache record names this URL for it, the same test `plan_logos` uses to skip a download.
#[cfg(test)]
pub(crate) fn logo_is_cached(catalog_dir: &Path, stem: &str, url: &str) -> bool {
    let filename = format!("{stem}.{}", logo_ext(url));
    let cache: BTreeMap<String, String> = fs::read(catalog_dir.join(LOGO_CACHE_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    catalog_dir.join(&filename).is_file() && cache.get(&filename).map(String::as_str) == Some(url)
}

/// What `download_logos` does with a logo directory, worked out from the wanted logos, the cache
/// record and the files already on disk.
#[derive(Debug, PartialEq)]
struct LogoPlan {
    /// `(filename, url)` to fetch.
    download: Vec<(String, String)>,
    /// Files no current logo maps to, including one whose extension changed.
    prune: Vec<String>,
    /// The record with entries for removed logos dropped. Downloads add to it.
    cache: BTreeMap<String, String>,
}

/// A logo is fetched when `force` is set, its file is missing, or the record holds a different
/// URL for it. Empty and already-local (`/...`) URLs are skipped.
fn plan_logos(
    logos: &[(String, &str)],
    mut cache: BTreeMap<String, String>,
    existing_files: &BTreeSet<String>,
    force: bool,
) -> LogoPlan {
    let wanted: BTreeMap<String, String> = logos
        .iter()
        .filter(|(_, url)| !url.is_empty() && !url.starts_with('/'))
        .map(|(stem, url)| (format!("{stem}.{}", logo_ext(url)), url.to_string()))
        .collect();

    cache.retain(|filename, _| wanted.contains_key(filename));

    let download = wanted
        .iter()
        .filter(|(filename, url)| {
            force || !existing_files.contains(*filename) || cache.get(*filename) != Some(*url)
        })
        .map(|(filename, url)| (filename.clone(), url.clone()))
        .collect();

    let prune = existing_files
        .iter()
        .filter(|f| f.as_str() != LOGO_CACHE_FILE && !wanted.contains_key(*f))
        .cloned()
        .collect();

    LogoPlan {
        download,
        prune,
        cache,
    }
}

/// Download logos from CDN URLs to a local static directory, one `(file stem, url)` per logo.
/// Files are saved with extensions so the static server sets correct Content-Type. Only logos
/// that are missing or whose URL changed are fetched (all of them with `force`), and files for
/// logos that no longer exist are removed.
fn download_logos(logos: &[(String, &str)], static_dir: &Path, force: bool) {
    println!("Syncing logos in {}...", static_dir.display());
    fs::create_dir_all(static_dir).expect("Failed to create logos directory");

    let cache_path = static_dir.join(LOGO_CACHE_FILE);
    let cache: BTreeMap<String, String> = fs::read(&cache_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    let existing_files: BTreeSet<String> = fs::read_dir(static_dir)
        .expect("Failed to read logos directory")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_ok_and(|t| t.is_file()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();

    let LogoPlan {
        download,
        prune,
        mut cache,
    } = plan_logos(logos, cache, &existing_files, force);

    for filename in &prune {
        fs::remove_file(static_dir.join(filename))
            .unwrap_or_else(|_| panic!("Failed to remove logo {filename}"));
    }

    let mut downloaded = 0;
    let mut failed = 0;

    if !download.is_empty() {
        let client = reqwest::blocking::Client::builder()
            .user_agent(LOGO_FETCH_USER_AGENT)
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        for (filename, url) in download {
            let result = client
                .get(&url)
                .send()
                .and_then(|r| r.error_for_status())
                .and_then(|r| r.bytes());
            match result {
                Ok(bytes) => {
                    fs::write(static_dir.join(&filename), &bytes)
                        .unwrap_or_else(|_| panic!("Failed to write logo {filename}"));
                    cache.insert(filename, url);
                    downloaded += 1;
                }
                Err(e) => {
                    // Keep any previous file, but drop the record so the next run retries.
                    eprintln!("  ⚠ {filename} — {e}");
                    cache.remove(&filename);
                    failed += 1;
                }
            }
        }
    }

    let json = serde_json::to_string_pretty(&cache).expect("Failed to serialize logo cache");
    fs::write(&cache_path, json).expect("Failed to write logo cache");

    println!(
        "  Logos: {} downloaded, {} failed, {} removed, {} unchanged",
        downloaded,
        failed,
        prune.len(),
        cache.len() - downloaded
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readme_generated_spans_are_current() {
        let readme = fs::read_to_string(readme_path()).unwrap();
        assert!(
            render_readme(&readme) == readme,
            "README.md is stale: run `make generate-fixtures`"
        );
    }

    #[test]
    fn list_label_joins_like_the_website() {
        let names = |n: &[&str]| n.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(list_label(&names(&["A"])), "A");
        assert_eq!(list_label(&names(&["A", "B"])), "A and B");
        assert_eq!(list_label(&names(&["A", "B", "C"])), "A, B, and C");
    }

    fn files(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    fn cache(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
        entries
            .iter()
            .map(|(f, u)| (f.to_string(), u.to_string()))
            .collect()
    }

    fn downloads(plan: &LogoPlan) -> Vec<&str> {
        plan.download.iter().map(|(f, _)| f.as_str()).collect()
    }

    const DOCKER: &str = "https://cdn.example/docker.svg";
    const PLEX: &str = "https://cdn.example/plex.png";

    #[test]
    fn fetches_only_missing_or_changed_logos() {
        let logos = vec![
            ("docker".to_string(), DOCKER),
            ("plex".to_string(), PLEX),
            ("local".to_string(), "/logos/local.svg"),
            ("none".to_string(), ""),
        ];

        let fresh = plan_logos(&logos, BTreeMap::new(), &BTreeSet::new(), false);
        assert_eq!(downloads(&fresh), ["docker.svg", "plex.png"]);

        let cached = cache(&[("docker.svg", DOCKER), ("plex.png", PLEX)]);
        let on_disk = files(&["docker.svg", "plex.png", LOGO_CACHE_FILE]);
        let unchanged = plan_logos(&logos, cached.clone(), &on_disk, false);
        assert!(unchanged.download.is_empty());
        assert!(unchanged.prune.is_empty());

        let moved = cache(&[
            ("docker.svg", "https://old.example/docker.svg"),
            ("plex.png", PLEX),
        ]);
        assert_eq!(
            downloads(&plan_logos(&logos, moved, &on_disk, false)),
            ["docker.svg"]
        );

        let missing = files(&["plex.png", LOGO_CACHE_FILE]);
        assert_eq!(
            downloads(&plan_logos(&logos, cached.clone(), &missing, false)),
            ["docker.svg"]
        );

        assert_eq!(
            downloads(&plan_logos(&logos, cached, &on_disk, true)),
            ["docker.svg", "plex.png"]
        );
    }

    #[test]
    fn prunes_files_for_removed_logos_and_old_extensions() {
        let logos = vec![("plex".to_string(), PLEX)];
        let cached = cache(&[
            ("plex.svg", "https://cdn.example/plex.svg"),
            ("gone.svg", DOCKER),
        ]);
        let on_disk = files(&["plex.svg", "gone.svg", LOGO_CACHE_FILE]);

        let plan = plan_logos(&logos, cached, &on_disk, false);

        assert_eq!(downloads(&plan), ["plex.png"]);
        assert_eq!(plan.prune, ["gone.svg", "plex.svg"]);
        assert!(plan.cache.is_empty());
    }

    #[test]
    fn rewrites_a_fixture_only_when_its_content_changes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.json");
        let past = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);

        write_fixture(&["a"], dir.path(), "f.json");
        fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(past)
            .unwrap();

        write_fixture(&["a"], dir.path(), "f.json");
        assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), past);

        write_fixture(&["b"], dir.path(), "f.json");
        assert_ne!(fs::metadata(&path).unwrap().modified().unwrap(), past);
        assert!(fs::read_to_string(&path).unwrap().contains("\"b\""));
    }
}
