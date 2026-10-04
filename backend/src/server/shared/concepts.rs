use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumDiscriminants, EnumIter, IntoStaticStr};

use crate::server::shared::types::{
    Color, Icon,
    metadata::{EntityMetadataProvider, HasId},
};

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    EnumDiscriminants,
    EnumIter,
    IntoStaticStr,
    Serialize,
    Deserialize,
    Display,
)]
#[strum_discriminants(derive(Display, Hash, EnumIter, IntoStaticStr))]
pub enum Concept {
    Dns,
    Vpn,
    Gateway,
    ReverseProxy,
    IoT,
    Storage,
    Virtualization,
    Containerization,
    Workloads,
    SNMP,
    L2,
    L3,
    Application,
}

impl HasId for Concept {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for Concept {
    fn color(&self) -> Color {
        match self {
            Concept::Dns => Color::Emerald,
            Concept::Vpn => Color::Green,
            Concept::Gateway => Color::Teal,
            Concept::ReverseProxy => Color::Cyan,
            Concept::IoT => Color::Yellow,
            Concept::Storage => Color::Green,
            Concept::SNMP => Color::Pink,

            Concept::L2 => Color::Green,
            Concept::L3 => Color::Blue,

            Concept::Workloads => Color::Orange,
            Concept::Virtualization => Color::Amber,
            Concept::Containerization => Color::Blue,

            Concept::Application => Color::Purple,
        }
    }

    fn icon(&self) -> Icon {
        match self {
            Concept::Dns => Icon::Search,
            Concept::Vpn => Icon::VenetianMask,
            Concept::Gateway => Icon::Router,
            Concept::ReverseProxy => Icon::Split,
            Concept::IoT => Icon::CircuitBoard,
            Concept::Storage => Icon::HardDrive,
            Concept::SNMP => Icon::Activity,

            Concept::L2 => Icon::Cable,
            Concept::L3 => Icon::Signpost,

            Concept::Workloads => Icon::Boxes,
            Concept::Virtualization => Icon::MonitorCog,
            Concept::Containerization => Icon::Box,

            Concept::Application => Icon::Component,
        }
    }
}
