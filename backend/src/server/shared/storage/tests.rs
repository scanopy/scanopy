use crate::server::{
    bindings::r#impl::base::Binding,
    credentials::r#impl::base::Credential,
    daemon_api_keys::r#impl::base::DaemonApiKey,
    daemons::r#impl::base::Daemon,
    dependencies::{dependency_members::DependencyMemberRecord, r#impl::base::Dependency},
    discovery::r#impl::base::Discovery,
    hosts::r#impl::base::Host,
    interface_neighbors::r#impl::base::{
        InterfaceNeighborCandidate, InterfaceNeighborHost, InterfaceNeighborInterface,
    },
    interfaces::r#impl::base::Interface,
    invites::r#impl::base::Invite,
    ip_addresses::r#impl::base::IPAddress,
    networks::r#impl::Network,
    organizations::r#impl::base::Organization,
    ports::r#impl::base::Port,
    services::r#impl::base::Service,
    shared::storage::traits::Storable,
    shares::r#impl::base::Share,
    snapshots::types::base::Snapshot,
    subnets::r#impl::base::Subnet,
    tags::entity_tags::EntityTag,
    tags::r#impl::base::Tag,
    topology::types::base::Topology,
    user_api_keys::r#impl::base::UserApiKey,
    users::r#impl::base::User,
    vlans::r#impl::{base::Vlan, subnet_vlans::SubnetVlanRecord},
};
use sqlx::postgres::PgRow;
use std::collections::HashMap;

// Type alias for the deserialization function
#[allow(dead_code)]
type DeserializeFn = Box<dyn Fn(&PgRow) -> Result<(), anyhow::Error> + Send + Sync>;

#[allow(dead_code)]
const TABLES_WITHOUT_ENTITIES: [&str; 2] = ["user_network_access", "user_api_key_network_access"];

