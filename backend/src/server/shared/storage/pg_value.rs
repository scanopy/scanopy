//! The wire shapes a column value can take, and the two that are sanitised.
//!
//! PostgreSQL cannot represent `U+0000` in a `text` column (SQLSTATE 22021) or inside a `jsonb`
//! value (22P05). Network hardware emits it anyway — D-Link's DGS series NUL-terminates its LLDP
//! port identifiers, so `lldpRemPortId` arrives as `31 00`, a perfectly valid UTF-8 `"1\0"`
//! (GH #668). One such byte on one port used to abort the insert of the entire host, because an
//! encoding error is not a unique violation and so was classified as a systemic failure rather
//! than a bad value.
//!
//! Nothing below us can fix this. PostgreSQL has no representation for the character, and the
//! Rust Postgres drivers deliberately pass bytes through — Diesel declined to sanitise
//! (diesel-rs/diesel#284) on the grounds that it would burden every string serialization. So the
//! invariant has to be ours, and it has to hold for *every* source: SNMP is only the one that
//! caught us, and UniFi controller JSON, banner grabs, reverse DNS and manual API input all reach
//! the same columns by different routes.
//!
//! # Why these are newtypes
//!
//! [`PgText`] and [`PgJson`] have private fields and no public constructor other than one that
//! strips. A value of either type has therefore been sanitised *by construction* — there is no
//! path that produces one without going through [`strip_nuls`].
//!
//! [`Bound`] then carries `PgText`/`PgJson` rather than a bare `&str`/`Value` for exactly the
//! variants that reach a `text` or `jsonb` column. Since `SqlValue::to_bound` returns a `Bound`
//! and never touches the sqlx `Query`, reaching one of those columns without sanitising would
//! require adding a *new variant to `Bound`* — a deliberate edit to this file, not something that
//! happens by accident while adding a column somewhere else. Rust cannot make enum variants
//! private, which is why the payloads are newtypes rather than raw values.
//!
//! This mirrors the enforcement already used for the DB-enum baseline in `traits.rs`: make the
//! compiler refuse the shortcut rather than trusting the next author to remember.

use std::borrow::Cow;

use chrono::{DateTime, Utc};
use ipnetwork::IpNetwork;
use mac_address::MacAddress;
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::server::shared::storage::traits::SqlValue;

/// Remove every `U+0000` from `s`, borrowing unchanged when there are none.
///
/// Stripping rather than rejecting: PostgreSQL has no representation for the character, so there
/// is no faithful-store option, and rejecting the value loses the interface — or, before the
/// error was reclassified, the whole device. Nothing an operator wanted is in a NUL.
///
/// The clean path is the overwhelmingly common one and does not allocate: `str::contains` over a
/// single `char` is a `memchr` scan, which is why this is affordable on every text column of
/// every row.
pub fn strip_nuls(s: &str) -> Cow<'_, str> {
    if s.contains('\0') {
        Cow::Owned(s.replace('\0', ""))
    } else {
        Cow::Borrowed(s)
    }
}

/// A string that can be stored in a PostgreSQL `text` column.
///
/// The fields are private and both constructors strip, so holding one of these *is* the guarantee.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgText<'a> {
    value: Cow<'a, str>,
    stripped: bool,
}

impl<'a> PgText<'a> {
    /// Borrow `s` when it is already storable, allocating only to strip.
    pub fn new(s: &'a str) -> Self {
        let value = strip_nuls(s);
        let stripped = matches!(value, Cow::Owned(_));
        Self { value, stripped }
    }

    /// For callers that already own the string — the `serde_json::to_string` arms, which
    /// serialize into a fresh `String` there is no point borrowing back out of.
    pub fn owned(s: String) -> Self {
        match strip_nuls(&s) {
            // Nothing to strip: keep the original allocation rather than the copy `strip_nuls`
            // would otherwise have to make.
            Cow::Borrowed(_) => Self {
                value: Cow::Owned(s),
                stripped: false,
            },
            Cow::Owned(cleaned) => Self {
                value: Cow::Owned(cleaned),
                stripped: true,
            },
        }
    }

