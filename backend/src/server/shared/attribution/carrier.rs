//! A value and the evidence that produced it, as one thing.
//!
//! `hosts/impl/name.rs` records why this is a carrier rather than a second field beside each
//! value: *"The first fix carried the rung in a second `HostBase` field beside `name`. Two fields
//! that must move together is a standing invitation to move only one: three construction sites
//! assigned `name` directly and let the rung default, and one of them shipped a host labelled with
//! an address it no longer held."* Subnets then shipped the two-field shape anyway, and
//! `subnets/impl/storage.rs` grew two `Storable` hooks writing `cidr_source` behind the applier's
//! back. One carrier, one applier, no second field to forget.

use std::borrow::Cow;
use std::fmt;
use std::marker::PhantomData;

use serde::de::{self, IgnoredAny, MapAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::{Deserializer, Serialize, Serializer};
use utoipa::openapi::schema::{ObjectBuilder, SchemaType, Type};
use utoipa::openapi::{Ref, RefOr, Schema};
use utoipa::{PartialSchema, ToSchema};

use super::AttributeSource;

/// The value half of a provenanced pair.
///
/// Deliberately separate from provenance: this is about the value being meaningful and about which
/// two keys it occupies, not about where it came from. The keys live here rather than at the field
/// declaration so a carrier and its keys cannot disagree — `Attributed<HostModelValue>` is the only
/// thing that can produce `model`/`model_source`.
pub trait AttributeValue:
    Sized + Clone + PartialEq + Serialize + serde::de::DeserializeOwned + PartialSchema
{
    /// Wire key and column name carrying the value.
    const VALUE_KEY: &'static str;
    /// Wire key and column name carrying the provenance. By convention `<VALUE_KEY>_source`.
    const SOURCE_KEY: &'static str;
    /// Component name for `Attributed<Self>` in the OpenAPI document.
    const SCHEMA_NAME: &'static str;

    /// Whether the value key is `required` in the schema of whatever flattens this.
    ///
    /// `true` only for the two that genuinely always have a value — a host's name and a subnet's
    /// range. Everything else is an optional attribute, and saying so is what stops a consumer
    /// having to handle an absence the type says cannot happen. Defaults to `false`, so a field
    /// has to claim the stronger guarantee deliberately.
    const VALUE_REQUIRED: bool = false;

    /// Whether a re-read from the same source at the same rung may change the value.
    ///
    /// `false` for the things that do not change in reality — a different serial number is a
    /// different device — so an equal-rung re-read cannot flap them. It gates only that arm: a
    /// strictly stronger source may still correct an immutable value, which is the case where a
    /// weak source wrote a wrong serial and SNMP later reads the right one.
    const REFRESHABLE: bool = true;

    /// A value that is present but empty: an absent value wearing a source.
    fn is_blank(&self) -> bool {
        false
    }
}

/// The schema most provenanced values want: a described string.
///
/// Here rather than repeated per declaration so the fourteen value types differ only in the words.
pub fn string_schema(description: &'static str) -> RefOr<Schema> {
    RefOr::T(Schema::Object(
        ObjectBuilder::new()
            .schema_type(SchemaType::new(Type::String))
            .description(Some(description))
            .build(),
    ))
}

/// The plain value of an optional pair, for the places that want a value without its provenance:
/// a CSV column, an API field that has always been a bare string, a log line.
pub fn text_of<T: AttributeValue + fmt::Display>(slot: &Option<Attributed<T>>) -> Option<String> {
    slot.as_ref().map(|carrier| carrier.value().to_string())
}

/// A value that is only allowed to exist alongside the source that produced it.
///
/// Fields are private behind [`Attributed::new`], so a call site cannot assign a value and let the
/// source default. There is no `Default` for the same reason.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Attributed<T: AttributeValue> {
    value: T,
    source: AttributeSource,
}