// Mapping from table name to deserialization function
#[allow(dead_code)]
fn get_entity_deserializers() -> HashMap<&'static str, DeserializeFn> {
    let mut map: HashMap<&'static str, DeserializeFn> = HashMap::new();

    map.insert(
        DaemonApiKey::table_name(),
        Box::new(|row| {
            DaemonApiKey::from_row(row)?;
            Ok(())
        }),
    );

    map.insert(
        Daemon::table_name(),
        Box::new(|row| {
            Daemon::from_row(row)?;
            Ok(())
        }),
    );

    map.insert(
        Discovery::table_name(),
        Box::new(|row| {
            Discovery::from_row(row)?;
            Ok(())
        }),
    );

    map.insert(
        Dependency::table_name(),
        Box::new(|row| {
            Dependency::from_row(row)?;
            Ok(())
        }),
    );

    map.insert(
        Host::table_name(),
        Box::new(|row| {
            Host::from_row(row)?;
            Ok(())
        }),
    );

    map.insert(
        Network::table_name(),
        Box::new(|row| {
            Network::from_row(row)?;
            Ok(())
        }),
    );

    map.insert(
        Organization::table_name(),
        Box::new(|row| {
            Organization::from_row(row)?;
            Ok(())
        }),
    );

    map.insert(
        Service::table_name(),
        Box::new(|row| {
            Service::from_row(row)?;
            Ok(())
        }),
    );

    map.insert(
        Subnet::table_name(),
        Box::new(|row| {
            Subnet::from_row(row)?;
            Ok(())
        }),
    );

    map.insert(
        User::table_name(),
        Box::new(|row| {
            User::from_row(row)?;
            Ok(())
        }),
    );

    map.insert(
        Topology::table_name(),
        Box::new(|row| {
            Topology::from_row(row)?;
            Ok(())
        }),
    );

    map.insert(
        Snapshot::table_name(),
        Box::new(|row| {
            Snapshot::from_row(row)?;
            Ok(())
        }),
    );

    map.insert(
        Tag::table_name(),
        Box::new(|row| {
            Tag::from_row(row)?;
            Ok(())
        }),
    );

    map.insert(
        Invite::table_name(),
        Box::new(|row| {
            Invite::from_row(row)?;
            Ok(())
        }),
    );

    map.insert(
        Share::table_name(),
        Box::new(|row| {
            Share::from_row(row)?;
            Ok(())
        }),
    );

    map.insert(
        IPAddress::table_name(),
        Box::new(|row| {
            IPAddress::from_row(row)?;
            Ok(())
        }),
    );

    map.insert(
        Port::table_name(),
        Box::new(|row| {
            Port::from_row(row)?;
            Ok(())
        }),
    );

    map.insert(
        Binding::table_name(),
        Box::new(|row| {
            Binding::from_row(row)?;
            Ok(())
        }),
    );

    map.insert(
        DependencyMemberRecord::table_name(),
        Box::new(|row| {
            DependencyMemberRecord::from_row(row)?;
            Ok(())
        }),
    );

    map.insert(
        UserApiKey::table_name(),
        Box::new(|row| {
            UserApiKey::from_row(row)?;
            Ok(())
        }),
    );

    map.insert(
        EntityTag::table_name(),
        Box::new(|row| {
            EntityTag::from_row(row)?;
            Ok(())
        }),
    );

    // snmp_credentials table was dropped by universal_credentials migration
    // SnmpCredential deserializer is no longer needed

    map.insert(
        Credential::table_name(),
        Box::new(|row| {
            Credential::from_row(row)?;
            Ok(())
        }),
    );

    // Junction tables for multi-credential support — no entity struct, just verify readable
    map.insert("host_credentials", Box::new(|_row| Ok(())));

    map.insert("network_credentials", Box::new(|_row| Ok(())));

    // Daemon ↔ interfaced-subnet junction — no entity struct, just verify readable.
    map.insert("daemon_interfaced_subnets", Box::new(|_row| Ok(())));

    map.insert(
        Interface::table_name(),
        Box::new(|row| {
            Interface::from_row(row)?;
            Ok(())
        }),
    );

    map.insert(
        InterfaceNeighborCandidate::table_name(),
        Box::new(|row| {
            InterfaceNeighborCandidate::from_row(row)?;
            Ok(())
        }),
    );

    map.insert(
        InterfaceNeighborInterface::table_name(),
        Box::new(|row| {
            InterfaceNeighborInterface::from_row(row)?;
            Ok(())
        }),
    );

    map.insert(
        InterfaceNeighborHost::table_name(),
        Box::new(|row| {
            InterfaceNeighborHost::from_row(row)?;
            Ok(())
        }),
    );

    map.insert(
        Vlan::table_name(),
        Box::new(|row| {
            Vlan::from_row(row)?;
            Ok(())
        }),
    );

    map.insert(
        SubnetVlanRecord::table_name(),
        Box::new(|row| {
            SubnetVlanRecord::from_row(row)?;
            Ok(())
        }),
    );

    map
}

#[tokio::test]
pub async fn test_all_tables_have_entity_mapping() {
    use crate::tests::setup_test_db;

    let (pool, _database_url, _container) = setup_test_db().await;

    // Apply migrations to create the schema
    crate::server::shared::storage::migration_runner::apply_embedded_migrations(&pool)
        .await
        .expect("Failed to run migrations");

    // Get all tables from information_schema
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT table_name FROM information_schema.tables
         WHERE table_schema = 'public'
         AND table_type = 'BASE TABLE'
         AND table_name != '_sqlx_migrations'",
    )
    .fetch_all(&pool)
    .await
    .expect("Failed to fetch table names");

    let deserializers = get_entity_deserializers();

    println!("Verifying entity mappings for all tables...");

    let mut missing_mappings = Vec::new();
    for table in &tables {
        if !deserializers.contains_key(table.as_str())
            && !TABLES_WITHOUT_ENTITIES.contains(&table.as_str())
        {
            missing_mappings.push(table.clone());
        }
    }

    if !missing_mappings.is_empty() {
        panic!(
            "The following tables are missing entity mappings in get_entity_deserializers():\n  - {}\n\
             Please add them to the registry.",
            missing_mappings.join("\n  - ")
        );
    }

    println!("✓ All {} tables have entity mappings", tables.len());
}

