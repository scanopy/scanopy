//! What a signup wants Scanopy to do: the multi-select step after the use case.
//!
//! The answer is stored on the organization (`organizations.intents`) and drives three things,
//! each declared here per intent so the mapping has one home:
//! - the plan the picker recommends ([`SignupIntent::features`], read by
//!   `billing::recommendation`)
//! - the Setup items that move up the list ([`SignupIntent::setup_items`])
//! - the topology view the tab opens on ([`SignupIntent::topology_view`])
//!
//! The credential categories the Create Daemon wizard lists first go out in the metadata.
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoStaticStr};
use utoipa::ToSchema;

use crate::server::{
    billing::types::features::Feature,
    credentials::r#impl::types::CredentialCategory,
    setup::types::SetupItemDiscriminants,
    shared::{
        concepts::Concept,
        entities::EntityDiscriminants,
        product_area::ProductArea,
        types::{
            Color, Icon,
            metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
        },
    },
    topology::types::views::TopologyView,
};

/// Free text answering "Something else" is optional, and at most this long once trimmed.
pub const OTHER_ANSWER_MAX_CHARS: usize = 200;

/// A "Something else" answer longer than [`OTHER_ANSWER_MAX_CHARS`].
#[derive(Debug, PartialEq, Eq)]
pub struct OtherAnswerTooLong;

/// The trimmed answer, `None` when it is blank, or an error when it is too long.
pub fn other_answer(text: &str) -> Result<Option<String>, OtherAnswerTooLong> {
    let trimmed = text.trim();
    if trimmed.chars().count() > OTHER_ANSWER_MAX_CHARS {
        return Err(OtherAnswerTooLong);
    }
    Ok((!trimmed.is_empty()).then(|| trimmed.to_string()))
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    EnumIter,
    IntoStaticStr,
    ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SignupIntent {
    CurrentDiagram,
    Inventory,
    AuditEvidence,
    Troubleshooting,
    ClientDocumentation,
    Workloads,
    Other,
}

impl SignupIntent {
    /// The features this intent needs. The recommended plan is the lowest one that has all of
    /// them, for every intent picked.
    pub fn features(self) -> &'static [Feature] {
        match self {
            Self::CurrentDiagram => &[Feature::NetworkTopology, Feature::ScheduledDiscovery],
            Self::Inventory => &[Feature::DeviceInventory, Feature::CsvExport],
            Self::AuditEvidence => &[
                Feature::ConfluenceExport,
                Feature::DrawioExport,
                Feature::VsdxExport,
            ],
            Self::Troubleshooting => &[Feature::NetworkTopology],
            Self::ClientDocumentation => &[Feature::Sharing],
            Self::Workloads => &[Feature::WorkloadMapping, Feature::ApplicationMapping],
            Self::Other => &[],
        }
    }

    /// The Setup items this intent moves up the list, after the daemon, the first scan and the
    /// topology.
    pub fn setup_items(self) -> &'static [SetupItemDiscriminants] {
        use SetupItemDiscriminants as T;
        match self {
            Self::Troubleshooting => &[
                T::DependencyTutorial,
                T::Dependencies,
                T::InterfaceCredential,
            ],
            Self::Workloads => &[T::ApplicationTags],
            Self::Inventory => &[T::InterfaceCredential, T::Tags],
            Self::ClientDocumentation => &[T::MultiSite, T::Share],
            Self::CurrentDiagram | Self::AuditEvidence | Self::Other => &[],
        }
    }

    /// The topology view this intent opens on, when the site has the data for it.
    pub fn topology_view(self) -> TopologyView {
        match self {
            Self::Troubleshooting => TopologyView::L2Physical,
            Self::Workloads => TopologyView::Workloads,
            Self::CurrentDiagram
            | Self::Inventory
            | Self::AuditEvidence
            | Self::ClientDocumentation
            | Self::Other => TopologyView::L3Logical,
        }
    }

    /// The credential categories the Create Daemon wizard lists first.
    pub fn credential_categories(self) -> &'static [CredentialCategory] {
        use CredentialCategory as C;
        match self {
            Self::Workloads => &[C::Containers, C::Virtualization],
            Self::CurrentDiagram | Self::Inventory | Self::Troubleshooting => {
                &[C::NetworkMonitoring, C::NetworkController]
            }
            Self::AuditEvidence | Self::ClientDocumentation | Self::Other => &[],
        }
    }
}

impl HasId for SignupIntent {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for SignupIntent {
    fn color(&self) -> Color {
        Color::Gray
    }

    fn icon(&self) -> Icon {
        match self {
            // Each intent wears the icon of the product area, concept or entity it is about.
            Self::CurrentDiagram => ProductArea::Topology.icon(),
            Self::Inventory => ProductArea::Inventory.icon(),
            Self::AuditEvidence => ProductArea::Security.icon(),
            Self::Troubleshooting => Concept::L2.icon(),
            Self::ClientDocumentation => EntityDiscriminants::Share.icon(),
            Self::Workloads => Concept::Workloads.icon(),
            Self::Other => Icon::MessageCircle,
        }
    }
}

impl TypeMetadataProvider for SignupIntent {
    fn name(&self) -> &'static str {
        match self {
            Self::CurrentDiagram => "Keep a current diagram of the network",
            Self::Inventory => "Keep an inventory of devices and addresses",
            Self::AuditEvidence => "Produce evidence for an audit or compliance requirement",
            Self::Troubleshooting => "Find what is connected where when something breaks",
            Self::ClientDocumentation => "Document client networks and share them",
            Self::Workloads => "See which services and containers run where",
            Self::Other => "Something else",
        }
    }

    fn metadata(&self) -> serde_json::Value {
        let categories: Vec<&'static str> = self
            .credential_categories()
            .iter()
            .map(|c| c.into())
            .collect();
        serde_json::json!({ "credential_categories": categories })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_other_answer_is_optional_trimmed_and_capped() {
        assert_eq!(
            other_answer("  a school project  "),
            Ok(Some("a school project".to_string()))
        );
        assert_eq!(other_answer("ab"), Ok(Some("ab".to_string())));
        assert_eq!(other_answer(""), Ok(None));
        assert_eq!(other_answer("   "), Ok(None));
        assert_eq!(
            other_answer(&"x".repeat(OTHER_ANSWER_MAX_CHARS + 1)),
            Err(OtherAnswerTooLong)
        );
        // Characters, not bytes: 200 accented letters fit.
        assert!(other_answer(&"é".repeat(OTHER_ANSWER_MAX_CHARS)).is_ok());
    }
}
