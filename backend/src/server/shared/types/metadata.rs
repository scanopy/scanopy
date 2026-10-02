use std::borrow::Cow;

use serde::Serialize;
use utoipa::ToSchema;

use super::{Color, Icon};

#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct MetadataRegistry {
    /// Every service Scanopy can identify, with its display metadata.
    pub service_definitions: Vec<TypeMetadata>,
    /// The subnet types entities can be classified as.
    pub subnet_types: Vec<TypeMetadata>,
    /// The relationships topology edges can represent.
    pub edge_types: Vec<TypeMetadata>,
    /// The ways nodes can be grouped into containers.
    pub group_types: Vec<TypeMetadata>,
    /// The entity types the API exposes, with their display metadata.
    pub entities: Vec<EntityMetadata>,
    /// The well-known ports Scanopy recognises.
    pub ports: Vec<TypeMetadata>,
    /// The types of discovery that can be run.
    pub discovery_types: Vec<TypeMetadata>,
    /// The plans available on this deployment.
    pub billing_plans: Vec<TypeMetadata>,
    /// Feature flags and the plans they belong to.
    pub features: Vec<TypeMetadata>,
    /// The roles a user or key can hold.
    pub permissions: Vec<TypeMetadata>,
    /// Cross-cutting concepts used for grouping and colouring.
    pub concepts: Vec<EntityMetadata>,
}

#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct TypeMetadata {
    /// Server-assigned unique identifier.
    pub id: &'static str,
    /// Human-facing name for this type.
    #[schema(required)]
    pub name: Option<&'static str>,
    /// Most providers supply a `&'static str` (wrapped as `Cow::Borrowed`); types
    /// whose description is composed at build time (e.g. credential transports
    /// derive theirs from a centralized integration description) supply
    /// `Cow::Owned`.
    #[schema(value_type = Option<String>, required)]
    pub description: Option<Cow<'static, str>>,
    /// Group this type is listed under.
    #[schema(required)]
    pub category: Option<&'static str>,
    /// Icon representing this type.
    #[schema(value_type = Option<String>, required)]
    pub icon: Option<Icon>,
    /// Colour representing this type.
    pub color: Color,
    /// Extra type-specific detail, shape depending on the registry it came from.
    #[schema(required)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct EntityMetadata {
    /// Server-assigned unique identifier.
    pub id: &'static str,
    /// Colour representing this entity type.
    pub color: Color,
    /// Icon representing this entity type.
    #[schema(value_type = String)]
    pub icon: Icon,
}

pub trait HasId {
    fn id(&self) -> &'static str;
}

pub trait MetadataProvider<T>: HasId {
    fn to_metadata(&self) -> T;
}

pub trait EntityMetadataProvider: MetadataProvider<EntityMetadata> {
    fn color(&self) -> Color;
    fn icon(&self) -> Icon;
}

pub trait TypeMetadataProvider: EntityMetadataProvider + MetadataProvider<TypeMetadata> {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str {
        ""
    }
    fn category(&self) -> &'static str {
        ""
    }
    fn metadata(&self) -> serde_json::Value {
        serde_json::json!({})
    }
}

impl<T> MetadataProvider<EntityMetadata> for T
where
    T: EntityMetadataProvider,
{
    fn to_metadata(&self) -> EntityMetadata {
        EntityMetadata {
            id: self.id(),
            color: self.color(),
            icon: self.icon(),
        }
    }
}

impl<T> MetadataProvider<TypeMetadata> for T
where
    T: TypeMetadataProvider,
{
    fn to_metadata(&self) -> TypeMetadata {
        let id = self.id();
        let name = self.name();
        let description = self.description();
        let category = self.category();
        let icon = self.icon();
        let color = self.color();
        let metadata = self.metadata();

        TypeMetadata {
            id,
            name: (!name.is_empty()).then_some(name),
            description: (!description.is_empty()).then_some(Cow::Borrowed(description)),
            category: (!category.is_empty()).then_some(category),
            icon: Some(icon),
            color,
            metadata: (!metadata.as_object().is_some_and(|obj| obj.is_empty())).then_some(metadata),
        }
    }
}

/// Pull the `{named}` slots out of a message template, in order of appearance.
///
/// Shared rather than duplicated: the error-code TypeScript generator needs the same parse to emit
/// each code's parameter type, and two extractors that disagree about what counts as a slot is
/// exactly the drift the warning-code test exists to catch.
pub fn extract_slots(template: &str) -> Vec<String> {
    let mut slots = Vec::new();
    let mut chars = template.chars().peekable();

    while let Some(c) = chars.next() {
        if c != '{' {
            continue;
        }
        let mut name = String::new();
        while let Some(&next) = chars.peek() {
            if next == '}' {
                chars.next();
                break;
            }
            name.push(chars.next().unwrap_or_default());
        }
        if !name.is_empty() {
            slots.push(name);
        }
    }

    slots
}
