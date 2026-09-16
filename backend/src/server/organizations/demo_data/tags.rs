//! Tags

use super::*;

pub(super) fn generate_tags(organization_id: Uuid, now: DateTime<Utc>) -> Vec<Tag> {
    // (name, description, color, is_application)
    let tag_definitions: [(&str, &str, Color, bool); 13] = [
        (
            "Production",
            "Systems running in production",
            Color::Red,
            false,
        ),
        (
            "Development",
            "Development and test systems",
            Color::Blue,
            false,
        ),
        (
            "Critical",
            "Business-critical services",
            Color::Orange,
            false,
        ),
        ("Backup Target", "Backup destinations", Color::Green, false),
        (
            "Monitoring",
            "Monitoring infrastructure",
            Color::Purple,
            true,
        ),
        ("Database", "Database servers", Color::Cyan, true),
        ("Web Tier", "Web and application servers", Color::Teal, true),
        ("IoT Device", "Smart devices", Color::Yellow, false),
        (
            "Needs Attention",
            "Requires admin review",
            Color::Rose,
            false,
        ),
        (
            "Managed Client",
            "Client-owned assets",
            Color::Indigo,
            false,
        ),
        (
            "DevOps Pipeline",
            "CI/CD and deployment tools",
            Color::Pink,
            true,
        ),
        ("Storage", "Storage and backup systems", Color::Lime, true),
        ("Messaging", "Message brokers and email", Color::Amber, true),
    ];

    tag_definitions
        .iter()
        .map(|(name, description, color, is_app)| Tag {
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
                is_application: *is_app,
            },
        })
        .collect()
}
