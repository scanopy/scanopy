use crate::server::billing::types::base::BillingPlanDiscriminants;
use crate::server::shared::types::metadata::EntityMetadataProvider;
use crate::server::shared::types::metadata::HasId;
use crate::server::shared::types::metadata::TypeMetadataProvider;
use crate::server::shared::types::{Color, Icon};
use serde::Deserialize;
use serde::Serialize;
use strum::Display;
use strum::EnumIter;
use strum::IntoStaticStr;

#[derive(Debug, Clone, Serialize, Deserialize, EnumIter, IntoStaticStr, Display, Default)]
pub enum Feature {
    #[default]
    ShareViews,
    OnboardingCall,
    AuditLogs,
    Webhooks,
    RemoveCreatedWith,
    ApiAccess,
    CustomSso,
    Saml,
    AirGappedDeployment,
    ManagedDeployment,
    Whitelabeling,
    EmailSupport,
    LiveChatSupport,
    PrioritySupport,
    Embeds,
    NetworkMapping,
    PngExport,
    SvgExport,
    MermaidExport,
    ConfluenceExport,
    PdfExport,
    HtmlExport,
    ScheduledDiscovery,
    DiscoveryIntegrations,
    CsvExport,
    SnapshotRetentionDays,
}

impl HasId for Feature {
    fn id(&self) -> &'static str {
        match self {
            Feature::Webhooks => "webhooks",
            Feature::AuditLogs => "audit_logs",
            Feature::ShareViews => "share_views",
            Feature::OnboardingCall => "onboarding_call",
            Feature::RemoveCreatedWith => "remove_created_with",
            Feature::CustomSso => "custom_sso",
            Feature::Saml => "saml",
            Feature::AirGappedDeployment => "air_gapped_deployment",
            Feature::ManagedDeployment => "managed_deployment",
            Feature::Whitelabeling => "whitelabeling",
            Feature::LiveChatSupport => "live_chat_support",
            Feature::Embeds => "embeds",
            Feature::EmailSupport => "email_support",
            Feature::PrioritySupport => "priority_support",
            Feature::ApiAccess => "api_access",
            Feature::NetworkMapping => "network_mapping",
            Feature::PngExport => "png_export",
            Feature::SvgExport => "svg_export",
            Feature::MermaidExport => "mermaid_export",
            Feature::ConfluenceExport => "confluence_export",
            Feature::PdfExport => "pdf_export",
            Feature::HtmlExport => "html_export",
            Feature::ScheduledDiscovery => "scheduled_discovery",
            Feature::DiscoveryIntegrations => "discovery_integrations",
            Feature::CsvExport => "csv_export",
            Feature::SnapshotRetentionDays => "snapshot_retention_days",
        }
    }
}

impl Feature {
    pub fn is_coming_soon(&self) -> bool {
        matches!(
            self,
            Feature::Webhooks | Feature::AuditLogs | Feature::Saml | Feature::Whitelabeling
        )
    }

    /// Returns the ID of the lowest-tier cloud plan that includes this feature.
    pub fn minimum_plan(&self) -> Option<&'static str> {
        use super::base::BillingPlan;

        let feature_id = self.id();
        let cloud_tiers = [
            BillingPlanDiscriminants::Free,
            BillingPlanDiscriminants::Starter,
            BillingPlanDiscriminants::Pro,
            BillingPlanDiscriminants::Business,
            BillingPlanDiscriminants::Enterprise,
        ];

