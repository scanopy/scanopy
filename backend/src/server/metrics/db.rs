//! Database time, per request and per pool acquire.
//!
//! sqlx emits a tracing event for every statement it runs (`sqlx::query`, with `elapsed_secs`)
//! and, once `acquire_time_level` is set on the pool, for every connection it hands out
//! (`sqlx::pool::acquire`, with `aquired_after_secs`, sqlx's spelling). [`DbTimingLayer`] reads
//! those two fields and adds them to the request running on the current task, so the
//! request-logging middleware can record how much of each request went to the database:
//!
//! - `http_request_db_seconds{method, path, phase}`: `acquire` (waiting for a pool
//!   connection), `query` (statements running, including Postgres row-lock waits) and
//!   `lock_wait` (waiting for an advisory lock in `shared/storage/lock.rs`)
//! - `http_request_db_queries{method, path}`: statements per request
//! - `scanopy_db_pool_acquire_seconds{scope}`: every acquire, `request` inside a request and
//!   `other` outside one (jobs, event subscribers, SSE streams, the session store)
//!
//! Work a handler hands to `tokio::spawn` runs on another task and counts as `other`.
//!
//! The field names are sqlx 0.8's. A sqlx upgrade that renames them leaves these metrics at
//! zero, so check `logger.rs` and `pool/inner.rs` in sqlx-core when bumping it.

use std::cell::Cell;
use std::future::Future;

use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::filter::Targets;
use tracing_subscriber::layer::{Context, Layer};

const QUERY_TARGET: &str = "sqlx::query";
const QUERY_ELAPSED_FIELD: &str = "elapsed_secs";
const ACQUIRE_TARGET: &str = "sqlx::pool::acquire";
const ACQUIRE_ELAPSED_FIELD: &str = "aquired_after_secs";

/// Database time accumulated by the request running on this task.
#[derive(Default)]
struct RequestDbTime {
    acquire_secs: Cell<f64>,
    query_secs: Cell<f64>,
    lock_wait_secs: Cell<f64>,
    queries: Cell<u64>,
}

tokio::task_local! {
    static REQUEST_DB_TIME: RequestDbTime;
}

/// What one request spent on the database.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct DbTotals {
    pub acquire_secs: f64,
    pub query_secs: f64,
    pub lock_wait_secs: f64,
    pub queries: u64,
}

/// Run `request` with database time attributed to it, and return that time with its output.
pub async fn measure<F: Future>(request: F) -> (F::Output, DbTotals) {
    REQUEST_DB_TIME
        .scope(RequestDbTime::default(), async {
            let output = request.await;
            let totals = REQUEST_DB_TIME.with(|t| DbTotals {
                acquire_secs: t.acquire_secs.get(),
                query_secs: t.query_secs.get(),
                lock_wait_secs: t.lock_wait_secs.get(),
                queries: t.queries.get(),
            });
            (output, totals)
        })
        .await
}

/// Add to the current request's totals. Returns false outside a request.
fn add_to_request(update: impl FnOnce(&RequestDbTime)) -> bool {
    REQUEST_DB_TIME.try_with(update).is_ok()
}

fn scope_label(in_request: bool) -> &'static str {
    if in_request { "request" } else { "other" }
}

/// Record a wait for the advisory lock `lock` (a `LockKey` variant name).
pub fn record_lock_wait(lock: &'static str, secs: f64) {
    let in_request = add_to_request(|t| t.lock_wait_secs.set(t.lock_wait_secs.get() + secs));
    metrics::histogram!(
        "scanopy_lock_wait_seconds",
        "lock" => lock,
        "scope" => scope_label(in_request),
    )
    .record(secs);
}

/// Record a request's database time under its route.
pub fn record_request(method: &str, path: &str, totals: DbTotals) {
    for (phase, secs) in [
        ("acquire", totals.acquire_secs),
        ("query", totals.query_secs),
        ("lock_wait", totals.lock_wait_secs),
    ] {
        metrics::histogram!(
            "http_request_db_seconds",
            "method" => method.to_string(),
            "path" => path.to_string(),
            "phase" => phase,
        )
        .record(secs);
    }
    metrics::histogram!(
        "http_request_db_queries",
        "method" => method.to_string(),
        "path" => path.to_string(),
    )
    .record(totals.queries as f64);
}

