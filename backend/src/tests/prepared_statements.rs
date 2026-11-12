//! A pooled connection holds a bounded number of server-side prepared statements, however many
//! distinct statements run on it.

use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

use crate::server::shared::storage::factory::connect_options;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::generic::GenericPostgresStorage;
use crate::server::shared::storage::traits::Storage;
use crate::server::tags::r#impl::base::Tag;
use crate::tests::setup_migrated_test_db;

#[tokio::test]
async fn a_connection_holds_a_bounded_number_of_prepared_statements() {
    let (setup_pool, url, _db) = setup_migrated_test_db().await;
    setup_pool.close().await;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_with(connect_options(&url).unwrap())
        .await
        .unwrap();

    // Dynamic storage queries: each IN-list length is a distinct statement.
    let tags = GenericPostgresStorage::<Tag>::new(pool.clone());
    for n in 1..=200 {
        let ids: Vec<Uuid> = (0..n).map(|_| Uuid::new_v4()).collect();
        tags.get_all(StorableFilter::new_from_entity_ids(&ids))
            .await
            .unwrap();
    }
    // Fixed-text queries outside the storage layer, as tower-sessions and the lock module run.
    for i in 0..300 {
        sqlx::query(&format!("SELECT {i}::int"))
            .execute(&pool)
            .await
            .unwrap();
    }

    let held: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_prepared_statements")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(
        held <= 101,
        "the connection holds {held} prepared statements after 500 distinct queries"
    );
}
