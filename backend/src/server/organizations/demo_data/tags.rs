//! Tags

use super::*;
use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::storage::seed_data::{
    create_starter_tags, hidden_tag, starter_custom_group_names, starter_group_description,
};
use crate::server::shared::types::Icon;
use crate::server::tags::groups::base::{TagGroup, TagGroupBase, TagGroupType};
use crate::server::tags::r#impl::base::TagIcon;
use std::collections::HashMap;

/// The tag group a demo tag belongs to.
enum Group {
    None,
    Application,
    BackupSchedule,
}

/// The demo's own group, limited to hosts: how often a storage host is backed up.
const BACKUP_SCHEDULE: &str = "Backup Schedule";

/// The demo's tag groups: the system groups, the starter groups every organization gets, and a
/// group limited to hosts.
pub(super) fn generate_tag_groups(organization_id: Uuid, now: DateTime<Utc>) -> Vec<TagGroup> {
    let system =
        TagGroupType::system().map(|t| TagGroupBase::new(organization_id, t.to_string(), t));
    let starter = starter_custom_group_names()
        .into_iter()
        .map(|name| TagGroupBase {
            description: starter_group_description(name).map(str::to_string),
            ..TagGroupBase::new(organization_id, name.to_string(), TagGroupType::Custom)
        });
    let backup_schedule = TagGroupBase {
        entity_types: Some(vec![EntityDiscriminants::Host]),
        description: Some("How often a storage host is backed up.".to_string()),
        ..TagGroupBase::new(
            organization_id,
            BACKUP_SCHEDULE.to_string(),
            TagGroupType::Custom,
        )
    };

    system
        .chain(starter)
        .chain(std::iter::once(backup_schedule))
        .map(|base| TagGroup {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base,
        })
        .collect()
}

pub(super) fn generate_tags(
    organization_id: Uuid,
    groups: &[TagGroup],
    now: DateTime<Utc>,
) -> Vec<Tag> {
    let group_id = |group: &Group| match group {
        Group::None => None,
        Group::Application => groups
            .iter()
            .find(|g| g.base.group_type == TagGroupType::Application)
            .map(|g| g.id),
        Group::BackupSchedule => groups
            .iter()
            .find(|g| g.base.name == BACKUP_SCHEDULE)
            .map(|g| g.id),
    };

    // (name, description, color, group, icon), in each group's order. Application tags draw the
    // application icon, so they carry none of their own.
    let tag_definitions: [(&str, &str, Color, Group, Option<Icon>); 14] = [
        (
            "Backup Target",
            "Backup destinations",
            Color::Green,
            Group::None,
            Some(Icon::Archive),
        ),
        (
            "Customer Portal",
            "Customer-facing ordering site",
            Color::Blue,
            Group::Application,
            None,
        ),
        (
            "Monitoring",
            "Metrics, logs, traces and uptime checks",
            Color::Purple,
            Group::Application,
            None,
        ),
        (
            "CI/CD",
            "Source control, builds and deployments",
            Color::Pink,
            Group::Application,
            None,
        ),
        (
            "Identity & Access",
            "Single sign-on and the team password vault",
            Color::Indigo,
            Group::Application,
            None,
        ),
        (
            "File Sharing",
            "Company file share",
            Color::Teal,
            Group::Application,
            None,
        ),
        (
            "Backups",
            "Nightly and weekly backups",
            Color::Lime,
            Group::Application,
            None,
        ),
        (
            "Wiki",
            "Internal documentation",
            Color::Cyan,
            Group::Application,
            None,
        ),
        (
            "Service Desk",
            "Support and IT tickets",
            Color::Amber,
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
            "Nightly",
            "Backed up every night",
            Color::Green,
            Group::BackupSchedule,
            Some(Icon::Moon),
        ),
        (
            "Weekly",
            "Backed up once a week",
            Color::Blue,
            Group::BackupSchedule,
            Some(Icon::CalendarDays),
        ),
    ];

    let mut positions: HashMap<Uuid, i32> = HashMap::new();
    let demo_tags: Vec<Tag> = tag_definitions
        .iter()
        .map(|(name, description, color, group, icon)| {
            let tag_group_id = group_id(group);
            // Only an ordered group numbers its tags; Application lists by name.
            let ordered = groups
                .iter()
                .any(|g| Some(g.id) == tag_group_id && g.base.ordered);
            let position = tag_group_id.filter(|_| ordered).map(|id| {
                let next = positions.entry(id).or_default();
                *next += 1;
                *next
            });
            Tag {
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
                    tag_group_id,
                    position,
                    icon: icon.map(TagIcon),
                    // The demo's own tags stand for tags a person added.
                    source: EntitySource::Manual,
                },
            }
        })
        .collect();

    // The tags every organization starts with, so the demo shows what a new org has.
    let starter_tags = create_starter_tags(organization_id, groups)
        .into_iter()
        .map(|tag| Tag {
            created_at: now,
            updated_at: now,
            valid_from: now,
            ..tag
        });

    // The Visibility group's Hidden tag, as every organization gets it.
    let hidden = groups
        .iter()
        .find(|g| g.base.group_type == TagGroupType::Visibility)
        .map(|g| Tag {
            created_at: now,
            updated_at: now,
            valid_from: now,
            ..hidden_tag(organization_id, g.id)
        });

    demo_tags
        .into_iter()
        .chain(starter_tags)
        .chain(hidden)
        .collect()
}
