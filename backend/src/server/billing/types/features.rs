use crate::server::billing::types::base::BillingPlanDiscriminants;
use crate::server::shared::product_area::ProductArea;
use crate::server::shared::types::metadata::EntityMetadataProvider;
use crate::server::shared::types::metadata::HasId;
use crate::server::shared::types::metadata::TypeMetadataProvider;
use crate::server::shared::types::{Color, Icon, Stability};
use serde::Deserialize;
use serde::Serialize;
use strum::Display;
use strum::EnumIter;
use strum::IntoStaticStr;

/// Declaration order is display order: grouped by [`area`](Feature::area), and within a
/// area in the order plan cards list them (`BillingPlan::incremental_features`).
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    EnumIter,
    IntoStaticStr,
    Display,
    Default,
)]
pub enum Feature {
    // Topology: the diagrams, their snapshots, and ways to share and export them
    NetworkTopology,
    WorkloadMapping,
    ApplicationMapping,
    SnapshotRetentionDays,
    #[default]
    Sharing,
    Exports,
    PngExport,
    SvgExport,
    PdfExport,
    HtmlExport,
    MermaidExport,
    ConfluenceExport,
    DrawioExport,
    VsdxExport,
    RemoveCreatedWith,
    // Inventory
    DeviceInventory,
    VlanSubnetInventory,
    Tagging,
    CsvExport,
    // Discovery
    NetworkDiscovery,
    ScheduledDiscovery,
    DiscoveryIntegrations,
    InstantOnIntegration,
    ServiceDetection,
    // API
    ApiAccess,
    Webhooks,
    McpServer,
    // Support
    EmailSupport,
    LiveChatSupport,
    PrioritySupport,
    OnboardingCall,
    SignedSla,
    // Security
    CustomSso,
    Saml,
    AuditLogs,
    // Deployment
    AirGappedDeployment,
    ManagedDeployment,
    Whitelabeling,
    // Billing
    InvoiceBilling,
    CustomBilling,
    ProcurementDocuments,
}

impl HasId for Feature {
    fn id(&self) -> &'static str {
        match self {
            Feature::Webhooks => "webhooks",
            Feature::McpServer => "mcp_server",
            Feature::AuditLogs => "audit_logs",
            Feature::OnboardingCall => "onboarding_call",
            Feature::RemoveCreatedWith => "remove_created_with",
            Feature::CustomSso => "custom_sso",
            Feature::Saml => "saml",
            Feature::AirGappedDeployment => "air_gapped_deployment",
            Feature::ManagedDeployment => "managed_deployment",
            Feature::Whitelabeling => "whitelabeling",
            Feature::LiveChatSupport => "live_chat_support",
            Feature::EmailSupport => "email_support",
            Feature::PrioritySupport => "priority_support",
            Feature::ApiAccess => "api_access",
            Feature::DeviceInventory => "device_inventory",
            Feature::VlanSubnetInventory => "vlan_subnet_inventory",
            Feature::Tagging => "tagging",
            Feature::NetworkTopology => "network_topology",
            Feature::WorkloadMapping => "workload_mapping",
            Feature::ApplicationMapping => "application_mapping",
            Feature::ServiceDetection => "service_detection",
            Feature::PngExport => "png_export",
            Feature::SvgExport => "svg_export",
            Feature::MermaidExport => "mermaid_export",
            Feature::ConfluenceExport => "confluence_export",
            Feature::PdfExport => "pdf_export",
            Feature::HtmlExport => "html_export",
            Feature::DrawioExport => "drawio_export",
            Feature::VsdxExport => "vsdx_export",
            Feature::NetworkDiscovery => "network_discovery",
            Feature::ScheduledDiscovery => "scheduled_discovery",
            Feature::DiscoveryIntegrations => "discovery_integrations",
            Feature::InstantOnIntegration => "instant_on_integration",
            Feature::CsvExport => "csv_export",
            Feature::SnapshotRetentionDays => "snapshot_retention_days",
            Feature::Sharing => "sharing",
            Feature::Exports => "exports",
            Feature::InvoiceBilling => "invoice_billing",
            Feature::CustomBilling => "custom_billing",
            Feature::ProcurementDocuments => "procurement_documents",
            Feature::SignedSla => "signed_sla",
        }
    }
}

