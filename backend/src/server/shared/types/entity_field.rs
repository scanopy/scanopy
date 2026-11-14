//! What each entity type is described by: the registry CSV export, the list tables, the topology
//! inspector and the diagram exports all read.
//!
//! A field names a value on the entity's serialized JSON, or on entities reached from it through
//! `path`. The registry is declared per type in `shared/entity_fields.rs` and travels to the
//! frontend inside the `entities` and `edge_types` fixtures, so every consumer resolves the same
//! fields the same way instead of each keeping its own list.

use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::storage::{
    relation::{Reference, Relation},
    traits::Entity,
};
use crate::server::shared::types::{
    Color, Icon,
    metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
};
use serde::Serialize;
use std::any::Any;
use strum_macros::{EnumIter, IntoStaticStr};
use utoipa::ToSchema;

/// The heading a field is listed under wherever fields are picked, such as a table's column
/// menu. One vocabulary for every entity: a MAC address is `Network` on hosts and interfaces
/// alike. Variant order is display order.
///
/// A field that points at other entities takes the first of these that fits:
/// `Virtualization` when it says what virtualizes, contains or manages the entity; `Network`
/// when the entities it reaches are addressing or connectivity (interfaces, IP addresses, ports,
/// bindings, subnets, VLANs); `Relationships` otherwise. A host's services are
/// `Relationships`: a service is what runs on the host, not how it is reached, and its ports
/// already show as the host's `Network` ports.
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash, ToSchema, EnumIter, IntoStaticStr)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum FieldGroup {
    /// Name, type, description, colour and icon, and identifiers unique to this one instance (a
    /// serial number, an asset tag, a chassis ID).
    Identity,
    /// Values that address something on the network, even the entity's own key (a port number,
    /// a CIDR, a VLAN number), and the interfaces, IP addresses, ports, bindings, subnets and
    /// VLANs it reaches.
    Network,
    /// Hardware and system facts read from the device.
    Device,
    /// What the entity runs under, or the virtual network it manages.
    Virtualization,
    /// Other entities it belongs to or holds that are not network addressing or virtualization:
    /// its host, services, site, daemon, user, tag group.
    Relationships,
    /// Current state and recent activity.
    Status,
    /// Settings that control how it behaves.
    Configuration,
    /// Permissions, enablement, expiry and sign-in.
    Access,
    /// How and when Scanopy found it.
    Discovery,
    /// When the record was created and last updated.
    Timestamps,
}

impl HasId for FieldGroup {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for FieldGroup {
    fn color(&self) -> Color {
        Color::Gray
    }

    fn icon(&self) -> Icon {
        match self {
            Self::Identity => Icon::Crosshair,
            Self::Network => Icon::Network,
            Self::Device => Icon::Cpu,
            Self::Virtualization => Icon::Boxes,
            Self::Relationships => Icon::Link,
            Self::Status => Icon::Activity,
            Self::Configuration => Icon::Settings,
            Self::Access => Icon::KeyRound,
            Self::Discovery => Icon::Radar,
            Self::Timestamps => Icon::Clock,
        }
    }
}

impl TypeMetadataProvider for FieldGroup {
    fn name(&self) -> &'static str {
        match self {
            Self::Identity => "Identity",
            Self::Network => "Network",
            Self::Device => "Device",
            Self::Virtualization => "Virtualization",
            Self::Relationships => "Relationships",
            Self::Status => "Status",
            Self::Configuration => "Configuration",
            Self::Access => "Access",
            Self::Discovery => "Discovery",
            Self::Timestamps => "Timestamps",
        }
    }
}

