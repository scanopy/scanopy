use crate::server::services::r#impl::patterns::MatchDetails;
use crate::server::shared::types::metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider};
use crate::server::shared::types::{Color, Icon};
use serde::{Deserialize, Serialize};
use strum::IntoDiscriminant;
use strum_macros::{EnumDiscriminants, EnumIter, IntoStaticStr, VariantNames};
use utoipa::ToSchema;

/// How recently discovery last observed an entity.
///
/// Derived, never persisted — computed from `last_seen_at` against the
/// entity's network staleness window (`Network::stale_cutoff`). Shared by the
/// discovery digest email and the UI so a host reported stale in the digest is
/// the same host badged stale in the inventory and topology; running two
/// different measures let them disagree (a scan-count measure calls an entity
/// missing after 3 scans, which is 45 minutes on one network and 3 months on
/// another).
///
/// Only discovery-managed entities can be `Stale` — see
/// [`DiscoveryTracked::is_discovery_managed`](crate::server::shared::storage::snapshot::DiscoveryTracked::is_discovery_managed).
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    ToSchema,
    strum_macros::EnumIter,
    strum_macros::IntoStaticStr,
)]
#[serde(rename_all = "snake_case")]
// Matches the serde representation so the filter-value id, the API value and
// the frontend's generated union are all the same string.
#[strum(serialize_all = "snake_case")]
pub enum EntityFreshness {
    /// First observed during the scan window being reported on. Only the
    /// digest distinguishes this; the inventory surfaces `created_at` directly.
    New,
    /// Observed within the network's staleness window, or not discovery-managed.
    #[default]
    Current,
    /// Discovery-managed and not observed within the network's staleness
    /// window. Asserts only "not seen recently" — never "removed".
    Stale,
}

impl HasId for EntityFreshness {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for EntityFreshness {
    fn color(&self) -> Color {
        match self {
            // Amber, not red: stale means behind, not broken — the same split
            // the daemon status tags use.
            Self::Stale => Color::Amber,
            Self::Current => Color::Green,
            Self::New => Color::Blue,
        }
    }

    fn icon(&self) -> Icon {
        match self {
            Self::Stale => Icon::Clock,
            Self::Current => Icon::Check,
            Self::New => Icon::Plus,
        }
    }
}

impl TypeMetadataProvider for EntityFreshness {
    fn name(&self) -> &'static str {
        match self {
            Self::New => "New",
            Self::Current => "Current",
            Self::Stale => "Stale",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::New => "First observed during the scan being reported on",
            Self::Current => "Observed within this network's staleness window",
            Self::Stale => "Not observed within this network's staleness window",
        }
    }
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    Default,
    Eq,
    PartialEq,
    Hash,
    EnumDiscriminants,
    VariantNames,
    ToSchema,
)]
// `IntoStaticStr` with no `serialize_all` yields the variant name, which is also the serde tag:
// the fixture id, the `sources` filter value and `source.type` on the wire are one string.
#[strum_discriminants(derive(Hash, IntoStaticStr, EnumIter, Serialize, Deserialize, ToSchema))]
#[serde(tag = "type")]
pub enum EntitySource {
    #[schema(title = "Manual")]
    Manual,
    #[default]
    #[schema(title = "System")]
    System,
    #[schema(title = "Discovery")]
    Discovery,
    #[schema(title = "DiscoveryWithMatch")]
    DiscoveryWithMatch { details: MatchDetails },
    /// Known only because something else reported it — an LLDP/CDP neighbour publishing an address,
    /// or a controller listing a device — and never contacted directly.
    ///
    /// A distinct rung rather than plain `Discovery` because the difference is exactly what an
    /// operator cannot otherwise see: a host with no ports, no services and an address nothing ever
    /// answered on is indistinguishable from a device that is simply down. It counts as discovered
    /// for every other purpose — see [`Self::is_from_discovery`] — so staleness, quota and the
    /// discovery FKs behave for it precisely as they do for a swept host.
    #[schema(title = "Inferred")]
    Inferred,
    // `other` makes an EntitySource variant a newer server adds degrade to
    // `Unknown` on an older daemon instead of failing the whole response.
    // EntitySource is embedded as `.source` on nearly every entity, so this is
    // the highest-blast-radius forward-compat guard.
    #[schema(title = "Unknown")]
    #[serde(other)]
    Unknown,
}

impl EntitySource {
    /// Returns true if this entity was created via discovery (network, Docker, etc.)
    pub fn is_from_discovery(&self) -> bool {
        self.discriminant().is_from_discovery()
    }
}

impl EntitySourceDiscriminants {
    /// The discriminant form of [`EntitySource::is_from_discovery`], for SQL predicates that
    /// compare the stored `type` tag and so have no `EntitySource` value to ask.
    pub fn is_from_discovery(&self) -> bool {
        matches!(
            self,
            Self::Discovery | Self::DiscoveryWithMatch | Self::Inferred
        )
    }
}

impl HasId for EntitySourceDiscriminants {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for EntitySourceDiscriminants {
    fn color(&self) -> Color {
        match self {
            Self::Discovery | Self::DiscoveryWithMatch => Color::Blue,
            // Neither amber (stale, `getFreshnessTag`) nor indigo (an assumed attribute,
            // `AttributeMethod::Inferred`): nothing is behind and no value is being guessed. The
            // host exists; nothing has looked at it yet.
            Self::Inferred => Color::Violet,
            Self::Manual => Color::Green,
            Self::System | Self::Unknown => Color::Gray,
        }
    }