impl Feature {
    pub fn area(&self) -> ProductArea {
        match self {
            Feature::NetworkDiscovery
            | Feature::ScheduledDiscovery
            | Feature::DiscoveryIntegrations
            | Feature::InstantOnIntegration
            | Feature::ServiceDetection => ProductArea::Discovery,

            Feature::NetworkTopology
            | Feature::WorkloadMapping
            | Feature::ApplicationMapping
            | Feature::SnapshotRetentionDays
            | Feature::Sharing
            | Feature::Exports
            | Feature::RemoveCreatedWith
            | Feature::PngExport
            | Feature::SvgExport
            | Feature::PdfExport
            | Feature::HtmlExport
            | Feature::MermaidExport
            | Feature::ConfluenceExport
            | Feature::DrawioExport
            | Feature::VsdxExport => ProductArea::Topology,

            Feature::DeviceInventory
            | Feature::VlanSubnetInventory
            | Feature::Tagging
            | Feature::CsvExport => ProductArea::Inventory,

            Feature::ApiAccess | Feature::Webhooks | Feature::McpServer => ProductArea::Api,

            Feature::EmailSupport
            | Feature::LiveChatSupport
            | Feature::PrioritySupport
            | Feature::OnboardingCall
            | Feature::SignedSla => ProductArea::Support,

            Feature::CustomSso | Feature::Saml | Feature::AuditLogs => ProductArea::Security,

            Feature::AirGappedDeployment | Feature::ManagedDeployment | Feature::Whitelabeling => {
                ProductArea::Deployment
            }

            Feature::InvoiceBilling | Feature::CustomBilling | Feature::ProcurementDocuments => {
                ProductArea::Billing
            }
        }
    }

    /// Whether plan cards and the comparison list this feature. Remove Watermark is left off: every
    /// plan the pricing page shows either includes it or has no diagram exports to watermark (Free);
    /// only Community, which the page doesn't list, exports with the watermark.
    pub fn is_listed_on_plans(&self) -> bool {
        !matches!(self, Feature::RemoveCreatedWith)
    }

    /// Whether the website's Product menu gives this feature an entry; the website reads nothing
    /// else to decide. Off for a feature the plans don't list, for a sub-feature (it shows under its
    /// parent), and for a top-line feature the product pages present in sections with no entry of
    /// its own: CSV Export applies to every inventory entity, and Scheduled Discovery comes with
    /// discovery.
    pub fn is_listed_in_product_menu(&self) -> bool {
        self.is_listed_on_plans()
            && self.parent().is_none()
            && !matches!(self, Feature::CsvExport | Feature::ScheduledDiscovery)
    }

    /// The feature this one nests under. Plan cards list it indented under its parent, and the
    /// Product menu shows only the parent.
    pub fn parent(&self) -> Option<Feature> {
        if self.export_format().is_some() {
            Some(Feature::Exports)
        } else if self.gates_integration() {
            // A feature that unlocks one integration is part of Discovery Integrations.
            Some(Feature::DiscoveryIntegrations)
        } else {
            None
        }
    }

    /// A feature no plan sets on its own: it groups the features under it (Exports) and is on for a
    /// plan when any of them is (`BillingPlan::feature_values`).
    pub fn is_group(&self) -> bool {
        matches!(self, Feature::Exports)
    }