    pub(super) fn into_inner(self) -> Cow<'a, str> {
        self.value
    }
}

/// A JSON value that can be stored in a PostgreSQL `jsonb` column.
///
/// Object *keys* are sanitised as well as string values — `jsonb` rejects the escape wherever it
/// appears, and a key is the easier one to forget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgJson {
    value: JsonValue,
    stripped: bool,
}

impl PgJson {
    pub fn new(value: JsonValue) -> Self {
        let mut stripped = false;
        let value = clean_json(value, &mut stripped);
        Self { value, stripped }
    }

    pub(super) fn into_inner(self) -> JsonValue {
        self.value
    }
}

/// Walk a `Value`, stripping NULs from every string and every object key.
fn clean_json(value: JsonValue, stripped: &mut bool) -> JsonValue {
    match value {
        JsonValue::String(s) => JsonValue::String(clean_string(s, stripped)),
        JsonValue::Array(items) => JsonValue::Array(
            items
                .into_iter()
                .map(|item| clean_json(item, stripped))
                .collect(),
        ),
        JsonValue::Object(map) => JsonValue::Object(
            map.into_iter()
                .map(|(key, val)| (clean_string(key, stripped), clean_json(val, stripped)))
                .collect(),
        ),
        // Null, Bool and Number cannot carry the character.
        other => other,
    }
}

fn clean_string(s: String, stripped: &mut bool) -> String {
    match strip_nuls(&s) {
        Cow::Borrowed(_) => s,
        Cow::Owned(cleaned) => {
            *stripped = true;
            cleaned
        }
    }
}

