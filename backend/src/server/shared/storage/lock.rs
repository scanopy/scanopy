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
//!   Acquisition polls `pg_try_advisory_lock` with backoff instead of
//!   blocking server-side, so waiters never pin pool connections that the
//!   lock holder needs for its own work.
//! - [`xact_lock`] — `pg_advisory_xact_lock` on an existing transaction,
//!   auto-released at commit/rollback, for tx-local read-modify-writes
//!   (position assignment, junction-table syncs).
//!
//! Key derivation reuses sqlx's [`PgAdvisoryLock`] HKDF scheme (stable
//! across processes and releases — required for rolling deploys where old
//! and new instances contend on the same entities).

use crate::server::shared::entities::EntityDiscriminants;
use sqlx::pool::PoolConnection;
use sqlx::postgres::{PgAdvisoryLock, PgAdvisoryLockKey};
use sqlx::{Either, PgPool, Postgres};
use std::time::Duration;
use uuid::Uuid;

pub const DEFAULT_LOCK_TIMEOUT: Duration = Duration::from_secs(30);
/// Consolidation locks two hosts and is user-interactive; fail faster.
pub const CONSOLIDATE_LOCK_TIMEOUT: Duration = Duration::from_secs(10);
const RETRY_INTERVAL: Duration = Duration::from_millis(50);

/// Closed registry of every DB-level lock in the system. Adding a lock site
/// means adding a variant here — keys are never built from ad-hoc strings,
/// so every lock scope is auditable in one place and two sites can't
/// accidentally mint overlapping keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockKey {
    /// Serialize update/delete/consolidate of one host.
    Host(Uuid),
    /// Serialize host discovery dedup within a network. Scope-keyed (not
    /// id-keyed) because two concurrent submissions of the same NEW device
    /// carry distinct fresh UUIDs — an id key would never contend.
    HostDedup { network_id: Uuid },
    /// Serialize update/delete/transfer of one service.
    Service(Uuid),
    /// Serialize service create-dedup and position assignment. Host-scoped:
    /// both the natural-key match and `next_position` read the per-host
    /// service list.
    ServiceDedup { host_id: Uuid },
    /// Serialize dependency-members read-modify-write loops (replaces the
    /// old process-global `dependency_update_lock`), per network.
    DependencyMembers { network_id: Uuid },
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
}

impl LockKey {
    /// Canonical string form; the sole input to key derivation and the
    /// total order used by [`session_lock_many`]. Changing an existing
    /// prefix breaks lock compatibility across a rolling deploy — don't.
    fn canonical(&self) -> String {
        match self {
            LockKey::Host(id) => format!("host:{id}"),
            LockKey::HostDedup { network_id } => format!("host-dedup:{network_id}"),
            LockKey::Service(id) => format!("service:{id}"),
            LockKey::ServiceDedup { host_id } => format!("service-dedup:{host_id}"),
            LockKey::DependencyMembers { network_id } => {
                format!("dependency-members:{network_id}")
            }
            LockKey::IpPositions { host_id } => format!("ip-positions:{host_id}"),
            LockKey::JunctionSync { parent, parent_id } => {
                let parent: &'static str = parent.into();
                format!("junction-sync:{parent}:{parent_id}")
            }
            LockKey::ApiKey(id) => format!("api-key:{id}"),
            LockKey::Organization(id) => format!("organization:{id}"),
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

/// Holds a session-scoped advisory lock on a dedicated pool connection.
///
/// Call [`release`](Self::release) on success paths; `Drop` covers
/// error/cancel paths by releasing asynchronously (and closing the
/// connection if the unlock fails, so a lock-holding connection can never
/// return to the pool).
pub struct SessionLockGuard {
    key: LockKey,
    lock: PgAdvisoryLock,
    conn: Option<PoolConnection<Postgres>>,
}

impl SessionLockGuard {
    /// Release the lock and return the connection to the pool.
    pub async fn release(mut self) -> Result<(), LockError> {
        if let Some(conn) = self.conn.take() {
            // Returns the connection, which drops back to the pool here.
            self.lock.force_release(conn).await?;
        }
        Ok(())
    }

    pub fn key(&self) -> LockKey {
        self.key
    }
}

impl Drop for SessionLockGuard {
    fn drop(&mut self) {
        if let Some(mut conn) = self.conn.take() {
            let pg_key = self.key.pg_key();
            tokio::spawn(async move {
                let unlocked = sqlx::query("SELECT pg_advisory_unlock($1)")
                    .bind(pg_key)
                    .execute(&mut *conn)
                    .await;
                if unlocked.is_err() {
                    // Never return a possibly-still-locked connection to the
                    // pool; closing the session releases its advisory locks
                    // server-side.
                    let _ = conn.close().await;
                }
            });
        }
    }
}

/// Acquire a session-scoped advisory lock, polling `pg_try_advisory_lock`
/// with backoff until `timeout`. The returned guard holds one pool
/// connection for the duration of the critical section.
pub async fn session_lock(
    pool: &PgPool,
    key: LockKey,
    timeout: Duration,
) -> Result<SessionLockGuard, LockError> {
    let lock = key.to_pg_lock();
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let conn = pool.acquire().await?;
        let acquired = lock.try_acquire(conn).await?;
        match acquired {
            Either::Left(guard) => {
                // Take over release management from sqlx's guard: `leak()`
                // hands the (still locked) connection back without queuing
                // an unlock. Consuming the guard also ends its borrow of
                // `lock`, letting us move `lock` into our own guard.
                let conn = guard.leak();
                return Ok(SessionLockGuard {
                    key,
                    // Reconstructed (cheap, deterministic) rather than moved:
                    // the borrow checker pins `lock` to the guard's lifetime.
                    lock: key.to_pg_lock(),
                    conn: Some(conn),
                });
            }
            // Not acquired: connection goes straight back to the pool so
            // waiting never starves the lock holder of connections.
            Either::Right(conn) => drop(conn),
        }
        if tokio::time::Instant::now() + RETRY_INTERVAL > deadline {
            return Err(LockError::Timeout(timeout, key));
        }
        tokio::time::sleep(RETRY_INTERVAL).await;
    }
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
    let result = sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(key.pg_key())
        .execute(&mut **tx)
        .await;
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
    use crate::tests::setup_test_db;

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
            LockKey::HostDedup {
                network_id: uuid_a(),
            }
            .pg_key(),
            LockKey::Service(uuid_a()).pg_key(),
            LockKey::ServiceDedup { host_id: uuid_a() }.pg_key(),
            LockKey::DependencyMembers {
                network_id: uuid_a(),
            }
            .pg_key(),
            LockKey::IpPositions { host_id: uuid_a() }.pg_key(),
            LockKey::JunctionSync {
                parent: EntityDiscriminants::Host,
                parent_id: uuid_a(),
            }
            .pg_key(),
            LockKey::ApiKey(uuid_a()).pg_key(),
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

        // Drop releases via a spawned task; poll until it lands.
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
        let b = LockKey::HostDedup {
            network_id: uuid_a(),
        };

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

    /// The whole point: exclusion must hold across separate pools (i.e.
    /// separate backend instances), not just within one process.
    #[tokio::test]
    async fn test_exclusion_across_pools() {
        let (pool_a, url, _container) = setup_test_db().await;
        let pool_b = PgPool::connect(&url).await.unwrap();
        let key = LockKey::HostDedup {
            network_id: Uuid::new_v4(),
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
