//! Site snapshots: point-in-time capture of a site's topology + entity
//! state. Each snapshot is a row in the `snapshots` table; closed entity rows
//! and snapshot topology rows carry `snapshot_id` FKs back to it. Retention
//! becomes a single `DELETE FROM snapshots WHERE taken_at < cutoff` that
//! cascades.
//!
//! The point-in-time read path is the SCD2 substrate: closed entity rows are
//! created at `taken_at` with `valid_to = T`, so `as_of(T)` returns the live
//! row set as it was when the snapshot was taken.
//!
//! Coordination with discovery: `DiscoveryService::try_acquire_site_for_snapshot`
//! / `release_site_for_snapshot` and the `AwaitingSnapshot` discovery phase.
//! The manual-snapshot API handler wraps acquire → run → release around
//! `SnapshotService::run_close_and_clone`.

pub mod handlers;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
