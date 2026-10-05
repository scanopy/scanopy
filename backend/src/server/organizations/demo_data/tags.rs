//! Tags

use super::*;
use crate::server::shared::types::Icon;
use crate::server::tags::r#impl::base::{ExclusiveSet, TagIcon};

/// The set a demo tag belongs to.
enum Set {
    None,
    Application,
    Group(&'static str),
}

pub(super) fn generate_tags(organization_id: Uuid, now: DateTime<Utc>) -> Vec<Tag> {
    // (name, description, color, set, icon). Application tags draw the application icon, so
    // they carry none of their own.
    let tag_definitions: [(&str, &str, Color, Set, Option<Icon>); 14] = [
        (
            "Production",
            "Systems running in production",
            Color::Red,
            Set::Group("Environment"),
            Some(Icon::Rocket),
        ),
        (
            "Development",
            "Development and test systems",
            Color::Blue,
            Set::Group("Environment"),
            Some(Icon::FlaskConical),
        ),
        (
            "Critical",
            "Business-critical services",
            Color::Orange,
            Set::None,
            Some(Icon::TriangleAlert),
        ),
        (
            "Backup Target",
            "Backup destinations",
            Color::Green,
            Set::None,
            Some(Icon::Archive),
        ),
        (
            "Monitoring",
            "Monitoring infrastructure",
            Color::Purple,
            Set::Application,
            None,
        ),
        (
            "Database",
            "Database servers",
            Color::Cyan,
            Set::Application,
            None,
        ),
        (
            "Web Tier",
            "Web and application servers",
            Color::Teal,
            Set::Application,
            None,
        ),
        (
            "IoT Device",
            "Smart devices",
            Color::Yellow,
            Set::None,
            Some(Icon::Cpu),
        ),
        (
            "Needs Attention",
            "Requires admin review",
            Color::Rose,
            Set::None,
            Some(Icon::Bell),
        ),
        (
            "Managed Client",
            "Client-owned assets",
            Color::Indigo,
            Set::None,
            Some(Icon::Briefcase),
        ),
        (
            "DevOps Pipeline",
            "CI/CD and deployment tools",
            Color::Pink,
            Set::Application,
            None,
        ),
        (
            "Storage",
            "Storage and backup systems",
            Color::Lime,
            Set::Application,
            None,
        ),
        (
            "Messaging",
            "Message brokers and email",
            Color::Amber,
            Set::Application,
            None,
        ),
        (
            "Decommissioned",
            "Retired from service; kept for the record",
            Color::Gray,
            Set::Group("Lifecycle"),
            Some(Icon::ArchiveX),
        ),
    ];

    tag_definitions
        .iter()
        .map(|(name, description, color, set, icon)| Tag {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            base: TagBase {
                name: name.to_string(),
                description: Some(description.to_string()),
                color: *color,
                organization_id,
                exclusive_set: match set {
                    Set::None => None,
                    Set::Application => Some(ExclusiveSet::Application),
                    Set::Group(name) => Some(ExclusiveSet::Group {
                        name: name.to_string(),
                    }),
                },
                icon: icon.map(TagIcon),
            },
        })
        .collect()
}
