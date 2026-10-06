use std::borrow::Cow;

use chrono::{DateTime, Utc};
use email_address::EmailAddress;
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumIter};
use strum_macros::EnumDiscriminants;
use uuid::Uuid;

use crate::server::shared::events::traits::{
    EventFilter, EventScope, Operation, ScopeOrganization,
};
use crate::server::shared::events::types::EventLogLevel;

/// Compact summary of a host for digest rendering. Keeps the payload small
/// enough to round-trip through the event bus without serializing entire
/// entity rows.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostSummary {
    pub id: Uuid,
    /// Hostname when discovery resolved one (SNMP `sysName.0`, reverse DNS,
    /// or user-assigned); falls back to the synthetic `name`.
    pub label: String,
}

/// Status for any `DiscoveryTracked` entity in the digest (hosts AND their
/// children) is [`EntityFreshness`] — the same derived, time-based verdict the
/// inventory and topology render, so a host reported stale here is the same host
/// badged stale in the app.
///
/// Status is encoded visually with a glyph or badge — never by recolouring the
/// entity. Colour stays bound to the entity type per
/// `EntityDiscriminants::color()`.
pub use crate::server::shared::types::entities::EntityFreshness;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PortSummary {
    pub id: Uuid,
    pub host_id: Uuid,
    /// Human-readable port-type label, e.g. `"22/tcp"` or `"Ssh"`. Never
    /// includes UUIDs.
    pub label: String,
    #[serde(default)]
    pub status: EntityFreshness,
    /// True when `status` was acquired this scan (a transition just
    /// happened). Stably-stale entities have `is_fresh = false` and
    /// don't trigger the host card's inclusion in the digest.
    #[serde(default)]
    pub is_fresh: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceSummary {
    pub id: Uuid,
    pub host_id: Uuid,
    pub name: String,
    /// True when the service is a container (Docker virtualization, etc).
    /// The renderer splits these into a separate "Containers" row.
    #[serde(default)]
    pub is_container: bool,
    /// Raw value from `ServiceDefinition::logo_url()`. The renderer
    /// rewrites relative `/logos/...` paths to absolute using the email
    /// service's `public_url`. `None` when the service has no logo.
    #[serde(default)]
    pub logo_url: Option<String>,
    #[serde(default)]
    pub status: EntityFreshness,
    #[serde(default)]
    pub is_fresh: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IpAddressSummary {
    pub id: Uuid,
    pub host_id: Uuid,
    pub address: String,
    #[serde(default)]
    pub status: EntityFreshness,
    #[serde(default)]
    pub is_fresh: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InterfaceSummary {
    pub id: Uuid,
    pub host_id: Uuid,
    pub label: String,
    #[serde(default)]
    pub status: EntityFreshness,
    #[serde(default)]
    pub is_fresh: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubnetSummary {
    pub id: Uuid,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VlanSummary {
    pub id: Uuid,
    pub vlan_number: u16,
    pub name: String,
}

/// Rich host representation for the digest email — mirrors the UI's
/// `HostCard.svelte` so a recipient sees the same shape they'd see in-app.
/// `status` uses the same `EntityFreshness` enum as its children, so the
/// digest's vocabulary stays consistent. A host that's listed because its
/// CHILDREN changed (rather than the host itself) carries `Current`; the
/// surrounding section header ("Hosts with changes") provides the context
/// and the host's badge is hidden in that case. Children reflect live state
/// at `finished_at`. Bindings are intentionally not included — they're the
/// service↔port↔IP join that the other rows already cover.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AffectedHostCard {
    pub host: HostSummary,
    pub status: EntityFreshness,
    pub services: Vec<ServiceSummary>,
    pub ip_addresses: Vec<IpAddressSummary>,
    pub interfaces: Vec<InterfaceSummary>,
    pub ports: Vec<PortSummary>,
}

/// Lightweight recipient identity. We carry just what the email subscriber
/// needs to dispatch — id (for the per-user discovery_digest gate), email,
/// and the in-memory `EmailSettings` snapshot so the gate is decided without
/// re-fetching the user.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DigestRecipient {
    pub user_id: Uuid,
    #[serde(with = "email_address_serde")]
    pub email: EmailAddress,
    pub discovery_digest_enabled: bool,
}

mod email_address_serde {
    use email_address::EmailAddress;
    use serde::{Deserialize, Deserializer, Serializer, de::Error};
    use std::str::FromStr;

    pub fn serialize<S: Serializer>(addr: &EmailAddress, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(addr.as_ref())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<EmailAddress, D::Error> {
        let s = String::deserialize(de)?;
        EmailAddress::from_str(&s).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscoveryDigestPayload {
    pub session_id: Uuid,
    pub site_id: Uuid,
    pub site_name: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    /// The site's *effective* staleness window in hours (its configured
    /// value, or the default). Carried so the email can state what "stale"
    /// meant for this site instead of asserting it without a definition —
    /// the threshold is per-site, so the legend cannot be static.
    pub stale_after_hours: i64,

    pub subnets_scanned: Vec<SubnetSummary>,
    pub hosts_added: Vec<AffectedHostCard>,
    /// Hosts that crossed into `Stale` during this session. Named for the
    /// claim we actually make — "not observed within this site's window" —
    /// rather than "vanished", which asserted a removal we cannot detect.
    pub hosts_stale: Vec<AffectedHostCard>,
    pub hosts_changed: Vec<AffectedHostCard>,
    pub vlans_added: Vec<VlanSummary>,
    pub vlans_stale: Vec<VlanSummary>,

    pub recipients: Vec<DigestRecipient>,
}

impl DiscoveryDigestPayload {
    /// Whether the digest has any user-visible deltas worth emailing about.
    /// Subnets-scanned alone is metadata, not a change — empty digests are
    /// suppressed regardless of recipient settings.
    pub fn has_changes(&self) -> bool {
        !self.hosts_added.is_empty()
            || !self.hosts_stale.is_empty()
            || !self.hosts_changed.is_empty()
            || !self.vlans_added.is_empty()
            || !self.vlans_stale.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, strum::Display, EnumDiscriminants)]
#[serde(tag = "type")]
#[strum(serialize_all = "snake_case")]
#[strum_discriminants(derive(Hash, EnumIter, strum::Display, Serialize, Deserialize, AsRefStr))]
pub enum DiscoveryDigestOperation {
    Computed {
        payload: Box<DiscoveryDigestPayload>,
    },
}

/// Org scope mirrors the existing `OrgScope` used by Billing/Onboarding —
/// digests are per-org-keyed for routing; the site_id rides on the payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DiscoveryDigestScope {
    pub organization_id: Uuid,
    pub site_id: Uuid,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DiscoveryDigestFlags {
    pub suppress_logs: bool,
}

impl EventScope for DiscoveryDigestScope {
    fn organization(&self) -> Option<ScopeOrganization> {
        Some(ScopeOrganization::Org(self.organization_id))
    }
}

/// What leaves the server about a digest: how much changed, never which hosts, subnets or
/// recipients.
#[derive(Serialize)]
struct DiscoveryDigestMetadata {
    session_id: Uuid,
    site_id: Uuid,
    subnets_scanned: usize,
    hosts_added: usize,
    hosts_stale: usize,
    hosts_changed: usize,
    vlans_added: usize,
    vlans_stale: usize,
    recipients: usize,
}

impl Operation for DiscoveryDigestOperation {
    type Scope = DiscoveryDigestScope;
    type Flags = DiscoveryDigestFlags;
    type Filter = EventFilter<DiscoveryDigestOperation>;

    fn log_level(&self) -> EventLogLevel {
        EventLogLevel::Info
    }

    fn event_name(&self, _scope: &DiscoveryDigestScope) -> Cow<'static, str> {
        format!("discovery_digest_{self}").into()
    }

    fn metadata<'a>(&'a self, _scope: &'a DiscoveryDigestScope) -> impl Serialize + Send + 'a {
        let Self::Computed { payload } = self;
        DiscoveryDigestMetadata {
            session_id: payload.session_id,
            site_id: payload.site_id,
            subnets_scanned: payload.subnets_scanned.len(),
            hosts_added: payload.hosts_added.len(),
            hosts_stale: payload.hosts_stale.len(),
            hosts_changed: payload.hosts_changed.len(),
            vlans_added: payload.vlans_added.len(),
            vlans_stale: payload.vlans_stale.len(),
            recipients: payload.recipients.len(),
        }
    }
}
