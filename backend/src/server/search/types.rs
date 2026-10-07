use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::server::shared::entities::{Entity, EntityDiscriminants};

/// Query parameters for the global search.
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct GlobalSearchQuery {
    /// Free text. Case-insensitive substring match against each entity type's searchable fields.
    pub q: Option<String>,
    /// Only entities carrying every one of these tags. Repeat for several.
    pub tag_ids: Option<Vec<Uuid>>,
}

/// One entity type's matches.
#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct GlobalSearchGroup {
    pub entity_type: EntityDiscriminants,
    /// Each match, tagged with its entity type.
    pub items: Vec<Entity>,
}

/// Matches grouped by entity type, in registry order. Types with no matches are left out.
#[derive(Serialize, Debug, Clone, Default, ToSchema)]
pub struct GlobalSearchResponse {
    pub groups: Vec<GlobalSearchGroup>,
}
