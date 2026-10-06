//! The event a discovery warning is, once it reaches the server.
//!
//! Warnings arrive from two producers that cannot share a carrier any other way: the daemon posts
//! them on the terminal payload, and LLDP/CDP resolution appends its own *after* that payload has
//! been written and its `DiscoveryPhase` event published. Hanging codes off `DiscoveryScope` would
//! therefore have counted the first producer and silently missed the second — and it would have
//! put a payload list on an identity scope.
//!
//! So the warning is its own operation, published once per occurrence by both producers, and the
//! metrics, analytics and logging subscribers each read it once. That is a fold rather than a
//! chain: nothing here republishes a fact to inform some other operation's subscriber.
//!
//! The code *is* the operation, which is what lets a subscriber filter by code, the metric label by
//! it directly, and the log line name itself after it.

use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::daemon::discovery::types::warnings::{DiscoveryWarning, DiscoveryWarningCode};
use crate::server::credentials::r#impl::mapping::CredentialQueryPayloadDiscriminants;
use crate::server::shared::events::EventFlags;
use crate::server::shared::events::traits::{
    EventFilter, EventScope, Operation, ScopeOrganization,
};
use crate::server::shared::events::types::EventLogLevel;

/// Which run a warning came from, and which integration produced it.
///
/// `integration` is on the scope rather than in the operation because it is an identity dimension:
/// it is the metric's second label, and the bus's whole shape puts those on the scope. `warning`
/// rides along so the log line carries the occurrence's own evidence — the address, the counts, the
/// library's diagnostic — which is the thing that used to exist only in a `tracing::warn!` the
/// operator could not read.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DiscoveryWarningScope {
    pub site_id: Uuid,
    pub session_id: Uuid,
    pub daemon_id: Uuid,
    /// `None` for the scan-level and link-resolution findings, which belong to the pipeline rather
    /// than to any one integration. It becomes the metric's `none` label rather than being dropped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integration: Option<CredentialQueryPayloadDiscriminants>,
    pub warning: DiscoveryWarning,
}

impl EventScope for DiscoveryWarningScope {
    fn organization(&self) -> Option<ScopeOrganization> {
        Some(ScopeOrganization::Site(self.site_id))
    }
}

/// What leaves the server about a warning: which run, which failure mode, whose integration.
/// Nothing that identifies a customer's network: no address, no host id, and not the library
/// diagnostic on `detail`.
#[derive(Serialize)]
struct DiscoveryWarningMetadata {
    session_id: Uuid,
    site_id: Uuid,
    daemon_id: Uuid,
    code: DiscoveryWarningCode,
    /// `"none"` for scan-level findings, matching the metric label.
    integration: String,
}

impl Operation for DiscoveryWarningCode {
    type Scope = DiscoveryWarningScope;
    type Flags = EventFlags;
    type Filter = EventFilter<DiscoveryWarningCode>;

    fn log_level(&self) -> EventLogLevel {
        // Every one of these is a non-fatal finding by construction — a fatal one sets `error` on
        // the payload and fails the run instead.
        EventLogLevel::Warn
    }

    fn event_name(&self, _scope: &DiscoveryWarningScope) -> Cow<'static, str> {
        "discovery_warning".into()
    }

    /// The code, so the log line names the failure mode itself.
    fn log_label(&self, _scope: &DiscoveryWarningScope) -> String {
        self.to_string()
    }

    fn metadata<'a>(&'a self, scope: &'a DiscoveryWarningScope) -> impl Serialize + Send + 'a {
        DiscoveryWarningMetadata {
            session_id: scope.session_id,
            site_id: scope.site_id,
            daemon_id: scope.daemon_id,
            code: *self,
            integration: scope
                .integration
                .map_or_else(|| "none".to_string(), |i| i.to_string()),
        }
    }
}

impl DiscoveryWarningScope {
    /// The scope for one warning from one session.
    pub fn new(
        site_id: Uuid,
        session_id: Uuid,
        daemon_id: Uuid,
        warning: DiscoveryWarning,
    ) -> Self {
        Self {
            site_id,
            session_id,
            daemon_id,
            integration: warning.integration(),
            warning,
        }
    }
}