#[tokio::test]
pub async fn test_database_schema_backward_compatibility() {
    use crate::tests::SERVER_DB_FIXTURE;
    use crate::tests::setup_test_db;
    use std::path::Path;

    let db_path = Path::new(SERVER_DB_FIXTURE);

    if db_path.exists() {
        use std::process::Command;

        println!("Testing backward compatibility with database from latest release");

        let (pool, database_url, _container) = setup_test_db().await;

        let url = url::Url::parse(&database_url).unwrap();
        let host = url.host_str().unwrap();
        let port = url.port().unwrap();
        let database = url.path().trim_start_matches('/');

        pool.close().await;

        let output = Command::new("psql")
            .arg("-h")
            .arg(host)
            .arg("-p")
            .arg(port.to_string())
            .arg("-U")
            .arg("postgres")
            .arg("-d")
            .arg(database)
            .arg("-f")
            .arg(db_path)
            .env("PGPASSWORD", "password")
            .output()
            .expect("Failed to execute psql - ensure it's installed");

        assert!(
            output.status.success(),
            "Failed to restore database:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );

        println!("Successfully restored database from fixture");

        let pool = sqlx::PgPool::connect(&database_url).await.unwrap();

        // Verify tables exist using the deserializers map
        let deserializers = get_entity_deserializers();
        for table_name in deserializers.keys() {
            // Check if table exists in the old schema
            let table_exists: bool = sqlx::query_scalar(
                "SELECT EXISTS (
                    SELECT FROM information_schema.tables
                    WHERE table_schema = 'public'
                    AND table_name = $1
                )",
            )
            .bind(table_name)
            .fetch_one(&pool)
            .await
            .unwrap();

            if !table_exists {
                println!(
                    "Table '{}' doesn't exist in old schema (new entity), skipping",
                    table_name
                );
                continue;
            }

            assert!(
                sqlx::query(&format!("SELECT * FROM {}", table_name))
                    .fetch_all(&pool)
                    .await
                    .is_ok(),
                "Failed to read table: {}",
                table_name
            );
        }

        println!("Successfully read all tables from latest release database");

        crate::server::shared::storage::migration_runner::apply_embedded_migrations(&pool)
            .await
            .expect("Failed to apply current schema to old database");

        println!("Successfully applied current schema to old database");
    } else {
        panic!("No database fixture found at {}", SERVER_DB_FIXTURE);
    }
}