/// One bound parameter, as the shape PostgreSQL receives rather than the domain type it came from.
///
/// Collapsing ~60 `SqlValue` variants onto these makes the mapping explicit in one place instead
/// of restated per variant, and — the point of the exercise — leaves `Text`/`OptText`/`TextArray`/
/// `Json`/`OptJson` as the only routes to a `text` or `jsonb` column, each of which can only be
/// built from an already-sanitised newtype.
#[derive(Debug)]
pub enum Bound<'q> {
    Text(PgText<'q>),
    /// SQL `NULL` when `None` — distinct from `Json(Value::Null)`, which is a `jsonb` null.
    OptText(Option<PgText<'q>>),
    TextArray(Vec<PgText<'q>>),
    OptTextArray(Option<Vec<PgText<'q>>>),
    Json(PgJson),
    /// SQL `NULL` when `None`. Several columns rely on the distinction from `Json`, whose `None`
    /// serializes to a `jsonb` `null` instead — preserved from the original per-arm behaviour.
    OptJson(Option<PgJson>),
    Uuid(&'q Uuid),
    OptUuid(&'q Option<Uuid>),
    UuidArray(Vec<Uuid>),
    OptUuidArray(Option<Vec<Uuid>>),
    I32(i32),
    OptI32(&'q Option<i32>),
    I64(i64),
    OptI64(&'q Option<i64>),
    Bool(&'q bool),
    Timestamp(&'q DateTime<Utc>),
    OptTimestamp(&'q Option<DateTime<Utc>>),
    IpNet(IpNetwork),
    OptIpNet(Option<IpNetwork>),
    Mac(MacAddress),
    OptMac(Option<MacAddress>),
    MacArray(Vec<MacAddress>),
}

impl Bound<'_> {
    /// Whether sanitising this value actually removed anything.
    ///
    /// Drives one `warn!` at the bind site carrying the table name. A strip is never routine: it
    /// means an ingestion path handed us something PostgreSQL cannot store, and the useful signal
    /// is *which table*, so the source can be found and fixed rather than quietly compensated for
    /// here forever.
    pub fn stripped(&self) -> bool {
        match self {
            Self::Text(t) => t.stripped,
            Self::OptText(t) => t.as_ref().is_some_and(|t| t.stripped),
            Self::TextArray(items) => items.iter().any(|t| t.stripped),
            Self::OptTextArray(items) => items
                .as_ref()
                .is_some_and(|items| items.iter().any(|t| t.stripped)),
            Self::Json(j) => j.stripped,
            Self::OptJson(j) => j.as_ref().is_some_and(|j| j.stripped),
            _ => false,
        }
    }
}

impl SqlValue {
    /// Map a domain value onto the shape PostgreSQL receives.
    ///
    /// Deliberately has no access to the sqlx `Query`: that is what makes [`Bound`]'s sanitised
    /// text and JSON variants the only way to reach a `text`/`jsonb` column. The match is
    /// exhaustive, so a new `SqlValue` variant fails the build until it declares its shape here.
    pub fn to_bound(&self) -> Result<Bound<'_>, anyhow::Error> {
        Ok(match self {
            // ── text ──────────────────────────────────────────────────────────
            Self::String(v) => Bound::Text(PgText::new(v)),
            Self::OptionalString(v) => Bound::OptText(v.as_deref().map(PgText::new)),
            Self::Email(v) => Bound::Text(PgText::new(v.as_str())),
            Self::UserOrgPermissions(v) => Bound::Text(PgText::new(v.as_str())),
            Self::EdgeStyle(v) => Bound::Text(PgText::owned(v.to_string())),
            Self::StringArray(v) => Bound::TextArray(v.iter().map(|s| PgText::new(s)).collect()),
            Self::OptionalStringArray(v) => Bound::OptTextArray(
                v.as_ref()
                    .map(|items| items.iter().map(|s| PgText::new(s)).collect()),
            ),
            // JSON serialized to *text*, not jsonb — these columns are varchar/text.
            Self::IpCidr(v) => Bound::Text(PgText::owned(serde_json::to_string(v)?)),
            Self::ServiceDefinition(v) => Bound::Text(PgText::owned(serde_json::to_string(v)?)),
            Self::DaemonMode(v) => Bound::Text(PgText::owned(serde_json::to_string(v)?)),
            Self::OptionalOsFamily(v) => Bound::OptText(
                v.as_ref()
                    .map(serde_json::to_string)
                    .transpose()?
                    .map(PgText::owned),
            ),
            Self::OptionalDaemonOs(v) => Bound::OptText(
                v.as_ref()
                    .map(serde_json::to_string)
                    .transpose()?
                    .map(PgText::owned),
            ),
            Self::OptionBillingPlanStatus(v) => {
                Bound::Text(PgText::owned(serde_json::to_string(v)?))
            }
            Self::EntityDiscriminant(v) => Bound::Text(PgText::owned(serde_json::to_string(v)?)),

            // ── jsonb ─────────────────────────────────────────────────────────
            //
            // The `Option`-typed values below split into two groups on purpose, and the split is
            // load-bearing: `Json(to_value(&Option<_>))` writes a jsonb `null` for `None`, while
            // `OptJson(None)` writes a SQL `NULL`. Both shapes existed before this refactor and
            // columns depend on which one they get, so each arm keeps the one it had.
            Self::EntitySource(v) => Bound::Json(PgJson::new(serde_json::to_value(v)?)),
            // jsonb rather than text because two of the variants carry a `ClientProbe`, which a bare
            // string could not hold without a format both the migrations and the frontend would have
            // to know. The other thirty-three do write a bare string — externally tagged, so a
            // variant with nothing to carry has no tag — and jsonb holds that as happily.
            Self::AttributeSource(v) => Bound::Json(PgJson::new(serde_json::to_value(v)?)),
            Self::OptionalServiceVirtualization(v) => {
                Bound::Json(PgJson::new(serde_json::to_value(v)?))
            }
            Self::OptionalHostVirtualization(v) => {
                Bound::Json(PgJson::new(serde_json::to_value(v)?))
            }
            Self::IPAddresses(v) => Bound::Json(PgJson::new(serde_json::to_value(v)?)),
            Self::Ports(v) => Bound::Json(PgJson::new(serde_json::to_value(v)?)),
            Self::Bindings(v) => Bound::Json(PgJson::new(serde_json::to_value(v)?)),
            Self::DiscoveryType(v) => Bound::Json(PgJson::new(serde_json::to_value(v)?)),
            Self::IntegrationTargets(v) => Bound::Json(PgJson::new(serde_json::to_value(v)?)),
            Self::EmailSettings(v) => Bound::Json(PgJson::new(serde_json::to_value(v)?)),
            Self::DisplaySettings(v) => Bound::Json(PgJson::new(serde_json::to_value(v)?)),
            Self::OptionBillingPlan(v) => Bound::Json(PgJson::new(serde_json::to_value(v)?)),
            Self::BillingOperation(v) => Bound::Json(PgJson::new(serde_json::to_value(v)?)),
            Self::AuthenticatedEntity(v) => Bound::Json(PgJson::new(serde_json::to_value(v)?)),
            Self::Nodes(v) => Bound::Json(PgJson::new(serde_json::to_value(v)?)),
            Self::Edges(v) => Bound::Json(PgJson::new(serde_json::to_value(v)?)),
            Self::TopologyOptions(v) => Bound::Json(PgJson::new(serde_json::to_value(v)?)),
            Self::Hosts(v) => Bound::Json(PgJson::new(serde_json::to_value(v)?)),
            Self::Subnets(v) => Bound::Json(PgJson::new(serde_json::to_value(v)?)),
            Self::Services(v) => Bound::Json(PgJson::new(serde_json::to_value(v)?)),
            Self::Dependencies(v) => Bound::Json(PgJson::new(serde_json::to_value(v)?)),
            Self::Interfaces(v) => Bound::Json(PgJson::new(serde_json::to_value(v)?)),
            Self::Tags(v) => Bound::Json(PgJson::new(serde_json::to_value(v)?)),
            Self::Vlans(v) => Bound::Json(PgJson::new(serde_json::to_value(v)?)),
            Self::OrgNotifications(v) => Bound::Json(PgJson::new(serde_json::to_value(v)?)),
            Self::OnboardingOperation(v) => Bound::Json(PgJson::new(serde_json::to_value(v)?)),
            Self::ShareOptions(v) => Bound::Json(PgJson::new(serde_json::to_value(v)?)),
            Self::RunType(v) => {
                // Transient `scanned: ScannedEntityIds` rides the wire (daemon → server) and the
                // in-memory entity carried by EntityOperation::Created event scope, but never
                // persists. Clear it on a typed clone before serializing so the persisted JSONB
                // never contains `scanned`. The in-memory struct held by callers is unchanged —
                // subscribers reading the entity off the EntityScope still see the populated
                // scanned.
                let mut rt_for_storage = v.clone();
                if let crate::server::discovery::r#impl::types::RunType::Historical { results } =
                    &mut rt_for_storage
                {
                    results.scanned = None;
                }
                Bound::Json(PgJson::new(serde_json::to_value(&rt_for_storage)?))
            }
            Self::CredentialType(v) => {
                // Expose secrets only for this DB write; the credential's default serialization
                // (API responses, logs, events) stays redacted.
                let _expose = crate::server::credentials::r#impl::types::ExposeSecretsGuard::new();
                Bound::Json(PgJson::new(serde_json::to_value(v)?))
            }
            Self::OptionalLldpChassisId(v) => Bound::OptJson(
                v.as_ref()
                    .map(|c| serde_json::to_value(c).map(PgJson::new))
                    .transpose()?,
            ),
            Self::OptionalLldpPortId(v) => Bound::OptJson(
                v.as_ref()
                    .map(|p| serde_json::to_value(p).map(PgJson::new))
                    .transpose()?,
            ),
            Self::OptionalFdbMacs(v) => Bound::OptJson(
                v.as_ref()
                    .map(|m| serde_json::to_value(m).map(PgJson::new))
                    .transpose()?,
            ),
            Self::OptionVecU16(v) => Bound::OptJson(
                v.as_ref()
                    .map(|ids| serde_json::to_value(ids).map(PgJson::new))
                    .transpose()?,
            ),
            Self::OptionVecUuid(v) => Bound::OptJson(
                v.as_ref()
                    .map(|ids| serde_json::to_value(ids).map(PgJson::new))
                    .transpose()?,
            ),
            Self::EnabledViews(v) => Bound::OptJson(
                v.as_ref()
                    .map(|views| serde_json::to_value(views).map(PgJson::new))
                    .transpose()?,
            ),

            // ── shapes that cannot carry a NUL ────────────────────────────────
            Self::Uuid(v) => Bound::Uuid(v),
            Self::OptionalUuid(v) => Bound::OptUuid(v),
            Self::UuidArray(v) => Bound::UuidArray(v.clone()),
            Self::OptionalUuidVec(v) => Bound::OptUuidArray(v.clone()),
            Self::I32(v) => Bound::I32(*v),
            Self::OptionalI32(v) => Bound::OptI32(v),
            Self::U16(v) => Bound::I32(i32::from(*v)),
            Self::I64(v) => Bound::I64(*v),
            Self::OptionalI64(v) => Bound::OptI64(v),
            Self::Bool(v) => Bound::Bool(v),
            Self::Timestamp(v) => Bound::Timestamp(v),
            Self::OptionTimestamp(v) => Bound::OptTimestamp(v),
            // Converted to IpNetwork for proper INET binding.
            Self::IpAddr(v) => Bound::IpNet(IpNetwork::from(*v)),
            Self::OptionalIpAddr(v) => Bound::OptIpNet(v.map(IpNetwork::from)),
            // sqlx's mac_address feature supports MacAddress directly.
            Self::MacAddress(v) => Bound::Mac(*v),
            Self::OptionalMacAddress(v) => Bound::OptMac(*v),
            Self::MacAddressArray(v) => Bound::MacArray(v.clone()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn strip_nuls_borrows_a_clean_string() {
        assert!(matches!(strip_nuls("eth1/0/1"), Cow::Borrowed(_)));
    }

    #[test]
    fn strip_nuls_removes_a_trailing_nul() {
        // The exact shape D-Link's DGS series sends for lldpRemPortId: `31 00`.
        assert_eq!(strip_nuls("1\0"), "1");
    }

    #[test]
    fn strip_nuls_removes_interior_and_repeated_nuls() {
        assert_eq!(strip_nuls("Port\0 9\0\0"), "Port 9");
    }

    #[test]
    fn pg_text_sanitises_and_reports() {
        let clean = PgText::new("Slot0/9");
        assert_eq!(clean.clone().into_inner(), "Slot0/9");
        assert!(!Bound::Text(clean).stripped());

        let dirty = PgText::new("Slot0/9\0");
        assert_eq!(dirty.clone().into_inner(), "Slot0/9");
        assert!(Bound::Text(dirty).stripped());
    }

    #[test]
    fn pg_text_owned_keeps_a_clean_allocation() {
        assert_eq!(
            PgText::owned("D-Link Port 9".to_string()).into_inner(),
            "D-Link Port 9"
        );
        assert_eq!(
            PgText::owned("D-Link Port 9\0".to_string()).into_inner(),
            "D-Link Port 9"
        );
    }

    #[test]
    fn pg_json_sanitises_nested_values_arrays_and_keys() {
        // The LldpPortId shape: {"subtype": "InterfaceName", "value": "1\0"}.
        let value = json!({
            "subtype": "InterfaceName",
            "value": "1\u{0}",
            "nested": { "deep\u{0}key": ["a\u{0}", 1, null, true] },
        });
        let cleaned = PgJson::new(value);
        assert!(cleaned.stripped);
        assert_eq!(
            cleaned.into_inner(),
            json!({
                "subtype": "InterfaceName",
                "value": "1",
                "nested": { "deepkey": ["a", 1, null, true] },
            })
        );
    }

    #[test]
    fn pg_json_leaves_a_clean_document_alone() {
        let value = json!({"subtype": "LocallyAssigned", "value": "eth1/0/51"});
        let cleaned = PgJson::new(value.clone());
        assert!(!cleaned.stripped);
        assert_eq!(cleaned.into_inner(), value);
    }

    #[test]
    fn null_bool_and_number_survive_untouched() {
        let value = json!([null, true, 1, 1.5, -2]);
        let cleaned = PgJson::new(value.clone());
        assert!(!cleaned.stripped);
        assert_eq!(cleaned.into_inner(), value);
    }
}