/// One step from an object to related objects.
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, ToSchema)]
#[serde(tag = "type")]
pub enum Hop {
    /// The entities named by `key` on the current object: one id, or an array of ids.
    #[schema(title = "Parent")]
    Parent {
        entity: EntityDiscriminants,
        /// Field on the current object holding the parent's id or ids.
        key: &'static str,
    },
    /// Every entity of type `entity` whose `key` holds the current object's id.
    #[schema(title = "Children")]
    Children {
        entity: EntityDiscriminants,
        /// Field on each child holding the current object's id.
        key: &'static str,
    },
    /// The array of objects held on the current object under `key`, such as a service's bindings.
    #[schema(title = "Embedded")]
    Embedded {
        /// Field on the current object holding the array.
        key: &'static str,
    },
}

impl Hop {
    /// The entity whose id `reference.key` holds.
    pub const fn parent(reference: Reference) -> Self {
        Hop::Parent {
            entity: reference.to,
            key: reference.key,
        }
    }

    /// Every `T` whose `relation` column holds the current object's id.
    pub const fn children<T: Entity>(relation: &Relation<T>) -> Self {
        Hop::Children {
            entity: T::ENTITY_TYPE,
            key: relation.column,
        }
    }

    pub const fn embedded(key: &'static str) -> Self {
        Hop::Embedded { key }
    }
}

/// A metadata fixture whose entries name an enum-valued field, so the UI shows the fixture's
/// name for the raw value. Serializes to the fixture key.
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, ToSchema, strum::EnumIter)]
#[serde(rename_all = "snake_case")]
pub enum MetadataFixture {
    IfOperStatuses,
    GuestStates,
    SubnetTypes,
    EntitySources,
    DependencyTypes,
    DiscoveryTypes,
    ServiceDefinitions,
    DiscoveryProtocols,
    Permissions,
    CredentialTypes,
}

/// How a field's value is written. Chosen by the value's shape, never by where it is shown.
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, ToSchema)]
#[serde(tag = "type")]
pub enum FieldFormat {
    /// Plain text.
    #[schema(title = "Text")]
    Text,
    /// Free text a person wrote, such as a description. Tables wrap it rather than truncating.
    #[schema(title = "Prose")]
    Prose,
    /// A number, written as is.
    #[schema(title = "Number")]
    Number,
    /// True or false.
    #[schema(title = "Boolean")]
    Boolean,
    /// A point in time, written as a date and time.
    #[schema(title = "Timestamp")]
    Timestamp,
    /// A value read as a code rather than prose (MAC, CIDR, chassis id): monospace, copyable.
    #[schema(title = "Identifier")]
    Identifier,
    /// `speed_bps`, written as 1G / 100M.
    #[schema(title = "BitsPerSecond")]
    BitsPerSecond,
    /// A `{number, protocol}` port, written as 80/tcp.
    #[schema(title = "Port")]
    Port,
    /// A host `os` object, written as its name and version.
    #[schema(title = "OperatingSystem")]
    OperatingSystem,
    /// An enum value, named through a metadata fixture.
    #[schema(title = "Metadata")]
    Metadata { fixture: MetadataFixture },
    /// Where a value came from (an `AttributeSource`), named as the UI names sources elsewhere:
    /// "SNMP", "Forwarding table".
    #[schema(title = "AttributeSource")]
    AttributeSource,
    /// A related entity. The UI links it; CSV writes its id; text-only outputs write its display
    /// field.
    #[schema(title = "Entity")]
    Entity,
}

/// Copies a value from the stored entity onto the incoming one. Both arguments are the entity type
/// the registry entry belongs to. Written by the `pin!`, `keep!` and `merge!` macros in
/// `shared/entity_fields.rs`, which name the field with a checked path. Serialized values can't do
/// this: an entity's JSON leaves out `#[serde(skip)]` fields and redacts its secrets.
#[derive(Clone, Copy)]
pub struct FieldRule(pub fn(&mut dyn Any, &dyn Any));

impl FieldRule {
    pub fn apply(&self, incoming: &mut dyn Any, stored: &dyn Any) {
        (self.0)(incoming, stored)
    }
}

impl std::fmt::Debug for FieldRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FieldRule")
    }
}

impl PartialEq for FieldRule {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::fn_addr_eq(self.0, other.0)
    }
}

impl Eq for FieldRule {}