/// Reads one `f64` field from an event.
struct SecondsField {
    name: &'static str,
    value: Option<f64>,
}

impl Visit for SecondsField {
    fn record_f64(&mut self, field: &Field, value: f64) {
        if field.name() == self.name {
            self.value = Some(value);
        }
    }

    fn record_debug(&mut self, _field: &Field, _value: &dyn std::fmt::Debug) {}
}

fn seconds_field(event: &Event<'_>, name: &'static str) -> Option<f64> {
    let mut visitor = SecondsField { name, value: None };
    event.record(&mut visitor);
    visitor.value
}

/// Turns sqlx's statement and acquire events into database-time metrics. Install it with
/// [`DbTimingLayer::filter`], which enables only the two sqlx targets it reads.
pub struct DbTimingLayer;

impl DbTimingLayer {
    pub fn filter() -> Targets {
        Targets::new()
            .with_target(QUERY_TARGET, Level::DEBUG)
            .with_target(ACQUIRE_TARGET, Level::TRACE)
    }
}

impl<S: Subscriber> Layer<S> for DbTimingLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        match event.metadata().target() {
            QUERY_TARGET => {
                if let Some(secs) = seconds_field(event, QUERY_ELAPSED_FIELD) {
                    add_to_request(|t| {
                        t.query_secs.set(t.query_secs.get() + secs);
                        t.queries.set(t.queries.get() + 1);
                    });
                }
            }
            ACQUIRE_TARGET => {
                if let Some(secs) = seconds_field(event, ACQUIRE_ELAPSED_FIELD) {
                    let in_request =
                        add_to_request(|t| t.acquire_secs.set(t.acquire_secs.get() + secs));
                    metrics::histogram!(
                        "scanopy_db_pool_acquire_seconds",
                        "scope" => scope_label(in_request),
                    )
                    .record(secs);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::layer::SubscriberExt;

    /// Emits events shaped like sqlx 0.8's (same targets, field names and types).
    fn sqlx_query(secs: f64) {
        tracing::event!(
            target: "sqlx::query",
            Level::DEBUG,
            summary = "select …",
            elapsed_secs = secs,
        );
    }

    fn sqlx_acquire(secs: f64) {
        tracing::event!(
            target: "sqlx::pool::acquire",
            Level::TRACE,
            aquired_after_secs = secs,
            "acquired connection"
        );
    }

    #[tokio::test]
    async fn request_gets_its_own_db_time_and_other_tasks_count_as_other() {
        let recorder = crate::server::metrics::dependency::tests::TestRecorder::new();
        let _metrics = recorder.install();
        let subscriber =
            tracing_subscriber::registry().with(DbTimingLayer.with_filter(DbTimingLayer::filter()));
        let _tracing = tracing::subscriber::set_default(subscriber);

        let ((), totals) = measure(async {
            sqlx_acquire(0.5);
            sqlx_query(0.25);
            sqlx_query(0.125);
            record_lock_wait("host_dedup", 1.0);
        })
        .await;
        // Outside any request: a background job.
        sqlx_acquire(2.0);
        sqlx_query(4.0);

        assert_eq!(
            totals,
            DbTotals {
                acquire_secs: 0.5,
                query_secs: 0.375,
                lock_wait_secs: 1.0,
                queries: 2,
            }
        );
        let rendered = recorder.render();
        assert!(
            rendered.contains(r#"scanopy_db_pool_acquire_seconds_sum{scope="request"} 0.5"#),
            "{rendered}"
        );
        assert!(
            rendered.contains(r#"scanopy_db_pool_acquire_seconds_sum{scope="other"} 2"#),
            "{rendered}"
        );
        assert!(
            rendered
                .contains(r#"scanopy_lock_wait_seconds_sum{lock="host_dedup",scope="request"} 1"#),
            "{rendered}"
        );
    }
}
