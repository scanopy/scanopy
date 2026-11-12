//! Per-scan timestamp normalization.
//!
//! `ScanContext` carries the single `scan_time` captured at the top of a
//! discovery submission so every entity created or refreshed during that
//! submission shares one timestamp. Without it, each entity's `crate::server::shared::clock::now()`
//! call drifts by microseconds-to-milliseconds across the host + children
//! tree, which makes session-window timestamp filters fuzzy at the
//! boundaries (per-scan diff queries become unreliable when entities from
//! the same scan straddle the [`started_at`, `finished_at`] cut).
//!
//! Scope of stamping: when a `ScanContext` is provided to
//! `HostService::discover_host`, the handler stamps `created_at`,
//! `valid_from`, and `last_seen_at` on the incoming host plus every child
//! entity (ip_addresses, ports, services, interfaces, subnets) to
//! `scan_time` BEFORE the entities reach the storage layer. Upsert paths
//! that refresh `last_seen_at` on a matched live row use the incoming
//! entity's already-stamped value, so all writes within one scan share
//! the same timestamp.

use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub struct ScanContext {
    /// Single timestamp stamped on every entity in this submission. Captured
    /// once at the discovery handler entry point.
    pub scan_time: DateTime<Utc>,
    /// Daemon that submitted this scan. Available for downstream use (e.g.
    /// to audit "which daemon last saw this entity") though the discovery
    /// FK columns already carry that via the historical Discovery row.
    pub daemon_id: Uuid,
}

impl ScanContext {
    pub fn new(daemon_id: Uuid) -> Self {
        Self {
            scan_time: crate::server::shared::clock::now(),
            daemon_id,
        }
    }
}