#[tokio::test]
pub async fn test_struct_deserialization_backward_compatibility() {
    use crate::tests::SERVER_DB_FIXTURE;
    use crate::tests::setup_test_db;
    use std::path::Path;

    let db_path = Path::new(SERVER_DB_FIXTURE);

    if db_path.exists() {
        use std::process::Command;

        println!("Testing struct deserialization from migrated old schema");

        let (pool, database_url, _container) = setup_test_db().await;

        let url = url::Url::parse(&database_url).unwrap();
        let host = url.host_str().unwrap();
        let port = url.port().unwrap();
        let database = url.path().trim_start_matches('/');

        pool.close().await;

        // Restore old database
        let output = Command::new("psql")
            .arg("-h")
            .arg(host)
            .arg("-p")
            .arg(port.to_string())
            .arg("-U")
            .arg("postgres")
            .arg("-d")
            .arg(database)
            .arg("-f")
            .arg(db_path)
            .env("PGPASSWORD", "password")
            .output()
            .expect("Failed to execute psql");

        assert!(
            output.status.success(),
            "Failed to restore database:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );

        let pool = sqlx::PgPool::connect(&database_url).await.unwrap();

        // Apply current migrations
        crate::server::shared::storage::migration_runner::apply_embedded_migrations(&pool)
            .await
            .expect("Failed to apply current schema");

        println!("Testing deserialization of all entity types...");

        let deserializers = get_entity_deserializers();

        for (table_name, deserialize_fn) in deserializers.iter() {
            let rows = sqlx::query(&format!("SELECT * FROM {}", table_name))
                .fetch_all(&pool)
                .await
                .expect(&format!("Failed to fetch {}", table_name));

            for row in rows.iter() {
                deserialize_fn(row)
                    .expect(&format!("Failed to deserialize row from {}", table_name));
            }

            println!(
                "✓ Successfully deserialized {} rows from {}",
                rows.len(),
                table_name
            );
        }

        println!("All entity types deserialized successfully from migrated schema");
    } else {
        panic!("No database fixture found at {}", SERVER_DB_FIXTURE);
    }
}

/// Compares each entity's `to_params()` column list against the live
/// `information_schema.columns` for its table. Fails on:
///   - Columns in `to_params()` that don't exist in the live schema (storage
///     would SELECT/INSERT a missing column).
///   - Live NOT-NULL columns without a default that aren't in `to_params()`
///     (entity INSERTs would omit a required column).
///
/// Complement to the release-time container harness — that catches drift at
/// runtime against the previously-deployed binary, this catches code/schema
/// drift within the current binary at `cargo test --lib` time.
///
/// `#[ignore]`-gated because it spins up a testcontainer; opt in with:
///   `cargo test --lib test_entity_columns_match_live_schema -- --ignored`
#[tokio::test]
#[ignore]
pub async fn test_entity_columns_match_live_schema() {
    use crate::server::shared::storage::traits::Storable;
    use crate::tests::setup_test_db;

    let (pool, _database_url, _container) = setup_test_db().await;
    crate::server::shared::storage::migration_runner::apply_embedded_migrations(&pool)
        .await
        .expect("Failed to run migrations");

    /// For a Storable type, fetch the live column spec for its table and
    /// compare against what to_params() produces. Pushes human-readable
    /// failure strings into `failures`.
    async fn check_entity<T: Storable>(pool: &sqlx::PgPool, failures: &mut Vec<String>) {
        let table = T::table_name();
        let insert_columns: Vec<&'static str> = T::default()
            .to_params()
            .expect("to_params should succeed on Default entity")
            .0;

        #[derive(sqlx::FromRow, Debug)]
        struct LiveColumn {
            column_name: String,
            is_nullable: String,
            column_default: Option<String>,
        }
        let live: Vec<LiveColumn> = sqlx::query_as(
            "SELECT column_name, is_nullable, column_default \
             FROM information_schema.columns \
             WHERE table_schema = 'public' AND table_name = $1",
        )
        .bind(table)
        .fetch_all(pool)
        .await
        .unwrap_or_else(|e| panic!("Failed to fetch columns for {}: {}", table, e));

        if live.is_empty() {
            failures.push(format!(
                "{}: table not found in live schema (but a Storable impl exists for it)",
                table
            ));
            return;
        }

        let live_names: std::collections::HashSet<&str> =
            live.iter().map(|c| c.column_name.as_str()).collect();

        for col in &insert_columns {
            if !live_names.contains(col) {
                failures.push(format!(
                    "{}: column {:?} in to_params() is missing from live schema",
                    table, col
                ));
            }
        }

        for col in &live {
            if col.is_nullable == "NO"
                && col.column_default.is_none()
                && !insert_columns.contains(&col.column_name.as_str())
            {
                failures.push(format!(
                    "{}: live schema has NOT NULL column {:?} with no default, \
                     but it's absent from to_params() — INSERTs will fail",
                    table, col.column_name
                ));
            }
        }
    }

    let mut failures: Vec<String> = Vec::new();

    // Exhaustive list of Storable entities — should stay in sync with
    // get_entity_deserializers above. Adding an entity: add an entry here and
    // there together.
    check_entity::<DaemonApiKey>(&pool, &mut failures).await;
    check_entity::<Daemon>(&pool, &mut failures).await;
    check_entity::<Discovery>(&pool, &mut failures).await;
    check_entity::<Dependency>(&pool, &mut failures).await;
    check_entity::<Host>(&pool, &mut failures).await;
    check_entity::<Network>(&pool, &mut failures).await;
    check_entity::<Organization>(&pool, &mut failures).await;
    check_entity::<Service>(&pool, &mut failures).await;
    check_entity::<Subnet>(&pool, &mut failures).await;
    check_entity::<IPAddress>(&pool, &mut failures).await;
    check_entity::<Invite>(&pool, &mut failures).await;
    check_entity::<Share>(&pool, &mut failures).await;
    check_entity::<User>(&pool, &mut failures).await;
    check_entity::<UserApiKey>(&pool, &mut failures).await;
    check_entity::<Tag>(&pool, &mut failures).await;
    check_entity::<Topology>(&pool, &mut failures).await;
    check_entity::<Port>(&pool, &mut failures).await;
    check_entity::<Binding>(&pool, &mut failures).await;
    check_entity::<Credential>(&pool, &mut failures).await;
    check_entity::<Interface>(&pool, &mut failures).await;
    check_entity::<Vlan>(&pool, &mut failures).await;
    check_entity::<EntityTag>(&pool, &mut failures).await;
    check_entity::<DependencyMemberRecord>(&pool, &mut failures).await;
    check_entity::<SubnetVlanRecord>(&pool, &mut failures).await;

    if !failures.is_empty() {
        panic!(
            "Entity column / live-schema drift detected:\n  - {}",
            failures.join("\n  - ")
        );
    }
}

// ============================================================================
// DB-backed enum backward-compat tests
// ============================================================================

#[allow(dead_code)]
const DB_ENUM_BASELINE_PATH: &str = "tests/fixtures/db_enum_baseline.json";

/// Canonical note text written to the fixture by every regen. Documents the
/// alias-preservation contract so future reviewers don't reintroduce a lossy
/// overwrite.
#[allow(dead_code)]
const DB_ENUM_BASELINE_NOTE: &str = "Extra variants beyond the current binary's VARIANTS list are preserved across regens for #[serde(alias = \"…\")] tolerance. To accept a breaking rename, delete the old entry from this file manually and declare Deploy-Mode: downtime for the release.";

/// Read the baseline fixture, filter out top-level metadata keys (anything
/// starting with `_`, e.g. `_note`), and return the enum→variants map. Future
/// metadata keys can be added at the top level without breaking the loader.
#[allow(dead_code)]
fn load_db_enum_baseline() -> std::collections::BTreeMap<String, Vec<String>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(DB_ENUM_BASELINE_PATH);
    let body = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("could not read baseline fixture at {:?}: {}", path, e);
    });
    let raw: std::collections::BTreeMap<String, serde_json::Value> = serde_json::from_str(&body)
        .unwrap_or_else(|e| {
            panic!("baseline fixture at {:?} is not valid JSON: {}", path, e);
        });
    raw.into_iter()
        .filter(|(k, _)| !k.starts_with('_'))
        .map(|(k, v)| {
            let variants: Vec<String> = serde_json::from_value(v).unwrap_or_else(|e| {
                panic!(
                    "baseline fixture entry {:?} at {:?} is not a string array: {}",
                    k, path, e
                );
            });
            (k, variants)
        })
        .collect()
}

