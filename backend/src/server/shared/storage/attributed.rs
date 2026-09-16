//! Reading and writing a provenanced pair as two columns.
//!
//! Separate from `shared::attribution` so the domain types carry no sqlx dependency — `name.rs`
//! never had one either. What this adds is the pairing: a column name and its value are handed out
//! together, so the positional vectors in a `to_params` cannot be edited apart.

use anyhow::Result;
use serde_json::Value as JsonValue;
use sqlx::{Row, postgres::PgRow};

use crate::server::shared::attribution::{AttributeSource, AttributeValue, Attributed};
use crate::server::shared::storage::traits::SqlValue;

/// A provenanced value that knows how it sits in a column.
///
/// One impl per underlying representation rather than a blanket one: the value column's type is a
/// property of the value, and a `MacAddress` does not round-trip through the same `SqlValue` as a
/// `String`.
pub trait AttributeColumn: AttributeValue {
    /// The value half, as it binds. `None` writes SQL `NULL`.
    fn value_param(value: Option<&Self>) -> SqlValue;
    /// The value half, read back. `Ok(None)` for a `NULL` column.
    fn read_value(row: &PgRow) -> Result<Option<Self>>;
}

/// The two columns this pair occupies, and the two values that fill them, in the same order.
///
/// Handed out together so a `to_params` cannot name a column and bind somebody else's value to it.
pub fn optional_params<T: AttributeColumn>(slot: &Option<Attributed<T>>) -> [SqlValue; 2] {
    match slot {
        Some(carrier) => present_params(carrier),
        // The value is `NULL`, so its source says nothing. `Unspecified` rather than a source we
        // would have to invent: there is no value here to have come from anywhere.
        None => [
            T::value_param(None),
            SqlValue::AttributeSource(AttributeSource::Unspecified),
        ],
    }
}

/// The same, for a pair that is always present.
pub fn present_params<T: AttributeColumn>(carrier: &Attributed<T>) -> [SqlValue; 2] {
    [
        T::value_param(Some(carrier.value())),
        SqlValue::AttributeSource(carrier.source()),
    ]
}

/// Read a pair back. A `NULL` value reads as `None` whatever its source column holds.
pub fn read_optional<T: AttributeColumn>(row: &PgRow) -> Result<Option<Attributed<T>>> {
    let Some(value) = T::read_value(row)? else {
        return Ok(None);
    };
    if value.is_blank() {
        return Ok(None);
    }
    Ok(Some(Attributed::new(
        value,
        read_source(row, T::SOURCE_KEY)?,
    )))
}

/// Read a pair whose value column is `NOT NULL`. An absent value is a real error here, unlike an
/// optional attribute where it simply means the field is unset.
pub fn read_required<T: AttributeColumn>(row: &PgRow) -> Result<Attributed<T>> {
    let value = T::read_value(row)?
        .ok_or_else(|| anyhow::anyhow!("{} is required but was NULL", T::VALUE_KEY))?;
    Ok(Attributed::new(value, read_source(row, T::SOURCE_KEY)?))
}

/// The source half.
///
/// Unrecognised identifiers degrade to `Unspecified` inside `AttributeSource`'s own `Deserialize`,
/// so a rung a newer binary wrote costs the row its provenance and not its existence. Only a value
/// that is not a source at all reaches the error path here.
pub fn read_source(row: &PgRow, column: &str) -> Result<AttributeSource> {
    let raw: JsonValue = row.try_get(column)?;
    Ok(serde_json::from_value(raw)?)
}

/// String-backed values — the eleven host attributes.
macro_rules! impl_string_attribute_column {
    ($($t:ty),* $(,)?) => {
        $(
            impl AttributeColumn for $t {
                fn value_param(value: Option<&Self>) -> SqlValue {
                    SqlValue::OptionalString(value.map(|v| v.0.clone()))
                }

                fn read_value(row: &PgRow) -> Result<Option<Self>> {
                    Ok(row
                        .try_get::<Option<String>, _>(Self::VALUE_KEY)?
                        .map(Self))
                }
            }
        )*
    };
}

use crate::server::hosts::r#impl::attributes::{
    HostChassisIdValue, HostFirmwareRevisionValue, HostHostnameValue, HostManagementUrlValue,
    HostManufacturerValue, HostModelValue, HostSerialNumberValue, HostSoftwareRevisionValue,
    HostSysContactValue, HostSysDescrValue, HostSysLocationValue, HostSysNameValue,
    HostSysObjectIdValue,
};

impl_string_attribute_column!(
    HostHostnameValue,
    HostSysDescrValue,
    HostSysObjectIdValue,
    HostSysLocationValue,
    HostSysContactValue,
    HostManagementUrlValue,
    HostChassisIdValue,
    HostSysNameValue,
    HostManufacturerValue,
    HostModelValue,
    HostSerialNumberValue,
    HostFirmwareRevisionValue,
    HostSoftwareRevisionValue,
);

use crate::server::hosts::r#impl::name::HostNameValue;

impl AttributeColumn for HostNameValue {
    /// `name` is `TEXT NOT NULL`, so an absent name is the empty string rather than `NULL` — the
    /// column carries `ORDER BY` and the free-text host search and has always been non-null.
    fn value_param(value: Option<&Self>) -> SqlValue {
        SqlValue::String(value.map(|v| v.as_str().into_owned()).unwrap_or_default())
    }

    fn read_value(row: &PgRow) -> Result<Option<Self>> {
        Ok(Some(Self::Text(row.try_get::<String, _>(Self::VALUE_KEY)?)))
    }
}

use crate::server::ip_addresses::r#impl::base::MacEvidenceValue;

impl AttributeColumn for MacEvidenceValue {
    fn value_param(value: Option<&Self>) -> SqlValue {
        SqlValue::OptionalMacAddress(value.map(|v| v.0))
    }

    fn read_value(row: &PgRow) -> Result<Option<Self>> {
        Ok(row
            .try_get::<Option<mac_address::MacAddress>, _>(Self::VALUE_KEY)?
            .map(Self))
    }
}

use crate::server::subnets::r#impl::base::SubnetCidrValue;

impl AttributeColumn for SubnetCidrValue {
    fn value_param(value: Option<&Self>) -> SqlValue {
        // `cidr` is `NOT NULL`, and `SubnetBase.cidr` is not optional, so this is only ever
        // reached with a value.
        SqlValue::IpCidr(value.expect("a subnet's CIDR is required").0)
    }

    /// The column is `text` holding the JSON form, which is how `SqlValue::IpCidr` binds it —
    /// not a Postgres `cidr`, so this decodes the string rather than asking sqlx for an `IpCidr`.
    fn read_value(row: &PgRow) -> Result<Option<Self>> {
        let raw: String = row.try_get(Self::VALUE_KEY)?;
        Ok(Some(Self(serde_json::from_str(&raw)?)))
    }
}
