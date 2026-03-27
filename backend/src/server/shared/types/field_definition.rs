use serde::Serialize;
use utoipa::ToSchema;

/// Field types for dynamic form generation
#[derive(Serialize, Debug, Clone, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    String,
    Number,
    Boolean,
    Select,
}

/// Definition of a form field for dynamic UI rendering.
/// Used by fixture generation to produce JSON consumed by the frontend.
#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct FieldDefinition {
    pub id: &'static str,
    pub label: &'static str,
    pub field_type: FieldType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<&'static str>,
    pub secret: bool,
    pub optional: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help_text: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<FieldOption>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<&'static str>,
}

/// An option for select-type fields
#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct FieldOption {
    pub label: &'static str,
    pub value: &'static str,
}
