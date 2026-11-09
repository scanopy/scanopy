use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::{
    shared::{
        entities::EntityDiscriminants,
        product_area::ProductArea,
        storage::{
            snapshot::Snapshotable,
            traits::{Entity, SqlValue, Storable},
        },
        types::entities::EntitySource,
    },
    tags::r#impl::base::{Tag, TagBase},
};

impl Storable for Tag {
    type BaseData = TagBase;

    fn table_name() -> &'static str {
        "tags"
    }

    fn search_predicates() -> &'static [&'static str] {
        &["tags.name ILIKE {}", "tags.description ILIKE {}"]
    }

    const HAS_SCD2: bool = true;

    fn is_live_row(&self) -> bool {
        self.valid_to.is_none()
    }

    fn new(base: Self::BaseData) -> Self {
        let now = crate::server::shared::clock::now();

        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            base,
        }
    }

    fn get_base(&self) -> Self::BaseData {
        self.base.clone()
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>), anyhow::Error> {
        let Self {
            id,
            created_at,
            updated_at,
            valid_from,
            valid_to,
            lineage_id,
            base:
                Self::BaseData {
                    name,
                    description,
                    color,
                    organization_id,
                    tag_group_id,
                    position,
                    icon,
                    source,
                },
        } = self.clone();

        Ok((
            vec![
                "id",
                "name",
                "description",
                "color",
                "organization_id",
                "tag_group_id",
                "position",
                "icon",
                "source",
                "created_at",
                "updated_at",
                "valid_from",
                "valid_to",
                "lineage_id",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::String(name),
                SqlValue::OptionalString(description),
                SqlValue::String(color.to_string()),
                SqlValue::Uuid(organization_id),
                SqlValue::OptionalUuid(tag_group_id),
                SqlValue::OptionalI32(position),
                SqlValue::OptionalString(icon.map(|icon| icon.to_string())),
                SqlValue::EntitySource(source),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
                SqlValue::Timestamp(valid_from),
                SqlValue::OptionTimestamp(valid_to),
                SqlValue::OptionalUuid(lineage_id),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        Ok(Tag {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            valid_from: row.get("valid_from"),
            valid_to: row.get("valid_to"),
            lineage_id: row.get("lineage_id"),
            base: TagBase {
                name: row.get("name"),
                description: row.get("description"),
                organization_id: row.get("organization_id"),
                color: row.get::<String, _>("color").parse().unwrap_or_default(),
                tag_group_id: row.get("tag_group_id"),
                position: row.get("position"),
                // An icon name the linked lucide build no longer knows reads as no icon rather
                // than failing the whole tag list.
                icon: row
                    .get::<Option<String>, _>("icon")
                    .and_then(|name| name.parse().ok()),
                source: serde_json::from_value(row.get::<serde_json::Value, _>("source"))
                    .map_err(|e| anyhow::anyhow!("Failed to deserialize source: {}", e))?,
            },
        })
    }
}

impl Snapshotable for Tag {
    fn id_value(&self) -> Uuid {
        self.id
    }
    fn set_id_value(&mut self, id: Uuid) {
        self.id = id;
    }
    fn valid_from(&self) -> DateTime<Utc> {
        self.valid_from
    }
    fn valid_to(&self) -> Option<DateTime<Utc>> {
        self.valid_to
    }
    fn lineage_id(&self) -> Option<Uuid> {
        self.lineage_id
    }
    fn set_valid_from(&mut self, t: DateTime<Utc>) {
        self.valid_from = t;
    }
    fn set_valid_to(&mut self, t: Option<DateTime<Utc>>) {
        self.valid_to = t;
    }
    fn set_lineage_id(&mut self, id: Option<Uuid>) {
        self.lineage_id = id;
    }
    // Tag is org-scoped — no within-tracked-set FKs to remap.
    // Lifecycle: per-action close-and-clone on rename via TagService::update.
}

impl Entity for Tag {
    fn id(&self) -> Uuid {
        self.id
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    fn set_id(&mut self, id: Uuid) {
        self.id = id;
    }

    fn set_created_at(&mut self, time: DateTime<Utc>) {
        self.created_at = time;
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Tag
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Tag";
    const ENTITY_NAME_PLURAL: &'static str = "Tags";
    const ENTITY_DESCRIPTION: &'static str =
        "Custom tags for categorization. Apply labels to entities for filtering and organization.";

    fn product_area() -> ProductArea {
        ProductArea::Platform
    }

    fn site_id(&self) -> Option<Uuid> {
        None
    }

    fn organization_id(&self) -> Option<Uuid> {
        Some(self.base.organization_id)
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    fn set_updated_at(&mut self, time: DateTime<Utc>) {
        self.updated_at = time;
    }

    fn set_source(&mut self, source: EntitySource) {
        self.base.source = source;
    }
}

#[cfg(test)]
mod tests {
    use crate::server::shared::storage::migration_runner::apply_migrations;
    use crate::tests::setup_test_db;
    use sqlx::{PgPool, Row};
    use std::path::Path;
    use uuid::Uuid;

    const MIGRATIONS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/migrations");
    const TAG_GROUPS: &str = "20261009120100";

    /// A copy of the migrations directory holding only the migrations before `version`.
    fn migrations_before(version: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for entry in std::fs::read_dir(MIGRATIONS_DIR).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_str().unwrap().to_string();
            if name.ends_with(".sql") && name.as_str() < version {
                std::fs::copy(&path, dir.path().join(&name)).unwrap();
            }
        }
        dir
    }

    async fn insert_tag(
        pool: &PgPool,
        organization_id: Uuid,
        name: &str,
        is_application: bool,
        tag_group: Option<&str>,
        valid_to: Option<&str>,
        snapshot_id: Option<Uuid>,
    ) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO tags (id, organization_id, name, color, created_at, updated_at, \
             is_application, tag_group, valid_to, snapshot_id) \
             VALUES ($1, $2, $3, 'Gray', now(), now(), $4, $5, $6::timestamptz, $7)",
        )
        .bind(id)
        .bind(organization_id)
        .bind(name)
        .bind(is_application)
        .bind(tag_group)
        .bind(valid_to)
        .bind(snapshot_id)
        .execute(pool)
        .await
        .unwrap();
        id
    }

    async fn assign(pool: &PgPool, entity_id: Uuid, tag_id: Uuid, valid_from: &str) -> Uuid {
        sqlx::query(
            "INSERT INTO entity_tags (entity_id, entity_type, tag_id, valid_from) \
             VALUES ($1, '\"Host\"', $2, $3::timestamptz) RETURNING id",
        )
        .bind(entity_id)
        .bind(tag_id)
        .bind(valid_from)
        .fetch_one(pool)
        .await
        .unwrap()
        .get("id")
    }

    /// The name and type of the group a tag row points at, and its position.
    async fn group_of(pool: &PgPool, id: Uuid) -> (Option<String>, Option<String>, Option<i32>) {
        let row = sqlx::query(
            "SELECT g.name, g.group_type, t.position FROM tags t \
             LEFT JOIN tag_groups g ON g.id = t.tag_group_id WHERE t.id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap();
        (row.get("name"), row.get("group_type"), row.get("position"))
    }

    async fn tag_group_of(pool: &PgPool, id: Uuid) -> Option<String> {
        group_of(pool, id).await.0
    }

    #[tokio::test]
    async fn tag_groups_migration_carries_every_group_forward() {
        let (pool, _url, _container) = setup_test_db().await;
        let before = migrations_before(TAG_GROUPS);
        apply_migrations(&pool, before.path()).await.unwrap();

        let org: Uuid = sqlx::query(
            "INSERT INTO organizations (id, name, plan) VALUES ($1, 'Acme', '{}') RETURNING id",
        )
        .bind(Uuid::new_v4())
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("id");
        let site: Uuid = sqlx::query(
            "INSERT INTO sites (id, name, organization_id, created_at, updated_at) \
             VALUES ($1, 'HQ', $2, now(), now()) RETURNING id",
        )
        .bind(Uuid::new_v4())
        .bind(org)
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("id");
        let snapshot: Uuid = sqlx::query(
            "INSERT INTO snapshots (id, site_id, taken_at) VALUES ($1, $2, now()) RETURNING id",
        )
        .bind(Uuid::new_v4())
        .bind(site)
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("id");

        let app_live = insert_tag(&pool, org, "Web", true, None, None, None).await;
        let app_closed = insert_tag(
            &pool,
            org,
            "Web",
            true,
            None,
            Some("2026-01-01T00:00:00Z"),
            None,
        )
        .await;
        let app_snapshot = insert_tag(
            &pool,
            org,
            "Web (snapshot)",
            true,
            None,
            None,
            Some(snapshot),
        )
        .await;
        let status = insert_tag(&pool, org, "Active", false, Some("Status"), None, None).await;
        let shouted = insert_tag(&pool, org, "Broken", false, Some("STATUS"), None, None).await;
        let custom_app =
            insert_tag(&pool, org, "Mine", false, Some("application"), None, None).await;
        let environment =
            insert_tag(&pool, org, "Prod", false, Some("Environment"), None, None).await;
        let planned = insert_tag(&pool, org, "Planned", false, Some("Status"), None, None).await;
        // A group name only a closed row still carries.
        let retired_group = insert_tag(
            &pool,
            org,
            "Old",
            false,
            Some("Retired Group"),
            Some("2026-01-01T00:00:00Z"),
            None,
        )
        .await;
        let ungrouped = insert_tag(&pool, org, "Loose", false, None, None, None).await;

        let empty_org: Uuid = sqlx::query(
            "INSERT INTO organizations (id, name, plan) VALUES ($1, 'Empty', '{}') RETURNING id",
        )
        .bind(Uuid::new_v4())
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("id");

        // A host holding a tag of each Status spelling: after the fold it may keep only one.
        let host = Uuid::new_v4();
        let older = assign(&pool, host, status, "2026-01-01T00:00:00Z").await;
        let newer = assign(&pool, host, shouted, "2026-02-01T00:00:00Z").await;

        // A daemon holding a Status tag, live and in its history: Status no longer applies to it.
        let daemon = Uuid::new_v4();
        for valid_to in [None, Some("2026-03-01T00:00:00Z")] {
            sqlx::query(
                "INSERT INTO entity_tags (entity_id, entity_type, tag_id, valid_to) \
                 VALUES ($1, '\"Daemon\"', $2, $3::timestamptz)",
            )
            .bind(daemon)
            .bind(planned)
            .bind(valid_to)
            .execute(&pool)
            .await
            .unwrap();
        }

        // Hosts hidden in each kind of row, and one shown.
        let insert_host = |hidden: bool, valid_to: Option<&'static str>, snapshot: Option<Uuid>| {
            let pool = pool.clone();
            async move {
                let id = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO hosts (id, site_id, name, source, created_at, updated_at, \
                     hidden, valid_to, snapshot_id) \
                     VALUES ($1, $2, 'h', '{}', now(), now(), $3, $4::timestamptz, $5)",
                )
                .bind(id)
                .bind(site)
                .bind(hidden)
                .bind(valid_to)
                .bind(snapshot)
                .execute(&pool)
                .await
                .unwrap();
                id
            }
        };
        let hidden_live = insert_host(true, None, None).await;
        let hidden_closed = insert_host(true, Some("2026-01-01T00:00:00Z"), None).await;
        let hidden_snapshot = insert_host(true, Some("2026-01-01T00:00:00Z"), Some(snapshot)).await;
        let shown = insert_host(false, None, None).await;

        apply_migrations(&pool, Path::new(MIGRATIONS_DIR))
            .await
            .unwrap();

        for id in [app_live, app_closed, app_snapshot] {
            assert_eq!(
                tag_group_of(&pool, id).await.as_deref(),
                Some("Application")
            );
        }
        assert_eq!(tag_group_of(&pool, status).await.as_deref(), Some("Status"));
        assert_eq!(
            tag_group_of(&pool, shouted).await.as_deref(),
            Some("Status")
        );
        assert_eq!(
            tag_group_of(&pool, custom_app).await.as_deref(),
            Some("application (custom)")
        );
        assert_eq!(
            tag_group_of(&pool, environment).await.as_deref(),
            Some("Environment")
        );
        assert_eq!(
            tag_group_of(&pool, retired_group).await.as_deref(),
            Some("Retired Group")
        );
        assert_eq!(group_of(&pool, ungrouped).await, (None, None, None));

        assert_eq!(
            group_of(&pool, app_live).await.1.as_deref(),
            Some("Application")
        );
        assert_eq!(
            group_of(&pool, environment).await.1.as_deref(),
            Some("Custom")
        );
        // Starter order first, then by name; closed and snapshot rows carry no position.
        assert_eq!(group_of(&pool, planned).await.2, Some(1));
        assert_eq!(group_of(&pool, status).await.2, Some(2));
        assert_eq!(group_of(&pool, shouted).await.2, Some(3));
        assert_eq!(group_of(&pool, app_closed).await.2, None);
        // Applications have no order.
        assert_eq!(group_of(&pool, app_live).await.2, None);
        let application_ordered: bool = sqlx::query(
            "SELECT ordered FROM tag_groups WHERE organization_id = $1 AND group_type = 'Application'",
        )
        .bind(org)
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("ordered");
        assert!(!application_ordered);
        assert_eq!(group_of(&pool, app_snapshot).await.2, None);

        let system_groups: Vec<String> = sqlx::query(
            "SELECT group_type FROM tag_groups WHERE organization_id = $1 ORDER BY group_type",
        )
        .bind(empty_org)
        .fetch_all(&pool)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get("group_type"))
        .collect();
        assert_eq!(system_groups, vec!["Application", "Status", "Visibility"]);

        // Every hidden host row holds the Hidden tag over the same span, in the same snapshot.
        let hidden: Vec<(Uuid, Option<chrono::DateTime<chrono::Utc>>, Option<Uuid>)> =
            sqlx::query(
                "SELECT et.entity_id, et.valid_to, et.snapshot_id FROM entity_tags et \
                 JOIN tags t ON t.id = et.tag_id JOIN tag_groups g ON g.id = t.tag_group_id \
                 WHERE g.group_type = 'Visibility' AND t.name = 'Hidden' \
                   AND et.entity_type = '\"Host\"' ORDER BY et.valid_to NULLS FIRST, et.snapshot_id NULLS FIRST",
            )
            .fetch_all(&pool)
            .await
            .unwrap()
            .into_iter()
            .map(|row| (row.get("entity_id"), row.get("valid_to"), row.get("snapshot_id")))
            .collect();
        let hidden_ids: Vec<Uuid> = hidden.iter().map(|(id, _, _)| *id).collect();
        assert_eq!(
            hidden_ids,
            vec![hidden_live, hidden_closed, hidden_snapshot]
        );
        assert!(hidden[0].1.is_none() && hidden[1].1.is_some());
        assert_eq!(hidden[2].2, Some(snapshot));
        assert!(!hidden_ids.contains(&shown));

        let daemon_status: i64 =
            sqlx::query("SELECT count(*) AS n FROM entity_tags WHERE entity_id = $1")
                .bind(daemon)
                .fetch_one(&pool)
                .await
                .unwrap()
                .get("n");
        assert_eq!(
            daemon_status, 0,
            "Status assignments on a daemon are deleted in every version"
        );

        let live: Vec<Uuid> =
            sqlx::query("SELECT id FROM entity_tags WHERE entity_id = $1 AND valid_to IS NULL")
                .bind(host)
                .fetch_all(&pool)
                .await
                .unwrap()
                .into_iter()
                .map(|row| row.get("id"))
                .collect();
        assert_eq!(live, vec![newer]);
        let closed: Option<chrono::DateTime<chrono::Utc>> =
            sqlx::query("SELECT valid_to FROM entity_tags WHERE id = $1")
                .bind(older)
                .fetch_one(&pool)
                .await
                .unwrap()
                .get("valid_to");
        assert!(
            closed.is_some(),
            "the older assignment is closed, not deleted"
        );

        let columns_left: bool = sqlx::query(
            "SELECT EXISTS (SELECT 1 FROM information_schema.columns \
             WHERE (table_name = 'tags' AND column_name IN ('is_application', 'tag_group')) \
                OR (table_name = 'hosts' AND column_name = 'hidden')) AS present",
        )
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("present");
        assert!(!columns_left);
    }
}
