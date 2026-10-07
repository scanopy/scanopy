use std::fmt::Display;

use crate::server::shared::{
    entities::ChangeTriggersTopologyStaleness,
    types::{Color, Icon, api::deserialize_empty_string_as_none},
};
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// A group of tags of which an entity may hold at most one.
///
/// Assigning a tag from a group replaces whichever tag of the same group the entity already holds.
/// `Application` is the built-in group: its tags drive the application view, which can place an
/// entity in one application only. `Named` groups are named by the organization (a status, an
/// environment, a tier) and exist for as long as a tag carries the name.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, ToSchema)]
#[serde(tag = "type")]
pub enum TagGroup {
    /// The built-in group of application tags.
    #[schema(title = "Application")]
    Application,
    /// A group the organization named.
    #[schema(title = "Named")]
    Named {
        /// The group's name, shared by every tag in it.
        #[serde(deserialize_with = "deserialize_trimmed")]
        name: String,
    },
}

impl Display for TagGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Application => write!(f, "Application"),
            Self::Named { name } => write!(f, "{name}"),
        }
    }
}

fn deserialize_trimmed<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(String::deserialize(d)?.trim().to_string())
}

fn validate_tag_group(group: &TagGroup) -> Result<(), validator::ValidationError> {
    match group {
        TagGroup::Named { name } if name.is_empty() || name.chars().count() > 100 => {
            Err(validator::ValidationError::new("tag_group_name")
                .with_message("Tag group name must be between 1 and 100 characters".into()))
        }
        _ => Ok(()),
    }
}

/// A lucide icon a person chose for a tag.
///
/// A newtype because `lucide_icons::Icon` derives neither `Eq` nor `Hash`, which every entity
/// base needs. The icon is a fieldless enum, so its discriminant is a complete identity.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
#[schema(value_type = String)]
pub struct TagIcon(pub Icon);

impl TagIcon {
    /// Every icon in the linked lucide build, ordered by name.
    ///
    /// The crate lists its icons only as font glyphs, which all sit in the Private Use Area, so
    /// walking that block finds each one exactly once.
    pub fn all() -> impl Iterator<Item = TagIcon> {
        let mut icons: Vec<TagIcon> = ('\u{e000}'..='\u{f8ff}')
            .filter_map(|glyph| Icon::try_from(glyph).ok().map(TagIcon))
            .collect();
        icons.sort_by_cached_key(|icon| icon.to_string());
        icons.into_iter()
    }
}

impl PartialEq for TagIcon {
    fn eq(&self, other: &Self) -> bool {
        self.0 as u32 == other.0 as u32
    }
}

impl Eq for TagIcon {}

impl std::hash::Hash for TagIcon {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (self.0 as u32).hash(state);
    }
}

impl Display for TagIcon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for TagIcon {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Icon::try_from(s)
            .map(TagIcon)
            .map_err(|_| anyhow::anyhow!("Unknown icon {s}"))
    }
}

fn validate_tag_base(base: &TagBase) -> Result<(), validator::ValidationError> {
    if base.is_application() && base.icon.is_some() {
        return Err(
            validator::ValidationError::new("application_tag_icon").with_message(
                "Application tags use the application icon and cannot set their own".into(),
            ),
        );
    }
    Ok(())
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize, Eq, PartialEq, Hash, ToSchema)]
#[validate(schema(function = "validate_tag_base"))]
pub struct TagBase {
    /// Human-facing name for this tag.
    #[validate(length(
        min = 1,
        max = 100,
        message = "Tag name must be between 1 and 100 characters"
    ))]
    pub name: String,
    /// Free-text notes about the tag.
    #[serde(deserialize_with = "deserialize_empty_string_as_none")]
    pub description: Option<String>,
    /// Colour the tag is drawn in.
    pub color: Color,
    /// The organization that owns this record.
    pub organization_id: Uuid,
    /// The group this tag belongs to, if any. An entity holds at most one tag of a group.
    #[serde(default)]
    #[validate(custom(function = "validate_tag_group"))]
    pub tag_group: Option<TagGroup>,
    /// Icon drawn on the tag. Application tags always use the application icon.
    #[serde(default)]
    pub icon: Option<TagIcon>,
}