/// Merge the existing baseline with the current binary's variants.
///
/// - Enums in `current` keep their entry; if an entry exists in `existing`,
///   union the variant lists (existing order first, new variants appended).
/// - Enums only in `current` are written as-is.
/// - Enums only in `existing` are dropped — whole-enum removal is treated as
///   genuine, not as an alias to preserve.
///
/// Variant-level removals are never applied: if a name appears in `existing`
/// but not in `current`, it stays in the merged output. That's the whole point
/// — `#[serde(alias = "OldName")]` doesn't show up in `strum::VariantNames`,
/// so the only record that the project tolerates `OldName` is the fixture
/// entry. Reviewers accept a breaking rename by hand-deleting the entry; the
/// regen then has nothing to preserve.
#[allow(dead_code)]
fn merge_db_enum_baseline(
    existing: &std::collections::BTreeMap<String, Vec<String>>,
    current: &std::collections::BTreeMap<String, Vec<String>>,
) -> std::collections::BTreeMap<String, Vec<String>> {
    current
        .iter()
        .map(|(enum_name, current_variants)| {
            let merged = match existing.get(enum_name) {
                Some(existing_variants) => {
                    let mut out = existing_variants.clone();
                    for v in current_variants {
                        if !out.contains(v) {
                            out.push(v.clone());
                        }
                    }
                    out
                }
                None => current_variants.clone(),
            };
            (enum_name.clone(), merged)
        })
        .collect()
}

/// Read the existing fixture's enum entries (filtered through
/// [`load_db_enum_baseline`]) without panicking when the file is missing —
/// regen tolerates a fresh checkout.
#[allow(dead_code)]
fn read_existing_db_enum_baseline() -> std::collections::BTreeMap<String, Vec<String>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(DB_ENUM_BASELINE_PATH);
    if !path.exists() {
        return std::collections::BTreeMap::new();
    }
    load_db_enum_baseline()
}