    fn icon(&self) -> Icon {
        match self {
            Self::Discovery | Self::DiscoveryWithMatch => Icon::Radar,
            Self::Inferred => Icon::Waypoints,
            Self::Manual => Icon::User,
            Self::System => Icon::Settings,
            Self::Unknown => Icon::CircleQuestionMark,
        }
    }
}

impl TypeMetadataProvider for EntitySourceDiscriminants {
    fn name(&self) -> &'static str {
        match self {
            Self::Manual => "Manual",
            Self::System => "System",
            Self::Discovery => "Scanned",
            Self::DiscoveryWithMatch => "Scanned and matched",
            Self::Inferred => "Inferred",
            Self::Unknown => "Unknown",
        }
    }

    /// Read where an operator meets an entity they did not expect: the chip tooltip on the
    /// Hosts tab and the notice on an inferred host. So each one says what put the entity there
    /// and, where it matters, what to do next.
    fn description(&self) -> &'static str {
        match self {
            Self::Manual => "Created by a person in Scanopy.",
            Self::System => "Created by Scanopy itself.",
            Self::Discovery => "Found by a scan that contacted it directly.",
            Self::DiscoveryWithMatch => {
                "Found by a scan that contacted it directly, and matched to a known definition."
            }
            Self::Inferred => {
                "A neighbouring device advertised this over LLDP or CDP, but no scan has contacted it."
            }
            Self::Unknown => "Created by a newer version of Scanopy than this one.",
        }
    }
}

// DiscoveryMetadata removed — its only consumer was a list of "last 5
// discoveries" that has been superseded by the typed
// last_discovery_id / first_discovery_id / last_seen_at columns on
// DiscoveryTracked entities. The metadata-strip migration (20260502000007)
// removes the field from existing source JSONB on disk.

#[cfg(test)]
mod forward_compat_tests {
    use super::*;
    use serde::Deserialize;

    #[test]
    fn unknown_variant_degrades_to_unknown() {
        // A source kind a newer server adds degrades to `Unknown` on an older
        // daemon instead of failing the entire response.
        let parsed: EntitySource =
            serde_json::from_value(serde_json::json!({ "type": "FutureSource" })).unwrap();
        assert_eq!(parsed, EntitySource::Unknown);
    }

    #[test]
    fn tolerates_legacy_metadata_field() {
        // Reverse-direction skew: a NEW daemon reading an OLD server's payload
        // that still carries the removed `metadata` field — extra field ignored.
        let parsed: EntitySource =
            serde_json::from_value(serde_json::json!({ "type": "Discovery", "metadata": [] }))
                .unwrap();
        assert_eq!(parsed, EntitySource::Discovery);
    }

    #[test]
    fn known_variants_round_trip() {
        for variant in [
            EntitySource::Manual,
            EntitySource::System,
            EntitySource::Discovery,
            EntitySource::Unknown,
        ] {
            let json = serde_json::to_value(&variant).unwrap();
            let back: EntitySource = serde_json::from_value(json).unwrap();
            assert_eq!(variant, back);
        }
    }

    /// The `sources` filter binds `discriminant().id()` and compares it to the stored
    /// `source->>'type'`, and the UI keys the fixture on `source.type`. All three hold only while
    /// the strum name and the serde tag agree for every variant.
    #[test]
    fn discriminant_id_is_the_serde_tag() {
        use strum::IntoEnumIterator;
        for discriminant in EntitySourceDiscriminants::iter() {
            let source = match discriminant {
                EntitySourceDiscriminants::Manual => EntitySource::Manual,
                EntitySourceDiscriminants::System => EntitySource::System,
                EntitySourceDiscriminants::Discovery => EntitySource::Discovery,
                EntitySourceDiscriminants::DiscoveryWithMatch => EntitySource::DiscoveryWithMatch {
                    details: MatchDetails::new_certain("test"),
                },
                EntitySourceDiscriminants::Inferred => EntitySource::Inferred,
                EntitySourceDiscriminants::Unknown => EntitySource::Unknown,
            };
            let json = serde_json::to_value(&source).unwrap();
            assert_eq!(json["type"], discriminant.id(), "{discriminant:?}");
        }
    }

    #[test]
    fn historical_required_metadata_failure_is_reproduced() {
        // Documents the original production failure: the pre-`ed235fa28`
        // EntitySource required `metadata` on `Discovery`, so the new server's
        // `{"type":"Discovery"}` could not be deserialized by an old daemon.
        #[derive(Deserialize)]
        #[serde(tag = "type")]
        enum OldEntitySource {
            #[allow(dead_code)]
            Discovery { metadata: Vec<serde_json::Value> },
        }
        let result: Result<OldEntitySource, _> =
            serde_json::from_value(serde_json::json!({ "type": "Discovery" }));
        let Err(err) = result else {
            panic!("expected old EntitySource to require `metadata`");
        };
        let err = err.to_string();
        assert!(err.contains("missing field `metadata`"), "got: {err}");
    }
}
