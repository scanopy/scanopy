//! A pooled connection holds a bounded number of server-side prepared statements, however many
//! distinct statements run on it, and a storage query of one shape is one statement whatever
//! its list lengths or row counts.

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

use crate::server::organizations::r#impl::base::Organization;
use crate::server::shared::storage::factory::connect_options;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::generic::GenericPostgresStorage;
use crate::server::shared::storage::traits::{Storable, Storage};
use crate::server::tags::r#impl::base::{Tag, TagBase};
use crate::tests::{organization, setup_migrated_test_db, setup_pooled_test_db};

/// A one-connection pool straight to Postgres, so `pg_prepared_statements` sees every statement.
async fn single_connection(url: &str) -> PgPool {
    PgPoolOptions::new()
        .max_connections(1)
        .connect_with(connect_options(url).unwrap())
        .await
        .unwrap()
}

async fn held(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM pg_prepared_statements")
        .fetch_one(pool)
        .await
        .unwrap()
}

fn tag(organization_id: Uuid) -> Tag {
    Tag::new(TagBase {
        name: format!("tag-{}", Uuid::new_v4().simple()),
        organization_id,
        ..TagBase::default()
    })
}

#[tokio::test]
async fn a_connection_holds_a_bounded_number_of_prepared_statements() {
    let (setup_pool, _url, db) = setup_migrated_test_db().await;
    setup_pool.close().await;
    let pool = single_connection(&db.url()).await;

    // Fixed-text queries outside the storage layer, as tower-sessions and the lock module run.
    for i in 0..500 {
        sqlx::query(&format!("SELECT {i}::int"))
            .execute(&pool)
            .await
            .unwrap();
    }

    let held = held(&pool).await;
    assert!(
        held <= 101,
        "the connection holds {held} prepared statements after 500 distinct queries"
    );
}

/// Behind the pooler, each backend holds at most the pooler's `max_prepared_statements` (Neon's
/// 1000) however many distinct statements its clients run.
#[tokio::test]
async fn a_backend_behind_the_pooler_holds_a_bounded_number_of_prepared_statements() {
    let (_direct, pooled, _db) = setup_pooled_test_db(4).await;
    for i in 0..1500 {
        sqlx::query(&format!("SELECT {i}::int"))
            .execute(&pooled)
            .await
            .unwrap();
    }

    // Round-robin sends each count to the next backend; enough reads reach all of them.
    let mut most = 0;
    for _ in 0..40 {
        most = most.max(held(&pooled).await);
    }
    assert!(
        most <= 1000,
        "a backend holds {most} prepared statements after 1500 distinct queries"
    );
}

/// An ID list binds as one array, so every list length runs the same statement.
#[tokio::test]
async fn reads_of_any_list_length_share_one_statement() {
    let (setup_pool, _url, db) = setup_migrated_test_db().await;
    setup_pool.close().await;
    let pool = single_connection(&db.url()).await;
    let tags = GenericPostgresStorage::<Tag>::new(pool.clone());

    tags.get_all(StorableFilter::new_from_entity_ids(&[Uuid::new_v4()]))
        .await
        .unwrap();
    let before = held(&pool).await;
    for n in 2..=200 {
        let ids: Vec<Uuid> = (0..n).map(|_| Uuid::new_v4()).collect();
        tags.get_all(StorableFilter::new_from_entity_ids(&ids))
            .await
            .unwrap();
    }
    let added = held(&pool).await - before;
    assert_eq!(
        added, 0,
        "199 more list lengths prepared {added} more statements"
    );
}

/// A bulk insert splits into power-of-two row counts, so batches of 1 to 64 rows run at most
/// seven insert statements between them.
#[tokio::test]
async fn bulk_inserts_of_any_size_share_a_few_statements() {
    let (setup_pool, _url, db) = setup_migrated_test_db().await;
    setup_pool.close().await;
    let pool = single_connection(&db.url()).await;
    let org = GenericPostgresStorage::<Organization>::new(pool.clone())
        .create(&organization())
        .await
        .unwrap();
    let tags = GenericPostgresStorage::<Tag>::new(pool.clone());

    let before = held(&pool).await;
    for n in 1..=64 {
        let batch: Vec<Tag> = (0..n).map(|_| tag(org.id)).collect();
        let saved = tags.create_many(&batch).await.unwrap();
        assert_eq!(saved.len(), n);
    }
    let added = held(&pool).await - before;
    assert!(
        added <= 7,
        "batches of 1 to 64 rows prepared {added} insert statements"
    );
}