/// Forward-compat: every variant the previous release could emit must still
/// deserialize into the current binary's type (directly or via a
/// `#[serde(alias)]`). Catches rename-without-alias and variant-removal.
///
/// With a variant-names baseline, "still deserializes" is checked as "name is
/// still a known variant" via `strum::VariantNames` on the current enum. If an
/// alias covers the rename, the baseline can be extended manually to include
/// the old name and regeneration will preserve it (regen warns on any name in
/// the current fixture that isn't a current-binary variant).
#[test]
#[ignore = "release-time coexistence gate: run by release.yml with --ignored, not on every build"]
fn test_current_reads_previous_release_variants() {
    use crate::server::shared::storage::traits::SqlValue;

    let baseline = load_db_enum_baseline();
    if baseline.is_empty() {
        // Fresh checkout with bootstrap fixture: nothing to compare yet. First
        // regeneration fills in the baseline; from that point on this test has
        // something to assert against.
        return;
    }

    let current = SqlValue::collect_all_db_enum_variants();

    let mut missing: Vec<String> = Vec::new();
    for (enum_name, baseline_variants) in &baseline {
        let current_variants: Option<&Vec<String>> = current.get(enum_name.as_str());
        let current_set: std::collections::HashSet<&str> = current_variants
            .into_iter()
            .flat_map(|v| v.iter().map(|s| s.as_str()))
            .collect();
        for variant in baseline_variants {
            if !current_set.contains(variant.as_str()) {
                missing.push(format!("{}::{}", enum_name, variant));
            }
        }
    }

    if !missing.is_empty() {
        let example = missing
            .first()
            .and_then(|s| s.rsplit(':').next())
            .unwrap_or("");
        panic!(
            "DB-backed enum forward-compat broken. The current binary has lost \
             variants that the previous release could emit. Rows written by the \
             previous binary will fail to deserialize in the new binary.\n\n\
             Missing variants:\n  - {}\n\n\
             Fix options:\n  \
             (a) Restore the variant — safest if the rename wasn't intentional.\n  \
             (b) Keep serde-level compatibility: add `#[serde(alias = \"{}\")]` \
             on the enum mapping the old name to a current variant, and add \
             the old name to `db_enum_baseline.json` manually for this release. \
             Future regens will preserve the entry automatically (the regen \
             merges the existing fixture with current VARIANTS rather than \
             overwriting).\n  \
             (c) Accept the removal as a breaking rename: delete the entry \
             from `db_enum_baseline.json` by hand and declare \
             `Deploy-Mode: downtime` for the release. Rows written by the \
             previous binary that contain this variant will fail to \
             deserialize until migrated.",
            missing.join("\n  - "),
            example,
        );
    }
}

/// Backward-compat (deploy-window coexistence): the current binary must not be
/// able to emit any variant the previous release's binary can't read. If it
/// does, the old binary panics the moment the new binary writes that variant
/// to the DB.
#[test]
#[ignore = "release-time coexistence gate: run by release.yml with --ignored, not on every build"]
fn test_current_writes_subset_of_previous_release() {
    use crate::server::shared::storage::traits::SqlValue;

    let baseline = load_db_enum_baseline();
    if baseline.is_empty() {
        return; // Fresh bootstrap — see test_current_reads note above.
    }

    let current = SqlValue::collect_all_db_enum_variants();

    let mut added: Vec<String> = Vec::new();
    for (enum_name, current_variants) in &current {
        let baseline_variants: Option<&Vec<String>> = baseline.get(*enum_name);
        let baseline_set: std::collections::HashSet<&str> = baseline_variants
            .into_iter()
            .flat_map(|v| v.iter().map(|s| s.as_str()))
            .collect();
        for variant in current_variants {
            if !baseline_set.contains(variant.as_str()) {
                added.push(format!("{}::{}", enum_name, variant));
            }
        }
    }

    if !added.is_empty() {
        panic!(
            "DB-backed enum backward-compat broken. The current binary can emit \
             variants the previous release's binary can't read. If anything \
             writes these to the DB during the deploy coexistence window, the \
             old binary panics reading them.\n\n\
             Added variants:\n  - {}\n\n\
             Fix: either (a) ensure no code path writes this variant until a \
             subsequent release, (b) ship an intermediate release that teaches \
             old binaries to tolerate the new variant (via `#[serde(alias)]` or \
             a fallback) and regenerate the baseline, or (c) declare \
             `Deploy-Mode: downtime` in the release notes — the CI harness \
             skips this test and the image is labeled for stop-migrate-start \
             deploy semantics.",
            added.join("\n  - ")
        );
    }
}

