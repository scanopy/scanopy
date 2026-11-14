//! DB-level entity locks: typed Postgres advisory locks that serialize
//! entity operations across all backend instances sharing the database.
//!
//! Replaces the per-process in-memory lock maps (`host_locks`,
//! `service_locks`, `dependency_update_lock`) documented in
//! `LOCK_AUDIT.md`. Two scopes, one key type:
//!
//! - [`session_lock`] — RAII guard on a dedicated pool connection, for
//!   multi-service critical sections that run pool-based storage calls and
//!   cannot become a single transaction (host/service create/update/delete).
//!   The guard holds `pg_try_advisory_xact_lock` inside an open transaction
//!   on its own connection; the critical section's storage calls run on other
//!   pool connections. Acquisition polls with backoff instead of blocking
//!   server-side, so waiters never pin pool connections that the lock holder
//!   needs for its own work.
//!
//!   Not a session-level lock (`pg_advisory_lock` / `pg_advisory_unlock`):
//!   behind a transaction-mode pooler (Neon's pooled endpoint, PgBouncer) two
//!   statements from one client can run on different backends, so the unlock
//!   lands on a backend that holds nothing and the lock stays held until the
//!   pooler recycles the first one. An open transaction stays on one backend,
//!   and commit or rollback releases the lock there.
//! - [`xact_lock`] — `pg_advisory_xact_lock` on an existing transaction,
//!   auto-released at commit/rollback, for tx-local read-modify-writes
//!   (position assignment, junction-table syncs).
//!
//! Key derivation reuses sqlx's [`PgAdvisoryLock`] HKDF scheme (stable
//! across processes and releases — required for rolling deploys where old
//! and new instances contend on the same entities).

use crate::server::shared::entities::EntityDiscriminants;
use sqlx::postgres::{PgAdvisoryLock, PgAdvisoryLockKey};
use sqlx::{PgPool, Postgres};
use std::time::Duration;
use uuid::Uuid;

pub const DEFAULT_LOCK_TIMEOUT: Duration = Duration::from_secs(30);
/// A merge locks two hosts and is user-interactive; fail faster.
pub const MERGE_LOCK_TIMEOUT: Duration = Duration::from_secs(10);
const RETRY_INTERVAL: Duration = Duration::from_millis(50);

/// Closed registry of every DB-level lock in the system. Adding a lock site
/// means adding a variant here — keys are never built from ad-hoc strings,
/// so every lock scope is auditable in one place and two sites can't
/// accidentally mint overlapping keys.
/// The variant name (snake_case) is the `lock` label on `scanopy_lock_wait_seconds`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum LockKey {
    /// Serialize update/delete/merge of one host.
    Host(Uuid),
    /// Serialize host discovery dedup within a site. Scope-keyed (not
    /// id-keyed) because two concurrent submissions of the same NEW device
    /// carry distinct fresh UUIDs — an id key would never contend.
    HostDedup { site_id: Uuid },
    /// Serialize update/delete/transfer of one service.
    Service(Uuid),
    /// Serialize service create-dedup and position assignment. Host-scoped:
    /// both the natural-key match and `next_position` read the per-host
    /// service list.
    ServiceDedup { host_id: Uuid },
    /// Serialize dependency-members read-modify-write loops (replaces the
    /// old process-global `dependency_update_lock`), per site.
    DependencyMembers { site_id: Uuid },
    /// Serialize MAX+1 position assignment for IP addresses on a host
    /// (the row being positioned doesn't exist yet, so `FOR UPDATE`
    /// cannot cover this).
    IpPositions { host_id: Uuid },
    /// Serialize a junction-table read-diff-write sync, keyed by the
    /// parent entity row.
    JunctionSync {
        parent: EntityDiscriminants,
        parent_id: Uuid,
    },
    /// Serialize API key rotation (read → set_key → update).
    ApiKey(Uuid),
    /// Serialize read-modify-writes of one organization row: the billing
    /// event mirror, license key regeneration, and license check-ins.
    Organization(Uuid),
    /// Serialize creating an organization's tag groups and reordering any group's tags.
    TagGroups { organization_id: Uuid },
    /// Serialize an organization's cloud setup (site, system subnets, live topology, starter
    /// tags), so registration and a billing event landing together can't both create a site.
    /// Taken before `SiteSetup` and `TagGroups`, never after.
    CloudSetup { organization_id: Uuid },
    /// Serialize changes to who holds Owner in an organization (deleting, demoting or moving a
    /// user out), so two of them can't each count the other and leave no Owner. The owners and
    /// the targets are read under it. Taken before `JunctionSync`, never after.
    OrganizationOwners { organization_id: Uuid },
    /// Serialize filling in one site's system subnets and live topology, so the org's cloud
    /// setup and the site's Created event can't both create them.
    SiteSetup { site_id: Uuid },
    /// Serialize the once-per-version release announcement across containers
    /// starting at the same time. One server-wide key.
    ReleaseAnnouncement,
    /// Serialize creating this database's server instance id across containers
    /// starting at the same time. One server-wide key.
    ServerInstance,
    /// Serialize recording license check-ins from an organization's server
    /// instances, so two first check-ins from one instance can't both insert it.
    LicenseInstances { organization_id: Uuid },
    /// Serialize the terms-notice sweep across containers, so a user is sent
    /// each terms update once. One server-wide key.
    TermsNotice,
}

