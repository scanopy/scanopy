//! Tags

use super::*;
use crate::server::shared::storage::seed_data::create_status_tags;
use crate::server::shared::types::Icon;
use crate::server::tags::r#impl::base::{TagGroup, TagIcon};

/// The tag group a demo tag belongs to.
enum Group {
    None,
    Application,
    Named(&'static str),
}

pub(super) fn generate_tags(organization_id: Uuid, now: DateTime<Utc>) -> Vec<Tag> {
    // (name, description, color, group, icon). Application tags draw the application icon, so
    // they carry none of their own.
    let tag_definitions: [(&str, &str, Color, Group, Option<Icon>); 13] = [
        (
            "Production",
            "Systems running in production",
            Color::Red,
            Group::Named("Environment"),
            Some(Icon::Rocket),
        ),
        (
            "Development",
            "Development and test systems",
            Color::Blue,
            Group::Named("Environment"),
            Some(Icon::FlaskConical),
        ),
        (
            "Critical",
            "Business-critical services",
            Color::Orange,
            Group::None,
            Some(Icon::TriangleAlert),
        ),
        (
            "Backup Target",
            "Backup destinations",
            Color::Green,
            Group::None,
            Some(Icon::Archive),
        ),
        (
            "Monitoring",
            "Monitoring infrastructure",
            Color::Purple,
            Group::Application,
            None,
        ),
        (
            "Database",
            "Database servers",
            Color::Cyan,
            Group::Application,
            None,
        ),
        (
            "Web Tier",
            "Web and application servers",
            Color::Teal,
            Group::Application,
            None,
        ),
        (
            "IoT Device",
            "Smart devices",
            Color::Yellow,
            Group::None,
            Some(Icon::Cpu),
        ),
        (
            "Needs Attention",
            "Requires admin review",
            Color::Rose,
            Group::None,
            Some(Icon::Bell),
        ),
        (
            "Managed Client",
            "Client-owned assets",
            Color::Indigo,
            Group::None,
            Some(Icon::Briefcase),
        ),
        (
            "DevOps Pipeline",
            "CI/CD and deployment tools",
            Color::Pink,
            Group::Application,
            None,
        ),
        (
            "Storage",
            "Storage and backup systems",
            Color::Lime,
            Group::Application,
            None,
        ),
        (
            "Messaging",
            "Message brokers and email",
            Color::Amber,
            Group::Application,
            None,
        ),
    ];

    let demo_tags = tag_definitions
        .iter()
        .map(|(name, description, color, group, icon)| Tag {
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
                tag_group: match group {
                    Group::None => None,
                    Group::Application => Some(TagGroup::Application),
                    Group::Named(name) => Some(TagGroup::Named {
                        name: name.to_string(),
                    }),
                },
                icon: icon.map(TagIcon),
            },
        });

    // The Status tags every organization starts with, so the demo shows what a new org has.
    let status_tags = create_status_tags(organization_id)
        .into_iter()
        .map(|tag| Tag {
            created_at: now,
            updated_at: now,
            valid_from: now,
            ..tag
        });

    demo_tags.chain(status_tags).collect()
}