impl<T: AttributeValue> Attributed<T> {
    /// The two columns this pair occupies, in `to_params` order.
    pub const COLUMNS: [&'static str; 2] = [T::VALUE_KEY, T::SOURCE_KEY];

    pub fn new(value: T, source: AttributeSource) -> Self {
        Self { value, source }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn into_value(self) -> T {
        self.value
    }

    pub fn source(&self) -> AttributeSource {
        self.source
    }

    pub fn rank(&self) -> u8 {
        self.source.rank()
    }

    pub fn is_blank(&self) -> bool {
        self.value.is_blank()
    }

    /// Lower the recorded provenance to `ceiling` if it claims more, keeping the value.
    ///
    /// The server applies this to daemon payloads, and it can only ever move the rung down.
    pub fn clamped_to(mut self, ceiling: AttributeSource) -> Self {
        if self.rank() > ceiling.rank() {
            self.source = ceiling;
        }
        self
    }

    /// Whether `candidate` may replace `existing`. The ordering rules, in one place, so the two
    /// appliers below cannot drift apart.
    fn supersedes(existing: &Self, candidate: &Self) -> bool {
        if candidate.rank() < existing.rank() || existing == candidate {
            return false;
        }
        // Equal rank replaces only from the same source, and only where the value can legitimately
        // move: that lets a firmware revision follow an upgrade when re-read, without letting two
        // sources at one rung flap the value depending on which finished first in a given scan.
        if candidate.rank() == existing.rank()
            && (existing.source != candidate.source || !T::REFRESHABLE)
        {
            return false;
        }
        true
    }

    /// The applier, for a field that may be absent. Returns whether anything changed.
    ///
    /// `upsert_host` publishes an `Updated` event and triggers a topology rebuild off this return
    /// value, so a scan that learns nothing new has to report `false` rather than write silently.
    pub fn apply(slot: &mut Option<Self>, candidate: Self) -> bool {
        // A blank candidate is an absent value, not a value — it must never displace a real one.
        if candidate.is_blank() {
            return false;
        }
        // A blank incumbent is an absent value wearing a source, and must not block a real one
        // however highly it claims to rank.
        if let Some(existing) = slot
            && !existing.is_blank()
            && !Self::supersedes(existing, &candidate)
        {
            return false;
        }
        *slot = Some(candidate);
        true
    }

    /// The same rules for a field that is always present — a host's name, a subnet's CIDR — where
    /// "absent" is a blank value rather than a `None`.
    pub fn apply_in_place(&mut self, candidate: Self) -> bool {
        if candidate.is_blank() {
            return false;
        }
        if !self.is_blank() && !Self::supersedes(self, &candidate) {
            return false;
        }
        *self = candidate;
        true
    }
}

/// Renders the value alone. Provenance is not part of how a value reads in a log line, a topology
/// label or an export — it is a property of how we came to know it.
impl<T: AttributeValue + fmt::Display> fmt::Display for Attributed<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(f)
    }
}

/// Serialised as two flat keys, not as a nested object or a tagged enum.
///
/// The value key has to stay a bare scalar at the top level: daemons at 0.17.11 and earlier POST
/// `{"name": "<string>"}` with no rung at all, `name` carries `ORDER BY` and the free-text host
/// search, and `cidr` is queried directly.
impl<T: AttributeValue> Serialize for Attributed<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // `serialize_struct`, so serde's `FlatMapSerializer` writes both keys into the parent map.
        let mut state = serializer.serialize_struct(T::SCHEMA_NAME, 2)?;
        state.serialize_field(T::VALUE_KEY, &self.value)?;
        state.serialize_field(T::SOURCE_KEY, &self.source)?;
        state.end()
    }
}

impl<T: AttributeValue> Attributed<T> {
    /// The one read path. `Ok(None)` means there is no usable value here — the key was absent, or
    /// its value was blank.
    ///
    /// **Must call `deserialize_map`, never `deserialize_struct`.** Serde's `FlatMapDeserializer`
    /// hands `deserialize_map` a `FlatMapAccess` that *borrows* each entry, leaving it available
    /// for the sibling flattened fields; `deserialize_struct` gets a `FlatStructAccess` that takes
    /// entries and nulls them out. With twelve carriers flattened into one struct, that one word
    /// would silently let the first eat every other carrier's keys.
    fn read<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Self>, D::Error> {
        struct CarrierVisitor<T>(PhantomData<T>);

        impl<'de, T: AttributeValue> Visitor<'de> for CarrierVisitor<T> {
            type Value = Option<Attributed<T>>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    f,
                    "an object with a `{}` and an optional `{}`",
                    T::VALUE_KEY,
                    T::SOURCE_KEY
                )
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut value: Option<T> = None;
                let mut source: Option<AttributeSource> = None;

                // Remaining keys belong to the flattening parent and to the sibling carriers.
                // Skip rather than reject.
                while let Some(key) = map.next_key::<Cow<'_, str>>()? {
                    match key.as_ref() {
                        // A pre-provenance daemon posts an explicit JSON `null` for a value it
                        // doesn't have (e.g. a loopback interface's `mac_address`) where a plain
                        // `Option<T>` field used to swallow it. `next_value::<Option<T>>` keeps
                        // that behavior instead of failing `T`'s deserializer on `null`.
                        k if k == T::VALUE_KEY => value = map.next_value::<Option<T>>()?,
                        k if k == T::SOURCE_KEY => source = Some(map.next_value()?),
                        _ => {
                            map.next_value::<IgnoredAny>()?;
                        }
                    }
                }