/// Regenerate `db_enum_baseline.json` from the current binary's DB-backed
/// enums. Run this after cutting a release so the next cycle's coexistence
/// checks compare against the released binary. Opt-in:
///
///   cargo test --lib regenerate_db_enum_baseline -- --ignored
#[test]
#[ignore]
fn regenerate_db_enum_baseline() {
    use crate::server::shared::storage::traits::SqlValue;

    // Merge-aware: preserves variants present in the existing fixture but absent
    // from current VARIANTS, so #[serde(alias)] entries survive release-cycle
    // regens. Whole-enum removals (enum gone from the codebase) are pruned.
    let existing = read_existing_db_enum_baseline();
    let current: std::collections::BTreeMap<String, Vec<String>> =
        SqlValue::collect_all_db_enum_variants()
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
    let merged = merge_db_enum_baseline(&existing, &current);

    // Build the output object. serde_json::Map without the `preserve_order`
    // feature uses BTreeMap, so keys serialize alphabetically — the `_note`
    // field lands at the end of the file (since `_` > `Z` in ASCII), after
    // all PascalCase enum entries. That's intentional; the data is on top
    // and the metadata trails it.
    let mut out = serde_json::Map::new();
    out.insert(
        "_note".to_string(),
        serde_json::Value::String(DB_ENUM_BASELINE_NOTE.to_string()),
    );
    for (enum_name, variants) in &merged {
        out.insert(
            enum_name.clone(),
            serde_json::Value::Array(
                variants
                    .iter()
                    .map(|v| serde_json::Value::String(v.clone()))
                    .collect(),
            ),
        );
    }

    let body = serde_json::to_string_pretty(&serde_json::Value::Object(out))
        .expect("baseline map must serialize to JSON");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(DB_ENUM_BASELINE_PATH);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create fixture dir");
    }
    std::fs::write(&path, format!("{}\n", body)).expect("write baseline fixture");
    println!("Regenerated DB-enum baseline at {:?}", path);
    println!("{} enums catalogued:", merged.len());
    for (enum_name, variants) in &merged {
        let preserved = existing
            .get(enum_name)
            .map(|prev| {
                prev.iter()
                    .filter(|v| !current.get(enum_name).map_or(false, |cur| cur.contains(v)))
                    .count()
            })
            .unwrap_or(0);
        if preserved > 0 {
            println!(
                "  {}: {} variants ({} preserved from previous baseline for serde alias)",
                enum_name,
                variants.len(),
                preserved,
            );
        } else {
            println!("  {}: {} variants", enum_name, variants.len());
        }
    }
    let dropped: Vec<&String> = existing
        .keys()
        .filter(|k| !current.contains_key(*k))
        .collect();
    if !dropped.is_empty() {
        println!(
            "Dropped {} enum(s) no longer present in the codebase: {:?}",
            dropped.len(),
            dropped,
        );
    }
}

