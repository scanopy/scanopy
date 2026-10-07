//! Which part of the product each entity belongs to.
//!
//! One source for grouping and ordering entities in the UI: the sidebar builds its sections from
//! these categories, and the global search lists its result groups in the same order. Variant
//! order is display order; within a category, entities follow the `Entity` enum's order.

use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter, IntoStaticStr};

use crate::server::shared::types::{
    Color, Icon,
    metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Display,
    EnumIter,
    IntoStaticStr,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum EntityCategory {
    /// What is on the network: sites, VLANs, subnets, hosts and what runs on them.
    Assets,
    /// What finds it: discovery runs and the daemons that perform them.
    Discover,
    /// What runs Scanopy across the organization: tags, users, keys and credentials.
    Platform,
    /// How it is drawn and shared: topologies, snapshots, shares and dependencies.
    Visualization,
}

impl HasId for EntityCategory {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for EntityCategory {
    fn color(&self) -> Color {
        Color::Gray
    }

    fn icon(&self) -> Icon {
        match self {
            Self::Assets => Icon::Server,
            Self::Discover => Icon::Radar,
            Self::Platform => Icon::Settings,
            Self::Visualization => Icon::Network,
        }
    }
}

impl TypeMetadataProvider for EntityCategory {
    fn name(&self) -> &'static str {
        match self {
            Self::Assets => "Assets",
            Self::Discover => "Discover",
            Self::Platform => "Platform",
            Self::Visualization => "Visualization",
        }
    }
}
