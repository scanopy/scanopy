//! Per-value provenance: what a discovered value is, and how it reached us.
//!
//! Before this, every attribute merged first-write-wins through an `is_none()` gate — three
//! separate copies of one, plus a writer that bypassed all three — so on a switch answering both
//! SNMP and EtherNet/IP whichever probe landed first owned the value permanently, and the source
//! that should usually win could not displace it. Precedence was an accident of scan ordering
//! rather than a decision anything stated.
//!
//! Three types and one trait. [`AttributeSource`] says which source; [`AttributeMethod`] is the
//! binding tier; [`Attributed`] carries a value with its source. The only trait is
//! [`AttributeValue`], and that is about values rather than provenance — with one source enum
//! there is nothing to abstract over.

mod carrier;
mod source;

pub use carrier::{AttributeValue, Attributed, optional, required, string_schema, text_of};
pub use source::{AttributeMethod, AttributeSource, AttributeSourceDiscriminants, Authorship};

/// Declare a provenanced value: its newtype, its [`AttributeValue`] impl and the alias every
/// field must be declared through.
///
/// The alias is not cosmetic. utoipa composes a component name for a *syntactically* generic field
/// type as `<Outer>_<Inner>`, so a field typed `Attributed<HostModelValue>` would publish as
/// `Attributed_HostModelValue` whatever `ToSchema::name()` returns. The derive cannot see through a
/// `type` alias, so declaring `HostModelAttributed` makes the type tree childless and the name is
/// honoured.
///
/// Both names are spelled out rather than concatenated from one stem: a reader grepping for
/// `HostModelAttributed` has to be able to find where it is declared.
#[macro_export]
macro_rules! attributed_value {
    (
        $(#[$meta:meta])*
        $vis:vis struct $value:ident($inner:ty) as $alias:ident {
            key: $key:literal,
            source_key: $source_key:literal,
            schema_name: $schema_name:literal,
            refreshable: $refreshable:literal,
            blank: $blank:expr,
            $(refines: $refines:expr,)?
            schema: $schema:expr,
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, ::serde::Serialize, ::serde::Deserialize)]
        #[serde(transparent)]
        $vis struct $value(pub $inner);

        impl ::std::convert::From<$inner> for $value {
            fn from(value: $inner) -> Self {
                Self(value)
            }
        }

        impl ::std::fmt::Display for $value {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                ::std::fmt::Display::fmt(&self.0, f)
            }
        }

        impl ::utoipa::PartialSchema for $value {
            fn schema() -> ::utoipa::openapi::RefOr<::utoipa::openapi::Schema> {
                $schema
            }
        }

        impl $crate::server::shared::attribution::AttributeValue for $value {
            const VALUE_KEY: &'static str = $key;
            const SOURCE_KEY: &'static str = $source_key;
            const SCHEMA_NAME: &'static str = $schema_name;
            const REFRESHABLE: bool = $refreshable;

            fn is_blank(&self) -> bool {
                #[allow(clippy::redundant_closure_call)]
                ($blank)(&self.0)
            }

            $(
                fn refines(&self, other: &Self) -> bool {
                    #[allow(clippy::redundant_closure_call)]
                    ($refines)(&self.0, &other.0)
                }
            )?
        }

        /// Declare fields through this alias, never through the generic form — see
        /// [`attributed_value!`] for why.
        $vis type $alias = $crate::server::shared::attribution::Attributed<$value>;
    };
}

#[cfg(test)]
mod tests;