        for disc in &cloud_tiers {
            if let Some(plan) = BillingPlan::default_for_discriminant(*disc)
                && plan.has_feature(feature_id)
            {
                return Some(plan.id());
            }
        }
        None
    }
}

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
        match self {
            Feature::NetworkMapping
            | Feature::DiscoveryIntegrations
            | Feature::ScheduledDiscovery => "Discovery",

            Feature::PngExport
            | Feature::SvgExport
            | Feature::MermaidExport
            | Feature::ConfluenceExport
            | Feature::PdfExport
            | Feature::HtmlExport
            | Feature::Embeds
            | Feature::ShareViews
            | Feature::RemoveCreatedWith
            | Feature::SnapshotRetentionDays => "Visualization",

            Feature::EmailSupport
            | Feature::LiveChatSupport
            | Feature::PrioritySupport
            | Feature::OnboardingCall => "Support",

            Feature::CustomSso
            | Feature::Saml
            | Feature::AirGappedDeployment
            | Feature::ManagedDeployment
            | Feature::Whitelabeling
            | Feature::AuditLogs => "Enterprise",

            Feature::CsvExport | Feature::Webhooks | Feature::ApiAccess => "Integrations",
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Feature::AuditLogs => "Audit Logs",
            Feature::Webhooks => "Webhooks",
            Feature::ShareViews => "Shareable Diagrams",
            Feature::OnboardingCall => "Onboarding Call",
            Feature::RemoveCreatedWith => "Remove Watermark",
            Feature::CustomSso => "Custom SSO",
            Feature::Saml => "SAML",
            Feature::AirGappedDeployment => "Air-gapped Deployment",
            Feature::ManagedDeployment => "Managed Deployment",
            Feature::Whitelabeling => "White Labeling",
            Feature::LiveChatSupport => "Live Chat Support",
            Feature::Embeds => "Embeddable Diagrams",
            Feature::ApiAccess => "API Access",
            Feature::EmailSupport => "Email Support",
            Feature::PrioritySupport => "Priority Support",
            Feature::NetworkMapping => "Automated Network Mapping",
            Feature::PngExport => "PNG Export",
            Feature::SvgExport => "SVG Export",
            Feature::MermaidExport => "Mermaid Export",
            Feature::ConfluenceExport => "Confluence Export",
            Feature::PdfExport => "PDF Export",
            Feature::HtmlExport => "HTML Export",
            Feature::ScheduledDiscovery => "Scheduled Discovery",
            Feature::DiscoveryIntegrations => "Discovery Integrations",
            Feature::CsvExport => "CSV Export",
            Feature::SnapshotRetentionDays => "Snapshot Retention",
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
            Feature::ShareViews => "Share live network diagrams with others",
            Feature::OnboardingCall => {
                "30 minute onboarding call to ensure you're getting the most out of Scanopy"
            }
            Feature::RemoveCreatedWith => {
                "Remove 'Created using scanopy.net' in bottom right corner of exported images"
            }
            Feature::ApiAccess => "Programmatic access to your data in Scanopy via API",
            Feature::PrioritySupport => "Prioritized email support with faster response times",
            Feature::Embeds => "Embed live network diagrams in wikis, dashboards, or documentation",
            Feature::CustomSso => {
                "Use your own identity provider (Okta, Azure AD, etc.) for single sign-on"
            }
            Feature::Saml => {
                "Connect your SAML 2.0 identity provider for enterprise single sign-on"
            }
            Feature::AirGappedDeployment => {
                "Runs fully offline. Deploy in networks with no outbound internet access"
            }
            Feature::ManagedDeployment => {
                "We deploy, configure, and manage Scanopy for you on a dedicated instance"
            }
            Feature::EmailSupport => "Access to the Scanopy team via email support tickets",
            Feature::Whitelabeling => "Deploy Scanopy with your logo and brand colors",
            Feature::LiveChatSupport => "Access to the Scanopy team via live chat",
            Feature::NetworkMapping => {
                "Automatically discover hosts, services, and connections and map them as interactive topology diagrams"
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
            Feature::ScheduledDiscovery => "Schedule automatic network discovery scans",
            Feature::DiscoveryIntegrations => {
                "Discover Docker containers and query network devices via SNMP"
            }
            Feature::CsvExport => {
                "Download host, service, and network data as CSV for use in spreadsheets and other tools"
            }
            Feature::SnapshotRetentionDays => {
                "How long captured snapshots are retained before automatic deletion"
            }
        }
    }

    fn metadata(&self) -> serde_json::Value {
        serde_json::json!({
            "is_coming_soon": self.is_coming_soon(),
            "minimum_plan": self.minimum_plan()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::billing::types::base::BillingPlan;
    use std::collections::HashSet;
    use strum::IntoEnumIterator;

    #[test]
    fn test_feature_ids_match_billing_plan_features_fields() {
        // Get all Feature IDs
        let feature_ids: HashSet<&str> = Feature::iter().map(|f| f.id()).collect();

        // Get all keys from BillingPlanFeatures by serializing an instance
        let features = BillingPlan::default().features();
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
}
