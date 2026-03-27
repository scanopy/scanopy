use crate::server::{
    bindings::r#impl::base::Binding,
    credentials::r#impl::base::Credential,
    daemon_api_keys::r#impl::base::DaemonApiKey,
    daemons::r#impl::base::Daemon,
    discovery::r#impl::base::Discovery,
    groups::{group_bindings::GroupBinding, r#impl::base::Group},
    hosts::r#impl::base::Host,
    if_entries::r#impl::base::IfEntry,
    interfaces::r#impl::base::Interface,
    invites::r#impl::base::Invite,
    networks::r#impl::Network,
    organizations::r#impl::base::Organization,
    ports::r#impl::base::Port,
    services::r#impl::base::Service,
    shared::storage::traits::Storable,
    shares::r#impl::base::Share,
    subnets::r#impl::base::Subnet,
    tags::entity_tags::EntityTag,
    tags::r#impl::base::Tag,
    topology::types::base::Topology,
    user_api_keys::r#impl::base::UserApiKey,
    users::r#impl::base::User,
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
        Group::table_name(),
        Box::new(|row| {
            Group::from_row(row)?;
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
        Interface::table_name(),
        Box::new(|row| {
            Interface::from_row(row)?;
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
        GroupBinding::table_name(),
        Box::new(|row| {
            GroupBinding::from_row(row)?;
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

    map.insert(
        IfEntry::table_name(),
        Box::new(|row| {
            IfEntry::from_row(row)?;
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
    sqlx::migrate!("./migrations")
        .run(&pool)
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

        sqlx::migrate!("./migrations")
            .run(&pool)
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
        sqlx::migrate!("./migrations")
            .run(&pool)
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
