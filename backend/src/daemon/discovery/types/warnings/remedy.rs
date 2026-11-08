//! The rung a warning sits on: what it asks of the person reading it.

use serde::{Deserialize, Serialize};
use serde_json::json;
use strum::{EnumIter, IntoStaticStr, VariantNames};
use utoipa::ToSchema;

use crate::server::shared::types::{
    Color, Icon,
    metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
};

/// What a warning asks of the person reading it.
///
/// The rung a code sits on *is* its instruction, which is why the warning list is grouped by this
/// and not by [`Severity`]: a reader scanning a completed run needs to know whether they are being
/// asked for something before they need to know how much data was lost. It reaches the UI as each
/// code's `category`, with this enum's own registry supplying the heading — the same shape
/// `service-definitions.json` and `service-categories.json` use.
///
/// Declaration order is the order the sections render in, most demanding first.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    ToSchema,
    EnumIter,
    IntoStaticStr,
    VariantNames,
)]
pub enum WarningRemedy {
    /// A credential or a scan setting has to change, and both live in Scanopy.
    FixInScanopy,
    /// The device answered and served less than it claims to. Scanopy has no setting for it.
    CheckTheDevice,
    /// A read stopped before it finished. The values already held were kept.
    ClearsOnTheNextScan,
    /// True of the device, or of data that arrived malformed. No scan will change it.
    NothingToDo,
}

impl WarningRemedy {
    /// Whether a warning on this rung asks the reader to do something. The two rungs below it
    /// either resolve themselves or cannot be resolved, so a summary that counts them as work
    /// would raise an alarm nobody can answer.
    pub fn is_actionable(&self) -> bool {
        match self {
            Self::FixInScanopy | Self::CheckTheDevice => true,
            Self::ClearsOnTheNextScan | Self::NothingToDo => false,
        }
    }
}

impl HasId for WarningRemedy {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for WarningRemedy {
    /// Grey throughout, so the section heading cannot compete with the row it heads. Colour in
    /// this list means severity and only severity; a second coloured vocabulary stacked on top of
    /// it would leave an amber heading over a red row saying two things at once.
    fn color(&self) -> Color {
        Color::Gray
    }

    fn icon(&self) -> Icon {
        match self {
            Self::FixInScanopy => Icon::Wrench,
            Self::CheckTheDevice => Icon::ScanSearch,
            Self::ClearsOnTheNextScan => Icon::RefreshCw,
            Self::NothingToDo => Icon::Info,
        }
    }
}

impl TypeMetadataProvider for WarningRemedy {
    /// The section heading. An instruction, not a category label — it is the whole point of the
    /// grouping that a reader can act on the heading alone.
    fn name(&self) -> &'static str {
        match self {
            Self::FixInScanopy => "Fix in Scanopy",
            Self::CheckTheDevice => "Check the device",
            Self::ClearsOnTheNextScan => "Should clear on the next full scan",
            Self::NothingToDo => "Nothing to do",
        }
    }

    /// The sub-line under the heading, saying why everything below it shares a response.
    fn description(&self) -> &'static str {
        match self {
            Self::FixInScanopy => {
                "A credential or a scan setting has to change. The next scan reads what this one could not."
            }
            Self::CheckTheDevice => {
                "The device answered, but served less than it says it holds. Nothing in Scanopy changes that — the agent's SNMP view or VLAN context has to."
            }
            Self::ClearsOnTheNextScan => {
                "A read stopped before it finished. Previously discovered values were kept, and a complete scan should replace these."
            }
            Self::NothingToDo => {
                "True of the device, or of the data it returned. Rescanning will not change it."
            }
        }
    }

    /// Publishes [`WarningRemedy::is_actionable`], so the UI picks out the rungs that ask for work
    /// from the same answer the server's counts use.
    fn metadata(&self) -> serde_json::Value {
        json!({ "actionable": self.is_actionable() })
    }
}