impl LockKey {
    /// Canonical string form; the sole input to key derivation and the
    /// total order used by [`session_lock_many`]. Changing an existing
    /// prefix breaks lock compatibility across a rolling deploy — don't.
    fn canonical(&self) -> String {
        match self {
            LockKey::Host(id) => format!("host:{id}"),
            LockKey::HostDedup { site_id } => format!("host-dedup:{site_id}"),
            LockKey::Service(id) => format!("service:{id}"),
            LockKey::ServiceDedup { host_id } => format!("service-dedup:{host_id}"),
            LockKey::DependencyMembers { site_id } => {
                format!("dependency-members:{site_id}")
            }
            LockKey::IpPositions { host_id } => format!("ip-positions:{host_id}"),
            LockKey::JunctionSync { parent, parent_id } => {
                let parent: &'static str = parent.into();
                format!("junction-sync:{parent}:{parent_id}")
            }
            LockKey::ApiKey(id) => format!("api-key:{id}"),
            LockKey::Organization(id) => format!("organization:{id}"),
            LockKey::TagGroups { organization_id } => format!("tag-groups:{organization_id}"),
            LockKey::CloudSetup { organization_id } => format!("cloud-setup:{organization_id}"),
            LockKey::OrganizationOwners { organization_id } => {
                format!("organization-owners:{organization_id}")
            }
            LockKey::SiteSetup { site_id } => format!("site-setup:{site_id}"),
            LockKey::ReleaseAnnouncement => "release-announcement".to_string(),
            LockKey::ServerInstance => "server-instance".to_string(),
            LockKey::TermsNotice => "terms-notice".to_string(),
            LockKey::LicenseInstances { organization_id } => {
                format!("license-instances:{organization_id}")
            }
        }
    }

    fn to_pg_lock(self) -> PgAdvisoryLock {
        PgAdvisoryLock::new(self.canonical())
    }