    /// The format a diagram export writes, which names its row under Exports; `None` for any
    /// other feature, CSV export included, which exports inventory data rather than a diagram.
    pub fn export_format(&self) -> Option<&'static str> {
        match self {
            Feature::PngExport => Some("PNG"),
            Feature::SvgExport => Some("SVG"),
            Feature::PdfExport => Some("PDF"),
            Feature::HtmlExport => Some("HTML"),
            Feature::MermaidExport => Some("Mermaid"),
            Feature::ConfluenceExport => Some("Confluence"),
            Feature::DrawioExport => Some("draw.io"),
            Feature::VsdxExport => Some("Visio"),
            _ => None,
        }
    }

    /// Release maturity. A beta feature opens to beta program members first, and its Beta tag
    /// links to the beta program sign-up. Plan cards hide that tag while the feature is also
    /// coming soon, and show the Coming Soon tag alone.
    pub fn stability(&self) -> Stability {
        match self {
            Feature::McpServer => Stability::Beta,
            _ => Stability::Stable,
        }
    }

    pub fn is_coming_soon(&self) -> bool {
        matches!(
            self,
            Feature::Webhooks
                | Feature::McpServer
                | Feature::AuditLogs
                | Feature::Saml
                | Feature::Whitelabeling
        )
    }

    /// Whether some credential type needs this feature to be created (see
    /// `CredentialTypeDiscriminants::required_feature`).
    pub fn gates_integration(&self) -> bool {
        use crate::server::credentials::r#impl::types::CredentialTypeDiscriminants;
        use strum::IntoEnumIterator;

        let id = self.id();
        CredentialTypeDiscriminants::iter()
            .any(|t| t.required_feature().is_some_and(|f| f.id() == id))
    }

    /// Returns the ID of the lowest-tier cloud plan that includes this feature.
    pub fn minimum_plan(&self) -> Option<&'static str> {
        self.minimum_plan_on(&CLOUD_LADDER).map(|disc| disc.into())
    }

    /// The lowest plan on `ladder` that includes this feature.
    pub fn minimum_plan_on(
        &self,
        ladder: &[BillingPlanDiscriminants],
    ) -> Option<BillingPlanDiscriminants> {
        use super::base::BillingPlan;

        let feature_id = self.id();
        ladder.iter().copied().find(|disc| {
            BillingPlan::default_for_discriminant(*disc)
                .is_some_and(|plan| plan.has_feature(feature_id))
        })
    }
}

/// The cloud plans sold today, lowest first. Starter and Team still exist for orgs that hold
/// them but are no longer sold, so nothing is recommended or "available from" on them.
pub const CLOUD_LADDER: [BillingPlanDiscriminants; 4] = [
    BillingPlanDiscriminants::Free,
    BillingPlanDiscriminants::Pro,
    BillingPlanDiscriminants::Business,
    BillingPlanDiscriminants::Enterprise,
];

/// The self-hosted plans sold today, lowest first.
pub const SELF_HOSTED_LADDER: [BillingPlanDiscriminants; 3] = [
    BillingPlanDiscriminants::Community,
    BillingPlanDiscriminants::SelfHostedStandard,
    BillingPlanDiscriminants::SelfHostedPlus,
];

impl EntityMetadataProvider for Feature {
    fn color(&self) -> Color {
        Color::Gray
    }

    fn icon(&self) -> Icon {
        Icon::Sparkle
    }
}