#[test]
fn test_merge_db_enum_baseline_preserves_alias_entries() {
    use std::collections::BTreeMap;

    // Existing baseline carries an alias-tolerated old name (`LegacyDaemon`)
    // and a fully-removed enum (`OldEnum`).
    let existing: BTreeMap<String, Vec<String>> = BTreeMap::from([
        (
            "EntitySource".to_string(),
            vec![
                "Daemon".to_string(),
                "User".to_string(),
                "LegacyDaemon".to_string(),
            ],
        ),
        ("OldEnum".to_string(), vec!["A".to_string()]),
    ]);
    // Current codebase has `EntitySource` without `LegacyDaemon` (alias only),
    // a brand-new enum `NewEnum`, and no `OldEnum`.
    let current: BTreeMap<String, Vec<String>> = BTreeMap::from([
        (
            "EntitySource".to_string(),
            vec!["Daemon".to_string(), "User".to_string()],
        ),
        ("NewEnum".to_string(), vec!["X".to_string()]),
    ]);

    let merged = merge_db_enum_baseline(&existing, &current);

    assert_eq!(
        merged.get("EntitySource"),
        Some(&vec![
            "Daemon".to_string(),
            "User".to_string(),
            "LegacyDaemon".to_string(),
        ]),
        "alias-only variant LegacyDaemon must survive regen",
    );
    assert_eq!(
        merged.get("NewEnum"),
        Some(&vec!["X".to_string()]),
        "newly-added enum must appear in merged output",
    );
    assert!(
        merged.get("OldEnum").is_none(),
        "enums absent from current codebase must be pruned, not preserved",
    );
    assert_eq!(merged.len(), 2, "expected exactly EntitySource + NewEnum");
}

/// Every filter the server-side field filters build must be SQL Postgres will
/// accept against the real schema.
///
/// The sibling tests in `storage/filter` assert on the generated string, which
/// cannot tell a valid predicate from one naming a column that does not exist
/// or mis-punctuating a JSON accessor — both of those only fail when a user
/// ticks the filter. Preparing each statement server-side checks syntax,
/// column existence and placeholder typing without needing a single seeded row.
#[tokio::test]
async fn server_side_field_filters_are_valid_sql_against_the_live_schema() {
    use crate::server::shared::storage::filter::StorableFilter;
    use crate::server::shared::storage::traits::Storable;
    use crate::tests::setup_test_db;
    use sqlx::Executor;
    use uuid::Uuid;

    let (pool, _database_url, _container) = setup_test_db().await;
    crate::server::shared::storage::migration_runner::apply_embedded_migrations(&pool)
        .await
        .expect("Failed to run migrations");

    let id = Uuid::new_v4();
    let named = || vec!["ssh".to_string()];

    // (label, table, WHERE clause) for each filter these tabs can send.
    let cases: Vec<(&str, &str, String)> = vec![
        (
            "hosts.hidden",
            Host::table_name(),
            StorableFilter::<Host>::new_unfiltered()
                .hidden_in(&[true])
                .to_where_clause(),
        ),
        (
            "hosts.virtualized_by",
            Host::table_name(),
            StorableFilter::<Host>::new_unfiltered()
                .virtualization_service_in(&[id], true)
                .to_where_clause(),
        ),
        (
            "hosts.virtualized_by (absence only)",
            Host::table_name(),
            StorableFilter::<Host>::new_unfiltered()
                .virtualization_service_in(&[], true)
                .to_where_clause(),
        ),
        (
            "hosts.services",
            Host::table_name(),
            StorableFilter::<Host>::new_unfiltered()
                .has_service_named(&named())
                .to_where_clause(),
        ),
        (
            "services.service_definition",
            Service::table_name(),
            StorableFilter::<Service>::new_unfiltered()
                .service_definition_in(&named())
                .to_where_clause(),
        ),
        (
            "services.containerized_by",
            Service::table_name(),
            StorableFilter::<Service>::new_unfiltered()
                .virtualization_service_in(&[id], true)
                .to_where_clause(),
        ),
        (
            "discovery.daemon_id",
            Discovery::table_name(),
            StorableFilter::<Discovery>::new_unfiltered()
                .daemon_ids(&[id])
                .to_where_clause(),
        ),
        (
            "discovery.discovery_type",
            Discovery::table_name(),
            StorableFilter::<Discovery>::new_unfiltered()
                .discovery_type_in(&named())
                .to_where_clause(),
        ),
    ];

    let mut failures = Vec::new();
    for (label, table, where_clause) in cases {
        let sql = format!("SELECT 1 FROM {} {}", table, where_clause);
        if let Err(e) = pool.describe(sql.as_str()).await {
            failures.push(format!("{label}: {e}\n  sql: {sql}"));
        }
    }

    assert!(
        failures.is_empty(),
        "Postgres rejected a filter the UI can send:\n{}",
        failures.join("\n")
    );
}
