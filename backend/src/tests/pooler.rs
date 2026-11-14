//! The storage layer behind a transaction-mode pooler, as prod reaches Neon. Background tasks run
//! storage queries concurrently, and the pooler hands consecutive protocol messages from one
//! client to whichever backend is free. v0.20.0's unnamed statements split across backends there
//! (`bind message supplies N parameters, but prepared statement "" requires M`, or a panic decoding
//! a row of the wrong shape) and took prod down on 2026-11-13.

use std::sync::Arc;

use uuid::Uuid;

use super::{organization, setup_pooled_test_db};
use crate::server::organizations::r#impl::base::Organization;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::generic::GenericPostgresStorage;
use crate::server::shared::storage::traits::{Storable, Storage};
use crate::server::tags::r#impl::base::{Tag, TagBase};

fn tag(organization_id: Uuid) -> Tag {
    Tag::new(TagBase {
        name: format!("tag-{}", Uuid::new_v4().simple()),
        organization_id,
        ..TagBase::default()
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn storage_queries_run_concurrently_through_a_transaction_pooler() {
    let (_direct, pooled, _db) = setup_pooled_test_db(10).await;
    let org = GenericPostgresStorage::<Organization>::new(pooled.clone())
        .create(&organization())
        .await
        .unwrap();
    let tags = Arc::new(GenericPostgresStorage::<Tag>::new(pooled.clone()));
    let mut seeded = Vec::new();
    for _ in 0..20 {
        seeded.push(tags.create(&tag(org.id)).await.unwrap().id);
    }
    let seeded = Arc::new(seeded);

    let mut tasks = Vec::new();
    for task in 0..16usize {
        let tags = tags.clone();
        let seeded = seeded.clone();
        let org_id = org.id;
        tasks.push(tokio::spawn(async move {
            for i in 0..40usize {
                // A different parameter count on each call, as discovery's batched reads have.
                let n = 1 + (task * 7 + i) % seeded.len();
                let found = tags
                    .get_all(StorableFilter::new_from_entity_ids(&seeded[..n]))
                    .await?;
                anyhow::ensure!(found.len() == n, "read {} of {n} tags", found.len());
                tags.count(StorableFilter::<Tag>::new_from_org_id(&org_id))
                    .await?;
                let mut created = tags.create(&tag(org_id)).await?;
                created.base.description = Some(format!("task {task} round {i}"));
                tags.update(&mut created).await?;
                tags.get_by_id(&created.id)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("tag {} vanished", created.id))?;
            }
            anyhow::Ok(())
        }));
    }

    let mut failures = Vec::new();
    for task in tasks {
        match task.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => failures.push(format!("{e:#}")),
            Err(panic) => failures.push(format!("panicked: {panic}")),
        }
    }
    assert!(
        failures.is_empty(),
        "{} of 16 tasks failed through the pooler:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// A running server's cached statements survive a migration that adds a column. Storage names its
/// columns, so the result type of a statement prepared before the `ALTER` is unchanged after it.
/// The same read as `SELECT *` gets SQLSTATE 0A000 ("cached plan must not change result type")
/// from every backend that prepared it before the column existed.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn storage_reads_survive_a_migration_that_adds_a_column() {
    let (direct, pooled, _db) = setup_pooled_test_db(10).await;
    let org = GenericPostgresStorage::<Organization>::new(pooled.clone())
        .create(&organization())
        .await
        .unwrap();
    let tags = GenericPostgresStorage::<Tag>::new(pooled.clone());
    tags.create(&tag(org.id)).await.unwrap();
    let by_org = || StorableFilter::<Tag>::new_from_org_id(&org.id);
    let select_star = || {
        sqlx::query("SELECT * FROM tags WHERE organization_id = $1")
            .bind(org.id)
            .fetch_all(&pooled)
    };

    // Prepare both reads on every backend round-robin reaches.
    for _ in 0..30 {
        tags.get_all(by_org()).await.unwrap();
        select_star().await.unwrap();
    }

    sqlx::query("ALTER TABLE tags ADD COLUMN added_by_a_later_release INT")
        .execute(&direct)
        .await
        .unwrap();

    let mut select_star_refused = 0;
    for _ in 0..30 {
        let read = tags.get_all(by_org()).await;
        assert!(
            read.is_ok(),
            "storage read failed after ADD COLUMN: {read:?}"
        );
        if let Err(sqlx::Error::Database(e)) = select_star().await
            && e.code().as_deref() == Some("0A000")
        {
            select_star_refused += 1;
        }
    }
    assert!(
        select_star_refused > 0,
        "SELECT * never hit 0A000, so this test no longer shows what named columns prevent"
    );
}