    /// The 64-bit Postgres advisory-lock key (sqlx HKDF derivation).
    pub fn pg_key(self) -> i64 {
        match self.to_pg_lock().key() {
            PgAdvisoryLockKey::BigInt(key) => *key,
            // `PgAdvisoryLock::new` always produces the BigInt keyspace.
            other => unreachable!("PgAdvisoryLock::new produced non-BigInt key {other:?}"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LockError {
    #[error("timed out after {0:?} waiting for DB lock {1:?}")]
    Timeout(Duration, LockKey),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

/// Holds a transaction-scoped advisory lock in an open transaction on a
/// dedicated pool connection.
///
/// Call [`release`](Self::release) on success paths; it commits. `Drop`
/// covers error/cancel paths: the dropped transaction rolls back when the
/// pool takes its connection back, which releases the lock.
pub struct SessionLockGuard {
    key: LockKey,
    tx: Option<sqlx::Transaction<'static, Postgres>>,
}

impl SessionLockGuard {
    /// Release the lock and return the connection to the pool.
    pub async fn release(mut self) -> Result<(), LockError> {
        if let Some(tx) = self.tx.take() {
            tx.commit().await?;
        }
        Ok(())
    }

    pub fn key(&self) -> LockKey {
        self.key
    }
}

/// Acquire an advisory lock held until the guard is released or dropped,
/// polling `pg_try_advisory_xact_lock` with backoff until `timeout`. The
/// returned guard holds one pool connection for the duration of the
/// critical section.
pub async fn session_lock(
    pool: &PgPool,
    key: LockKey,
    timeout: Duration,
) -> Result<SessionLockGuard, LockError> {
    let pg_key = key.pg_key();
    let started = tokio::time::Instant::now();
    let deadline = started + timeout;
    loop {
        let mut tx = pool.begin().await?;
        let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1)")
            .bind(pg_key)
            .fetch_one(&mut *tx)
            .await?;
        if acquired {
            record_lock_wait(key, started.elapsed());
            return Ok(SessionLockGuard { key, tx: Some(tx) });
        }
        // Not acquired: the connection goes straight back to the pool so
        // waiting never starves the lock holder of connections.
        tx.rollback().await?;
        if tokio::time::Instant::now() + RETRY_INTERVAL > deadline {
            record_lock_wait(key, started.elapsed());
            return Err(LockError::Timeout(timeout, key));
        }
        tokio::time::sleep(RETRY_INTERVAL).await;
    }
}

fn record_lock_wait(key: LockKey, waited: Duration) {
    crate::server::metrics::db::record_lock_wait(key.into(), waited.as_secs_f64());
}

/// Acquire several session locks in the process-wide canonical order
/// (sorted by [`LockKey::canonical`]), so concurrent multi-lock sites can
/// never deadlock/livelock each other. Always use this — never nest
/// individual [`session_lock`] calls.
pub async fn session_lock_many(
    pool: &PgPool,
    keys: &[LockKey],
    timeout: Duration,
) -> Result<Vec<SessionLockGuard>, LockError> {
    let mut sorted = keys.to_vec();
    sorted.sort_by_key(|k| k.canonical());
    let mut guards = Vec::with_capacity(sorted.len());
    for key in sorted {
        // On failure, already-acquired guards drop and release.
        guards.push(session_lock(pool, key, timeout).await?);
    }
    Ok(guards)
}

/// Release a batch of guards on the success path.
pub async fn release_all(guards: Vec<SessionLockGuard>) -> Result<(), LockError> {
    for guard in guards {
        guard.release().await?;
    }
    Ok(())
}

/// Acquire a transaction-scoped advisory lock (`pg_advisory_xact_lock`) —
/// auto-released at commit/rollback. Blocks server-side up to `timeout`
/// (bounded via `SET LOCAL lock_timeout`, restored right after so later
/// row-lock waits in the transaction keep their configured behavior).
pub async fn xact_lock(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    key: LockKey,
    timeout: Duration,
) -> Result<(), LockError> {
    let millis = timeout.as_millis().max(1);
    // `SET LOCAL` doesn't support bind parameters; the value is a plain
    // integer derived from a Duration, not user input.
    sqlx::query(&format!("SET LOCAL lock_timeout = '{millis}ms'"))
        .execute(&mut **tx)
        .await?;
    // Postgres blocks inside this statement, so the wait also counts as query time.
    let started = tokio::time::Instant::now();
    let result = sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(key.pg_key())
        .execute(&mut **tx)
        .await;
    record_lock_wait(key, started.elapsed());
    match result {
        Ok(_) => {
            sqlx::query("SET LOCAL lock_timeout = DEFAULT")
                .execute(&mut **tx)
                .await?;
            Ok(())
        }
        Err(e) => {
            // SQLSTATE 55P03: lock_not_available (lock_timeout elapsed).
            let is_lock_timeout = e
                .as_database_error()
                .and_then(|db| db.code())
                .is_some_and(|code| code == "55P03");
            if is_lock_timeout {
                Err(LockError::Timeout(timeout, key))
            } else {
                Err(LockError::Db(e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{setup_pooled_test_db, setup_test_db};

    fn uuid_a() -> Uuid {
        Uuid::parse_str("11111111-2222-3333-4444-555555555555").unwrap()
    }

    /// Lock keys must be stable across releases: a rolling deploy has old
    /// and new instances contending on the same entities. Fails if the
    /// canonical string format or sqlx's HKDF derivation ever changes —
    /// either would silently break cross-version mutual exclusion.
    #[test]
    fn test_pg_key_stable_across_releases() {
        assert_eq!(LockKey::Host(uuid_a()).pg_key(), -3647068583662342453);
    }

    /// Distinct variants over the same UUID must produce distinct keys —
    /// otherwise unrelated operations on one entity would falsely contend.
    #[test]
    fn test_distinct_variants_distinct_keys() {
        let keys = [
            LockKey::Host(uuid_a()).pg_key(),
            LockKey::HostDedup { site_id: uuid_a() }.pg_key(),
            LockKey::Service(uuid_a()).pg_key(),
            LockKey::ServiceDedup { host_id: uuid_a() }.pg_key(),
            LockKey::DependencyMembers { site_id: uuid_a() }.pg_key(),
            LockKey::IpPositions { host_id: uuid_a() }.pg_key(),
            LockKey::JunctionSync {
                parent: EntityDiscriminants::Host,
                parent_id: uuid_a(),
            }
            .pg_key(),
            LockKey::ApiKey(uuid_a()).pg_key(),
            LockKey::Organization(uuid_a()).pg_key(),
            LockKey::TagGroups {
                organization_id: uuid_a(),
            }
            .pg_key(),
            LockKey::CloudSetup {
                organization_id: uuid_a(),
            }
            .pg_key(),
            LockKey::OrganizationOwners {
                organization_id: uuid_a(),
            }
            .pg_key(),
            LockKey::SiteSetup { site_id: uuid_a() }.pg_key(),
        ];
        let unique: std::collections::HashSet<i64> = keys.iter().copied().collect();
        assert_eq!(unique.len(), keys.len(), "lock key collision: {keys:?}");
    }

    #[tokio::test]
    async fn test_session_lock_mutual_exclusion_and_release() {
        let (pool, _url, _container) = setup_test_db().await;
        let key = LockKey::Host(Uuid::new_v4());
        let other_key = LockKey::Host(Uuid::new_v4());

        let guard = session_lock(&pool, key, DEFAULT_LOCK_TIMEOUT)
            .await
            .unwrap();

        // Same key: times out while held.
        let contended = session_lock(&pool, key, Duration::from_millis(200)).await;
        assert!(matches!(contended, Err(LockError::Timeout(_, _))));

        // Different key: acquires immediately.
        let other = session_lock(&pool, other_key, Duration::from_millis(200))
            .await
            .unwrap();
        other.release().await.unwrap();

        // After release: acquires.
        guard.release().await.unwrap();
        let reacquired = session_lock(&pool, key, Duration::from_millis(500))
            .await
            .unwrap();
        reacquired.release().await.unwrap();
    }

    #[tokio::test]
    async fn test_session_guard_drop_releases() {
        let (pool, _url, _container) = setup_test_db().await;
        let key = LockKey::Service(Uuid::new_v4());

        let guard = session_lock(&pool, key, DEFAULT_LOCK_TIMEOUT)
            .await
            .unwrap();
        drop(guard);

        // The rollback lands when the pool takes the connection back; poll until it does.
        let reacquired = session_lock(&pool, key, Duration::from_secs(5))
            .await
            .expect("lock should be released after guard drop");
        reacquired.release().await.unwrap();
    }

    #[tokio::test]
    async fn test_xact_lock_auto_release_on_commit_and_rollback() {
        let (pool, _url, _container) = setup_test_db().await;
        let key = LockKey::IpPositions {
            host_id: Uuid::new_v4(),
        };

        // Held by tx1 → tx2 times out.
        let mut tx1 = pool.begin().await.unwrap();
        xact_lock(&mut tx1, key, DEFAULT_LOCK_TIMEOUT)
            .await
            .unwrap();
        let mut tx2 = pool.begin().await.unwrap();
        let contended = xact_lock(&mut tx2, key, Duration::from_millis(200)).await;
        assert!(matches!(contended, Err(LockError::Timeout(_, _))));
        drop(tx2); // aborted by the 55P03 error; discard

        // Commit tx1 → acquirable again.
        tx1.commit().await.unwrap();
        let mut tx3 = pool.begin().await.unwrap();
        xact_lock(&mut tx3, key, Duration::from_millis(500))
            .await
            .unwrap();

        // Rollback (via drop) also releases.
        drop(tx3);
        let mut tx4 = pool.begin().await.unwrap();
        xact_lock(&mut tx4, key, Duration::from_millis(500))
            .await
            .unwrap();
        tx4.rollback().await.unwrap();
    }

    /// Sorted acquisition in session_lock_many must prevent the classic
    /// AB/BA deadlock (with try-polling it would present as livelock until
    /// timeout). Two tasks repeatedly grab the same pair passed in opposite
    /// orders; every round must complete well within the timeout.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_session_lock_many_ordering_prevents_deadlock() {
        let (pool, _url, _container) = setup_test_db().await;
        let a = LockKey::Host(uuid_a());
        let b = LockKey::HostDedup { site_id: uuid_a() };

        let spawn_looper = |pool: PgPool, keys: [LockKey; 2]| {
            tokio::spawn(async move {
                for _ in 0..20 {
                    let guards = session_lock_many(&pool, &keys, Duration::from_secs(10))
                        .await
                        .expect("ordered acquisition must not deadlock");
                    release_all(guards).await.unwrap();
                }
            })
        };

        let t1 = spawn_looper(pool.clone(), [a, b]);
        let t2 = spawn_looper(pool.clone(), [b, a]);
        t1.await.unwrap();
        t2.await.unwrap();
    }

    /// Behind a transaction-mode pooler (prod's Neon endpoint), a released lock must be free.
    /// A session-level lock taken on one backend and unlocked from another stays held until the
    /// pooler recycles the first, which is how registration's `TagGroups` lock outlived its
    /// guard and timed out the billing run's cloud setup on 2026-10-19.
    #[tokio::test]
    async fn test_released_lock_is_free_behind_transaction_pooler() {
        let (direct, pooled, _db) = setup_pooled_test_db(4).await;

        for _ in 0..10 {
            let key = LockKey::TagGroups {
                organization_id: Uuid::new_v4(),
            };
            let guard = session_lock(&pooled, key, DEFAULT_LOCK_TIMEOUT)
                .await
                .unwrap();
            guard.release().await.unwrap();

            let reacquired = session_lock(&direct, key, Duration::from_secs(2))
                .await
                .expect("a released lock must be free on every backend");
            reacquired.release().await.unwrap();
        }
    }

    /// The whole point: exclusion must hold across separate pools (i.e.
    /// separate backend instances), not just within one process.
    #[tokio::test]
    async fn test_exclusion_across_pools() {
        let (pool_a, url, _container) = setup_test_db().await;
        let pool_b = PgPool::connect(&url).await.unwrap();
        let key = LockKey::HostDedup {
            site_id: Uuid::new_v4(),
        };

        let guard = session_lock(&pool_a, key, DEFAULT_LOCK_TIMEOUT)
            .await
            .unwrap();
        let contended = session_lock(&pool_b, key, Duration::from_millis(200)).await;
        assert!(
            matches!(contended, Err(LockError::Timeout(_, _))),
            "lock held via pool A must exclude pool B"
        );

        // And a session lock must block an xact lock on the same key from
        // the other pool (shared keyspace).
        let mut tx = pool_b.begin().await.unwrap();
        let xact_contended = xact_lock(&mut tx, key, Duration::from_millis(200)).await;
        assert!(matches!(xact_contended, Err(LockError::Timeout(_, _))));
        drop(tx);

        guard.release().await.unwrap();
        let from_b = session_lock(&pool_b, key, Duration::from_millis(500))
            .await
            .unwrap();
        from_b.release().await.unwrap();
    }
}