impl TypeMetadataProvider for Feature {
    fn category(&self) -> &'static str {
        self.area().id()
    }

    fn name(&self) -> &'static str {
        match self {
            Feature::AuditLogs => "Audit Logs",
            Feature::Webhooks => "Webhooks",
            Feature::McpServer => "MCP Server",
            Feature::OnboardingCall => "Onboarding Call",
            Feature::RemoveCreatedWith => "Remove Watermark",
            Feature::CustomSso => "Custom SSO",
            Feature::Saml => "SAML",
            Feature::AirGappedDeployment => "Air-gapped Deployment",
            Feature::ManagedDeployment => "Managed Deployment",
            Feature::Whitelabeling => "White Labeling",
            Feature::LiveChatSupport => "Live Chat Support",
            Feature::ApiAccess => "API Access",
            Feature::EmailSupport => "Email Support",
            Feature::PrioritySupport => "Priority Support",
            Feature::DeviceInventory => "Device Inventory",
            Feature::VlanSubnetInventory => "VLAN & Subnet Inventory",
            Feature::Tagging => "Tagging",
            Feature::NetworkTopology => "Network Mapping",
            Feature::WorkloadMapping => "Workload Mapping",
            Feature::ApplicationMapping => "Application Mapping",
            Feature::ServiceDetection => "Service Detection",
            Feature::PngExport => "PNG Export",
            Feature::SvgExport => "SVG Export",
            Feature::MermaidExport => "Mermaid Export",
            Feature::ConfluenceExport => "Confluence Export",
            Feature::PdfExport => "PDF Export",
            Feature::HtmlExport => "HTML Export",
            Feature::DrawioExport => "draw.io Export",
            Feature::VsdxExport => "Visio Export",
            Feature::NetworkDiscovery => "Network Discovery",
            Feature::ScheduledDiscovery => "Scheduled Discovery",
            Feature::DiscoveryIntegrations => "Discovery Integrations",
            Feature::InstantOnIntegration => "Aruba Instant On",
            Feature::CsvExport => "CSV Export",
            Feature::SnapshotRetentionDays => "Snapshots",
            Feature::Sharing => "Sharing",
            Feature::Exports => "Exports",
            Feature::InvoiceBilling => "Invoice & PO Billing",
            Feature::CustomBilling => "Custom Billing",
            Feature::ProcurementDocuments => "Procurement Documents",
            Feature::SignedSla => "Signed SLA",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Feature::AuditLogs => {
                "Track all user actions and data changes for compliance and security"
            }
            Feature::Webhooks => {
                "Push real-time events to external systems when hosts, services, or topology changes"
            }
            Feature::McpServer => {
                "Connect AI assistants to your network data in Scanopy through the Model Context Protocol"
            }
            Feature::OnboardingCall => {
                "A call with the Scanopy team to get Scanopy set up, deployed and mapping your first network"
            }
            Feature::RemoveCreatedWith => {
                "Remove 'Created using scanopy.net' in bottom right corner of exported images"
            }
            Feature::ApiAccess => "Programmatic access to your data in Scanopy via API",
            Feature::PrioritySupport => "Prioritized email support with faster response times",
            Feature::CustomSso => {
                "Use your own identity provider (Okta, Azure AD, etc.) for single sign-on"
            }
            Feature::Saml => {
                "Connect your SAML 2.0 identity provider for enterprise single sign-on"
            }
            Feature::AirGappedDeployment => {
                "License validated fully offline. Deploy in networks with no outbound internet access"
            }
            Feature::ManagedDeployment => {
                "We deploy, configure, and manage Scanopy for you on a dedicated instance"
            }
            Feature::EmailSupport => "Access to the Scanopy team via email support tickets",
            Feature::Whitelabeling => "Deploy Scanopy with your logo and brand colors",
            Feature::LiveChatSupport => "Access to the Scanopy team via live chat",
            Feature::DeviceInventory => {
                "A searchable inventory of every host discovery finds, with its addresses, services, ports and interfaces"
            }
            Feature::Tagging => {
                "Tag anything in Scanopy, from hosts and VLANs to credentials and API keys, to group and filter it"
            }
            Feature::VlanSubnetInventory => {
                "VLANs read from your switches, linked to the subnets behind them"
            }
            Feature::NetworkTopology => {
                "Physical (L2) and logical (L3) diagrams of your network, drawn from the network itself"
            }
            Feature::WorkloadMapping => {
                "Diagrams of hypervisors, VMs and containers, nested the way they run"
            }
            Feature::ApplicationMapping => {
                "Diagrams of each application, the services it depends on and the hosts they run on"
            }
            Feature::ServiceDetection => {
                "Recognize the services running on each host, from databases and web servers to printers and IoT devices"
            }
            Feature::PngExport => "Export network diagrams as high-resolution PNG images",
            Feature::SvgExport => "Export network diagrams as scalable SVG vector images",
            Feature::MermaidExport => {
                "Export topology as Mermaid flowchart for use in documentation and wikis"
            }
            Feature::ConfluenceExport => {
                "Export topology as Confluence wiki markup tables for team documentation"
            }
            Feature::PdfExport => "Export network diagrams as printable PDF documents",
            Feature::HtmlExport => {
                "Export network diagrams as self-contained HTML pages for offline viewing"
            }
            Feature::DrawioExport => {
                "Export network diagrams as editable draw.io files with shapes, groups and connectors"
            }
            Feature::VsdxExport => {
                "Export network diagrams as editable Visio (VSDX) files with shapes, groups and connectors"
            }
            Feature::NetworkDiscovery => {
                "Scan your subnets for hosts, open ports and running services"
            }
            Feature::ScheduledDiscovery => "Schedule automatic network discovery scans",
            Feature::DiscoveryIntegrations => {
                "Credentials that let discovery read switches, controllers, hypervisors, container runtimes and hosts"
            }
            Feature::InstantOnIntegration => {
                "Add Aruba Instant On credentials so discovery reads switches, access points and their links from the Instant On portal"
            }
            Feature::CsvExport => {
                "Download host, service, and network data as CSV for use in spreadsheets and other tools"
            }
            Feature::SnapshotRetentionDays => {
                "Point-in-time copies of your network map, kept for your plan's retention window"
            }
            Feature::Sharing => "Share live diagrams by link or embed",
            Feature::Exports => "Export diagrams to image, document and diagram files",
            Feature::InvoiceBilling => "Pay by invoice against a purchase order instead of by card",
            Feature::CustomBilling => {
                "Billing schedule and payment terms negotiated to fit your procurement process"
            }
            Feature::ProcurementDocuments => {
                "W-9 and NDAA Section 889 attestation for vendor onboarding"
            }
            Feature::SignedSla => {
                "A signed service level agreement with committed support response times"
            }
        }
    }

    fn metadata(&self) -> serde_json::Value {
        serde_json::json!({
            "is_coming_soon": self.is_coming_soon(),
            "stability": self.stability(),
            "minimum_plan": self.minimum_plan(),
            // Plan cards list the integrations themselves, so they skip this feature's line.
            "gates_integration": self.gates_integration(),
            "listed_on_plans": self.is_listed_on_plans(),
            "listed_in_product_menu": self.is_listed_in_product_menu(),
            // Plan cards list a feature under its parent, an export by its format and any other
            // feature by its name.
            "parent": self.parent().map(|p| p.id()),
            "export_format": self.export_format()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::billing::plans::get_community_plan;
    use std::collections::HashSet;
    use strum::IntoEnumIterator;

    #[test]
    fn test_feature_ids_match_billing_plan_features_fields() {
        // Get all Feature IDs
        // Group features (Exports) have no field: their value derives from their members.
        let feature_ids: HashSet<&str> = Feature::iter()
            .filter(|f| !f.is_group())
            .map(|f| f.id())
            .collect();

        // Get all keys from BillingPlanFeatures by serializing an instance
        let features = get_community_plan().features();
        let features_json = serde_json::to_value(&features).expect("Failed to serialize features");
        let features_map = features_json
            .as_object()
            .expect("Features should be an object");

        let billing_plan_features: HashSet<&str> =
            features_map.keys().map(|s| s.as_str()).collect();

        // Check that every Feature ID exists in BillingPlanFeatures
        for feature_id in &feature_ids {
            assert!(
                billing_plan_features.contains(feature_id),
                "Feature ID '{}' does not exist in BillingPlanFeatures",
                feature_id
            );
        }

        // Check that every BillingPlanFeatures field has a corresponding Feature
        for feature in &billing_plan_features {
            assert!(
                feature_ids.contains(feature),
                "BillingPlanFeatures field '{}' does not have a corresponding Feature variant",
                feature
            );
        }

        // Verify they have the same count
        assert_eq!(
            feature_ids.len(),
            billing_plan_features.len(),
            "Feature enum has {} variants but BillingPlanFeatures has {} fields",
            feature_ids.len(),
            billing_plan_features.len()
        );
    }

    /// A feature lists under its parent, so the parent has to be a listed, top-line feature of the
    /// same category, declared before it. A group feature has to group something.
    #[test]
    fn sub_features_follow_a_top_line_parent_in_their_category() {
        let order: Vec<&str> = Feature::iter().map(|f| f.id()).collect();
        for group in Feature::iter().filter(|f| f.is_group()) {
            assert!(
                Feature::iter().any(|f| f.parent() == Some(group)),
                "{} groups no feature",
                group.id()
            );
        }
        for feature in Feature::iter() {
            let Some(parent) = feature.parent() else {
                continue;
            };
            assert!(
                parent.parent().is_none(),
                "{} nests under a sub-feature",
                feature.id()
            );
            assert!(
                parent.is_listed_on_plans(),
                "{} nests under an unlisted feature",
                feature.id()
            );
            assert_eq!(
                parent.area(),
                feature.area(),
                "{} nests across categories",
                feature.id()
            );
            let pos = |id| order.iter().position(|o| *o == id);
            assert!(
                pos(parent.id()) < pos(feature.id()),
                "{} is declared before its parent",
                feature.id()
            );
        }
    }

    /// Plan cards list features in declaration order within each area, so a feature declared
    /// away from its area's other variants lands out of place in that list.
    #[test]
    fn features_are_declared_grouped_by_category() {
        let areas: Vec<(&str, ProductArea)> = Feature::iter().map(|f| (f.id(), f.area())).collect();
        assert!(
            areas.is_sorted_by_key(|(_, area)| *area),
            "declare each feature among its area, areas in ProductArea order: {areas:?}"
        );
    }
}