impl TagBase {
    /// Whether this tag groups an application, so it drives the application view.
    pub fn is_application(&self) -> bool {
        matches!(self.tag_group, Some(TagGroup::Application))
    }
}

impl Default for TagBase {
    fn default() -> Self {
        Self {
            name: "New Tag".to_string(),
            description: None,
            color: Color::Yellow,
            organization_id: Uuid::nil(),
            tag_group: None,
            icon: None,
        }
    }
}

#[derive(
    Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, Default, ToSchema, Validate,
)]
#[schema(example = crate::server::shared::types::examples::tag)]
pub struct Tag {
    /// Server-assigned unique identifier.
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    /// When this record was first created.
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    /// When this record was last modified.
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    /// Start of the interval this revision was current for (SCD2 history).
    #[serde(default)]
    #[schema(read_only)]
    pub valid_from: DateTime<Utc>,
    /// End of the interval this revision was current for. `null` while it is the live revision.
    #[serde(default)]
    #[schema(read_only)]
    pub valid_to: Option<DateTime<Utc>>,
    /// Stable identifier shared by every revision of the same entity across its history.
    #[serde(default)]
    #[schema(read_only)]
    pub lineage_id: Option<Uuid>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: TagBase,
}

impl Tag {
    /// Whether this tag groups an application, so it drives the application view.
    pub fn is_application(&self) -> bool {
        self.base.is_application()
    }
}

impl ChangeTriggersTopologyStaleness<Tag> for Tag {
    /// Always returns true — the actual determination of whether a Tag change
    /// should mark a topology stale lives downstream in the topology subscriber,
    /// which has the cross-service access needed to check if the tag is an
    /// application tag or is referenced by a ByTag element rule.
    /// See `TopologyService::tag_affects_any_topology`.
    fn triggers_staleness(&self, _other: Option<Tag>) -> bool {
        true
    }
}

impl Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Tag {}: {}", self.base.name, self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base(tag_group: Option<TagGroup>, icon: Option<Icon>) -> TagBase {
        TagBase {
            tag_group,
            icon: icon.map(TagIcon),
            ..Default::default()
        }
    }

    #[test]
    fn an_application_tag_cannot_choose_its_own_icon() {
        assert!(
            base(Some(TagGroup::Application), Some(Icon::Rocket))
                .validate()
                .is_err()
        );
        assert!(base(Some(TagGroup::Application), None).validate().is_ok());
        let status = Some(TagGroup::Named {
            name: "Status".to_string(),
        });
        assert!(base(status, Some(Icon::Rocket)).validate().is_ok());
    }

    #[test]
    fn a_group_name_is_trimmed_and_a_blank_one_refused() {
        let tag: TagBase = serde_json::from_value(serde_json::json!({
            "name": "Old printer",
            "description": null,
            "color": "Gray",
            "organization_id": Uuid::nil(),
            "tag_group": { "type": "Named", "name": "  Status " },
        }))
        .unwrap();
        assert_eq!(
            tag.tag_group,
            Some(TagGroup::Named {
                name: "Status".to_string()
            })
        );

        let blank = base(
            Some(TagGroup::Named {
                name: String::new(),
            }),
            None,
        );
        assert!(blank.validate().is_err());
    }

    #[test]
    fn an_unknown_icon_name_is_refused() {
        assert!(serde_json::from_value::<TagIcon>(serde_json::json!("not-an-icon")).is_err());
        assert!(serde_json::from_value::<TagIcon>(serde_json::json!("archive-x")).is_ok());
    }

    /// The picker offers exactly what `all` lists, so every entry must round-trip through the
    /// API's name for it, and none may repeat.
    #[test]
    fn every_listed_icon_round_trips_through_its_name() {
        let icons: Vec<TagIcon> = TagIcon::all().collect();
        assert!(icons.len() > 1000, "expected the full lucide set");
        let names: std::collections::HashSet<String> =
            icons.iter().map(|icon| icon.to_string()).collect();
        assert_eq!(names.len(), icons.len());
        for icon in icons {
            assert_eq!(icon.to_string().parse::<TagIcon>().unwrap(), icon);
        }
    }
}