/// Who writes a field, and what an update does with an incoming value. Applied by
/// `Entity::preserve_immutable_fields`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FieldWrite {
    /// The caller sets it, on create and on update.
    #[default]
    Writable,
    /// The caller sets it on create. Every update keeps the stored value.
    Immutable(FieldRule),
    /// Only the server sets it, so it never deserializes from a request. Every update keeps the
    /// stored value.
    ServerOwned(FieldRule),
    /// `None` on an update means the source had nothing to say, so the stored value stays.
    KeepWhenNone(FieldRule),
    /// An update reconciles the stored and incoming values by a rule named on the entity. Runs
    /// after every other policy, in registry order, so a rule can read the fields settled before
    /// it.
    Merged(FieldRule),
}

impl FieldWrite {
    /// The rule to run in the first pass of an update.
    pub fn settled_rule(&self) -> Option<FieldRule> {
        match *self {
            Self::Immutable(rule) | Self::ServerOwned(rule) | Self::KeepWhenNone(rule) => {
                Some(rule)
            }
            Self::Writable | Self::Merged(_) => None,
        }
    }

    /// The rule to run in the second pass of an update.
    pub fn merged_rule(&self) -> Option<FieldRule> {
        match *self {
            Self::Merged(rule) => Some(rule),
            _ => None,
        }
    }
}

/// One fact that describes an entity, possibly read from a related entity. Every serialized key an
/// entity carries has one: the flags say which surfaces show it, and `write` says who sets it.
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, ToSchema)]
pub struct EntityField {
    /// Stable key: the CSV header, the list-table column id and the diagram Shape Data name.
    pub id: &'static str,
    /// English label; the UI translates it through a generated `meta_*` key.
    pub label: &'static str,
    pub group: FieldGroup,
    /// Steps from the entity to the objects the value is read from. Empty for the entity itself.
    pub path: &'static [Hop],
    /// Dotted path to the value on the last object, or `None` for the object itself (an
    /// `Entity` or `Port` field).
    pub field: Option<&'static str>,
    pub format: FieldFormat,
    /// Sibling field holding the value's attribution, for attributed host facts.
    pub source_field: Option<&'static str>,
    /// Shown by default in tables and in the inspector's details.
    pub summary: bool,
    /// Shown on the entity's hover tooltip, for a fact that isn't a `summary` default column.
    pub tooltip: bool,
    /// Whether one entity can hold several values: set by any `Children` or `Embedded` hop, and
    /// declared with [`EntityField::many`] for a key holding an array of ids or values.
    pub many: bool,
    /// Matched by the topology search: a value that names or identifies the entity on the map
    /// (a name, an address, a MAC, a port name, a vendor). The search reads only these.
    pub searchable: bool,
    /// Offered as a list-table column.
    pub table: bool,
    /// Shown by the topology inspector and diagram Shape Data, and read by the topology search.
    pub inspector: bool,
    /// Written to CSV export.
    #[serde(skip)]
    pub csv: bool,
    /// Worked out by the table or a response type rather than held on the stored entity.
    #[serde(skip)]
    pub computed: bool,
    /// Never serialized, so the entity's JSON and schema don't carry it.
    #[serde(skip)]
    pub unserialized: bool,
    /// Who sets it, and what an update does with an incoming value.
    #[serde(skip)]
    pub write: FieldWrite,
}

/// Whether a path can reach several objects from one.
const fn path_is_many(path: &[Hop]) -> bool {
    let mut i = 0;
    while i < path.len() {
        if matches!(path[i], Hop::Children { .. } | Hop::Embedded { .. }) {
            return true;
        }
        i += 1;
    }
    false
}

impl EntityField {
    /// A text field read from `id` on the entity itself.
    pub const fn new(id: &'static str, label: &'static str, group: FieldGroup) -> Self {
        EntityField {
            id,
            label,
            group,
            path: &[],
            field: Some(id),
            format: FieldFormat::Text,
            source_field: None,
            summary: false,
            tooltip: false,
            many: false,
            searchable: false,
            table: true,
            inspector: true,
            csv: true,
            computed: false,
            unserialized: false,
            write: FieldWrite::Writable,
        }
    }

