//! The event a corrected subnet range is.
//!
//! A range Scanopy inferred from LLDP neighbour addresses is a guess, and the first thing that
//! actually reads a netmask for that segment — a daemon's own interface, a device's
//! `ipAdEntNetMask` — settles it. That correction rewrites a row nobody asked it to touch, so it
//! has to say so out loud.
//!
//! It is its own operation rather than something hung off [`DiscoveryWarningCode`] for the same
//! reason that one exists: the scope would not fit. `DiscoveryWarningScope` requires a `session_id`
//! and a `daemon_id` as identity dimensions, and the path this fires on most often has neither — a
//! daemon's interfaced subnets arrive through `process_status`, which is not a scan. Forcing them
//! optional would put a payload on an identity scope and weaken every metric label that reads it.
//!
//! Publishing *is* the log. `LoggingService` subscribes to every operation type and renders one
//! line at the operation's declared level, so there is no `tracing::warn!` beside this — that
//! duplication is exactly what the warning events replaced. Nothing else subscribes today; metrics,
//! analytics or a subscriber that appends to a scan record can, without this code changing.

use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use strum::IntoDiscriminant;
use strum_macros::{AsRefStr, EnumDiscriminants};
use uuid::Uuid;

use crate::server::shared::attribution::AttributeSource;
use crate::server::shared::events::EventFlags;
use crate::server::shared::events::registry::to_snake_case;
use crate::server::shared::events::traits::{
    EventFilter, EventScope, Operation, ScopeOrganization,
};
use crate::server::shared::events::types::EventLogLevel;

/// What a reading did to a range that had been inferred.
///
/// The three are told apart because they mean different things to whoever reads the log. A promotion
/// says the guess was right. A widening says the segment was bigger than assumed. A narrowing says
/// the guess was too wide — and it is the only one that moves addresses, because the corrected range
/// no longer covers everything filed in it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, EnumDiscriminants)]
#[strum_discriminants(derive(Hash, AsRefStr, Serialize, Deserialize))]
pub enum SubnetCorrection {
    /// The reading agreed with the assumed range exactly; only the confidence changed.
    Promoted,
    /// The segment turned out larger than assumed. Nothing is displaced by widening.
    Widened,
    /// The segment turned out smaller than assumed, so the widening that produced it was wrong.
    Narrowed {
        /// Addresses the corrected range no longer covers, sent back through placement.
        addresses_replaced: usize,
    },
}

/// Which range was corrected, and from what to what.
///
/// The before/after pair rides on the scope rather than being left to the reader to reconstruct:
/// once the row is written, nothing records what it used to be — subnet updates are in place, not
/// versioned — so if this line does not carry the old range, nothing does.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SubnetCorrectionScope {
    pub site_id: Uuid,
    pub subnet_id: Uuid,
    pub from_cidr: String,
    pub to_cidr: String,
    pub from_source: AttributeSource,
    pub to_source: AttributeSource,
}

impl Operation for SubnetCorrection {
    type Scope = SubnetCorrectionScope;
    type Flags = EventFlags;
    type Filter = EventFilter<SubnetCorrection>;

    fn log_level(&self) -> EventLogLevel {
        // Nothing is broken — a guess was replaced by a reading, which is the system working. It is
        // `Warn` rather than `Info` because it changed a row on its own initiative and an operator
        // who sees a range move should be able to find out why without turning up the log level.
        EventLogLevel::Warn
    }

    /// `subnet_promoted`, `subnet_widened` or `subnet_narrowed`: the three mean different things
    /// to whoever reads the log.
    fn event_name(&self, _scope: &SubnetCorrectionScope) -> Cow<'static, str> {
        format!("subnet_{}", to_snake_case(self.discriminant().as_ref())).into()
    }

    fn metadata<'a>(&'a self, scope: &'a SubnetCorrectionScope) -> impl Serialize + Send + 'a {
        SubnetCorrectionMetadata {
            site_id: scope.site_id,
            subnet_id: scope.subnet_id,
            from_source: &scope.from_source,
            to_source: &scope.to_source,
            correction: self,
        }
    }
}

impl EventScope for SubnetCorrectionScope {
    fn organization(&self) -> Option<ScopeOrganization> {
        Some(ScopeOrganization::Site(self.site_id))
    }
}

/// What leaves the server about a correction. The ranges stay behind: they map a customer's
/// network.
#[derive(Serialize)]
struct SubnetCorrectionMetadata<'a> {
    site_id: Uuid,
    subnet_id: Uuid,
    from_source: &'a AttributeSource,
    to_source: &'a AttributeSource,
    correction: &'a SubnetCorrection,
}