                Ok(match value {
                    Some(value) if value.is_blank() => None,
                    // A payload predating provenance. The value is real but unattributable, so it
                    // enters at the bottom and cannot displace anything we know the source of.
                    Some(value) => Some(Attributed::new(
                        value,
                        source.unwrap_or(AttributeSource::Unspecified),
                    )),
                    None => None,
                })
            }
        }

        deserializer.deserialize_map(CarrierVisitor::<T>(PhantomData))
    }
}

/// For an `Option<Attributed<_>>` field: absent or blank reads as `None`.
///
/// A `deserialize_with` rather than plain `#[serde(flatten)] Option<T>`, which reaches `None` only
/// by making the inner deserializer fail and then swallowing *every* error with `.ok()` — so a
/// malformed value would silently become `None` instead of a 400.
pub fn optional<'de, D, T>(deserializer: D) -> Result<Option<Attributed<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: AttributeValue,
{
    Attributed::<T>::read(deserializer)
}

/// For a field with no empty state, such as a subnet's CIDR: absent is an error.
pub fn required<'de, D, T>(deserializer: D) -> Result<Attributed<T>, D::Error>
where
    D: Deserializer<'de>,
    T: AttributeValue,
{
    Attributed::<T>::read(deserializer)?.ok_or_else(|| de::Error::missing_field(T::VALUE_KEY))
}

impl<T: AttributeValue> PartialSchema for Attributed<T> {
    fn schema() -> RefOr<Schema> {
        let mut object = ObjectBuilder::new()
            .schema_type(SchemaType::new(Type::Object))
            // The value's own schema carries its description, pattern and example, so those
            // live with the value type rather than being repeated per entity.
            .property(T::VALUE_KEY, <T as PartialSchema>::schema())
            // A `$ref` rather than `AttributeSource::schema()`, which would inline a copy of
            // the enum into every carrier component instead of pointing at the shared one.
            .property(
                T::SOURCE_KEY,
                Ref::from_schema_name(<AttributeSource as ToSchema>::name()),
            );

        // The source is never required: a payload that omits it reads as `Unspecified`, which is
        // what lets a daemon predating provenance keep POSTing bare values.
        if T::VALUE_REQUIRED {
            object = object.required(T::VALUE_KEY);
        }

        RefOr::T(Schema::Object(object.build()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::ip_addresses::r#impl::base::MacEvidenceValue;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct Carrier {
        #[serde(flatten, deserialize_with = "optional")]
        mac_address: Option<Attributed<MacEvidenceValue>>,
    }

    /// A daemon predating provenance (or any interface/address genuinely without a MAC, like
    /// loopback) posts an explicit JSON `null` rather than omitting the key. That must read as
    /// absent, not fail `T`'s deserializer on `null`.
    #[test]
    fn an_explicit_null_reads_as_absent() {
        let carrier: Carrier = serde_json::from_str(r#"{"mac_address": null}"#).unwrap();
        assert!(carrier.mac_address.is_none());
    }

    #[test]
    fn a_missing_key_reads_as_absent() {
        let carrier: Carrier = serde_json::from_str("{}").unwrap();
        assert!(carrier.mac_address.is_none());
    }

    #[test]
    fn a_present_value_still_deserializes() {
        let carrier: Carrier =
            serde_json::from_str(r#"{"mac_address": "a4:bb:6d:12:34:56"}"#).unwrap();
        assert_eq!(
            carrier.mac_address.unwrap().value,
            MacEvidenceValue("a4:bb:6d:12:34:56".parse().unwrap())
        );
    }
}

impl<T: AttributeValue> ToSchema for Attributed<T> {
    fn name() -> Cow<'static, str> {
        Cow::Borrowed(T::SCHEMA_NAME)
    }

    /// Register the enum the `$ref` above points at.
    ///
    /// `HostName`'s hand-written `ToSchema` omitted this and its ref resolved only because
    /// `HostResponse.name_source` happened to be a derived field of that type. That accident does
    /// not survive the generalisation — most of these carriers have no such twin.
    fn schemas(schemas: &mut Vec<(String, RefOr<Schema>)>) {
        schemas.push((
            <AttributeSource as ToSchema>::name().into_owned(),
            <AttributeSource as PartialSchema>::schema(),
        ));
    }
}
