//! What each entity type is described by: the registry CSV export, the list tables, the topology
//! inspector and the diagram exports all read.
//!
//! A field names a value on the entity's serialized JSON, or on entities reached from it through
//! `path`. The registry is declared per type in `shared/entity_fields.rs` and travels to the
//! frontend inside the `entities` and `edge_types` fixtures, so every consumer resolves the same
//! fields the same way instead of each keeping its own list.

use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::types::{
    Color, Icon,
    metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
};
use serde::Serialize;
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
    pub const fn parent(entity: EntityDiscriminants, key: &'static str) -> Self {
        Hop::Parent { entity, key }
    }

    pub const fn children(entity: EntityDiscriminants, key: &'static str) -> Self {
        Hop::Children { entity, key }
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

/// A list-table column that is not a registry field: a value the table computes, reads from
/// another endpoint, or keeps out of CSV export (`created_at`, `last_seen_at`). Declared here so
/// the column menu files it under a group the backend chose. CSV, diagram Shape Data and the
/// inspector read only fields, never these.
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, ToSchema)]
pub struct TableColumn {
    /// The column's key in the list table, and so in saved column preferences.
    pub id: &'static str,
    pub group: FieldGroup,
}

impl TableColumn {
    pub const fn new(id: &'static str, group: FieldGroup) -> Self {
        TableColumn { id, group }
    }
}

/// One fact that describes an entity, possibly read from a related entity.
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
    /// Shown by default in tables and in the inspector's details. Every field is exported.
    pub summary: bool,
    /// Shown on the entity's hover tooltip, for a fact that isn't a `summary` default column.
    pub tooltip: bool,
    /// Whether one entity can hold several values: set by any `Children` or `Embedded` hop, and
    /// declared with [`EntityField::many`] for a key holding an array of ids or values.
    pub many: bool,
    /// Matched by the topology search: a value that names or identifies the entity on the map
    /// (a name, an address, a MAC, a port name, a vendor). The search reads only these.
    pub searchable: bool,
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