    /// A list-table column and nothing else: shown only in tables, never exported.
    pub const fn column(id: &'static str, label: &'static str, group: FieldGroup) -> Self {
        EntityField {
            inspector: false,
            csv: false,
            ..Self::new(id, label, group)
        }
    }

    /// A field no surface shows, declared for its references or its write policy.
    pub const fn internal(id: &'static str) -> Self {
        EntityField {
            table: false,
            inspector: false,
            csv: false,
            ..Self::new(id, "", FieldGroup::Configuration)
        }
    }

    /// Whether any surface shows this field.
    pub const fn is_internal(&self) -> bool {
        !self.table && !self.inspector && !self.csv
    }

    /// This field as CSV reads it. A field read through `Hop::Children` writes the ids of the rows
    /// its path ends on, which join to those rows' own export. Every other surface reads the field
    /// as declared.
    pub const fn for_csv(self) -> Self {
        let mut i = 0;
        while i < self.path.len() {
            if matches!(self.path[i], Hop::Children { .. }) {
                return EntityField {
                    field: None,
                    format: FieldFormat::Entity,
                    ..self
                };
            }
            i += 1;
        }
        self
    }

    /// The table works this value out, or reads it from another endpoint or a response type.
    pub const fn computed(self) -> Self {
        EntityField {
            computed: true,
            ..self
        }
    }

    /// The field is `#[serde(skip)]` or `skip_serializing`.
    pub const fn unserialized(self) -> Self {
        EntityField {
            unserialized: true,
            ..self
        }
    }

    pub const fn immutable(self, rule: FieldRule) -> Self {
        EntityField {
            write: FieldWrite::Immutable(rule),
            ..self
        }
    }

    pub const fn server_owned(self, rule: FieldRule) -> Self {
        EntityField {
            write: FieldWrite::ServerOwned(rule),
            ..self
        }
    }

    pub const fn keep_when_none(self, rule: FieldRule) -> Self {
        EntityField {
            write: FieldWrite::KeepWhenNone(rule),
            ..self
        }
    }

    pub const fn merged(self, rule: FieldRule) -> Self {
        EntityField {
            write: FieldWrite::Merged(rule),
            ..self
        }
    }

    /// Matched by the topology search.
    pub const fn searchable(self) -> Self {
        EntityField {
            searchable: true,
            ..self
        }
    }

    /// Read the value from `field` instead of the field named by `id`.
    pub const fn field(self, field: &'static str) -> Self {
        EntityField {
            field: Some(field),
            ..self
        }
    }

    pub const fn format(self, format: FieldFormat) -> Self {
        EntityField { format, ..self }
    }

    /// The related entities themselves, reached through `path`.
    pub const fn entity(self, path: &'static [Hop]) -> Self {
        EntityField {
            path,
            field: None,
            format: FieldFormat::Entity,
            many: path_is_many(path),
            ..self
        }
    }

    /// Read `field` on the objects reached through `path`.
    pub const fn via(self, path: &'static [Hop], field: Option<&'static str>) -> Self {
        EntityField {
            path,
            field,
            many: path_is_many(path),
            ..self
        }
    }

    pub const fn source(self, source_field: &'static str) -> Self {
        EntityField {
            source_field: Some(source_field),
            ..self
        }
    }

    pub const fn summary(self) -> Self {
        EntityField {
            summary: true,
            ..self
        }
    }

    pub const fn tooltip(self) -> Self {
        EntityField {
            tooltip: true,
            ..self
        }
    }

    pub const fn metadata(self, fixture: MetadataFixture) -> Self {
        self.format(FieldFormat::Metadata { fixture })
    }

    /// The value is an array: several ids or values held on one key.
    pub const fn many(self) -> Self {
        EntityField { many: true, ..self }
    }
}
