pub mod db;
pub mod dependency;
pub mod handlers;
pub mod service;
pub mod subscriber;

use metrics::{Unit, describe_counter, describe_gauge, describe_histogram};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder};

/// The exporter configuration: every histogram gets explicit buckets, so it is exported as a
/// Prometheus histogram rather than the exporter's default summary. Summaries can't be aggregated
/// across servers or time ranges; buckets can.
pub fn prometheus_builder() -> PrometheusBuilder {
    let buckets: [(&str, &[f64]); 8] = [
        // 30s to the 6h default scan ceiling and past it.
        (
            "scanopy_discovery_session_duration_seconds",
            &[
                30.0, 60.0, 120.0, 300.0, 600.0, 1800.0, 3600.0, 7200.0, 14400.0, 21600.0, 43200.0,
            ],
        ),
        // Brackets the 10s resolution budget, so the share that hits it is the `le="10"` bucket.
        (
            "scanopy_link_resolution_duration_seconds",
            &[0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 15.0],
        ),
        // Every bucket is a series per route, so the edges sit where answers change: 2.5s stays
        // for dashboards that read it, and 3s and 4s stop a p99 being interpolated across
        // 2.5–5s. 30s is sqlx's acquire timeout.
        (
            "http_request_duration_seconds",
            &[0.025, 0.1, 0.25, 0.5, 1.0, 2.5, 3.0, 4.0, 5.0, 10.0, 30.0],
        ),
        // Up to sqlx's 30s acquire timeout.
        ("scanopy_db_pool_acquire_seconds", &WAIT_SECONDS_BUCKETS),
        // Up to the 30s default lock timeout.
        ("scanopy_lock_wait_seconds", &WAIT_SECONDS_BUCKETS),
        (
            "scanopy_dependency_request_duration_seconds",
            &[0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0],
        ),
        // 256B to 16MB, ×4 per bucket.
        ("http_request_size_bytes", &SIZE_BUCKETS),
        ("http_response_size_bytes", &SIZE_BUCKETS),
    ];

    buckets
        .into_iter()
        .fold(PrometheusBuilder::new(), |builder, (name, values)| {
            builder
                .set_buckets_for_metric(Matcher::Full(name.to_string()), values)
                .expect("bucket lists are non-empty")
        })
}

const WAIT_SECONDS_BUCKETS: [f64; 6] = [0.005, 0.05, 0.25, 1.0, 5.0, 30.0];

const SIZE_BUCKETS: [f64; 9] = [
    256.0, 1024.0, 4096.0, 16384.0, 65536.0, 262144.0, 1048576.0, 4194304.0, 16777216.0,
];

/// `# HELP` text and units for every metric the server records. Must run after the recorder is
/// installed: descriptions go to whichever recorder is global at the time.
pub fn describe_metrics() {
    describe_gauge!(
        "http_requests_in_flight",
        Unit::Count,
        "Requests being handled right now"
    );
    describe_counter!(
        "http_requests_total",
        Unit::Count,
        "Completed HTTP requests by route template, status and caller"
    );
    describe_histogram!(
        "http_request_duration_seconds",
        Unit::Seconds,
        "HTTP request latency until the response starts, by route and method. Excludes SSE streams and routes that wait on purpose (daemon reachability tests)"
    );
    describe_counter!(
        "http_slow_requests_total",
        Unit::Count,
        "HTTP requests that took 1s or more, by route"
    );
    describe_counter!(
        "http_slow_request_milliseconds_total",
        Unit::Milliseconds,
        "Time of HTTP requests that took 1s or more, by route and phase: acquire (waiting for a pool connection), query (statements running, including row-lock waits), lock_wait (waiting for an advisory lock), other (the rest)"
    );
    describe_counter!(
        "http_slow_request_queries_total",
        Unit::Count,
        "SQL statements run by HTTP requests that took 1s or more, by route"
    );
    describe_histogram!(
        "scanopy_db_pool_acquire_seconds",
        Unit::Seconds,
        "Time to get a connection from the main database pool, by scope (request, or other: jobs, subscribers, SSE streams, sessions)"
    );
    describe_gauge!(
        "scanopy_db_pool_connections",
        Unit::Count,
        "Open connections in the main database pool by state (idle, in_use), read at scrape"
    );
    describe_gauge!(
        "scanopy_db_pool_max_connections",
        Unit::Count,
        "Most connections the main database pool opens"
    );
    describe_histogram!(
        "scanopy_lock_wait_seconds",
        Unit::Seconds,
        "Time to take an advisory lock, by lock"
    );
    describe_gauge!(
        "scanopy_runtime_workers",
        Unit::Count,
        "Tokio runtime worker threads"
    );
    describe_counter!(
        "scanopy_runtime_worker_busy_seconds_total",
        Unit::Seconds,
        "Time Tokio workers spent running tasks, summed over workers. Divide its rate by scanopy_runtime_workers for the busy share"
    );
    describe_gauge!(
        "scanopy_runtime_global_queue_depth",
        Unit::Count,
        "Tasks waiting in the Tokio runtime's global queue, read at scrape"
    );
    describe_gauge!(
        "scanopy_runtime_alive_tasks",
        Unit::Count,
        "Tokio tasks alive, read at scrape"
    );
    describe_histogram!(
        "scanopy_dependency_request_duration_seconds",
        Unit::Seconds,
        "Outbound call time to external dependencies, by dependency"
    );
    describe_histogram!(
        "http_request_size_bytes",
        Unit::Bytes,
        "HTTP request body size from Content-Length"
    );
    describe_histogram!(
        "http_response_size_bytes",
        Unit::Bytes,
        "HTTP response body size from Content-Length"
    );
    describe_counter!(
        "scanopy_entity_events_total",
        Unit::Count,
        "Entity events on the event bus by entity type and operation"
    );
    describe_counter!(
        "scanopy_events_total",
        Unit::Count,
        "Non-entity events on the event bus by category and operation"
    );
    describe_counter!(
        "scanopy_event_subscriber_errors_total",
        Unit::Count,
        "Event bus subscriber deliveries that hit an error, by final outcome (recovered after retry, exhausted, not_retried)"
    );
    describe_counter!(
        "scanopy_discovery_warnings_total",
        Unit::Count,
        "Discovery warning occurrences by code and integration"
    );
    describe_counter!(
        "scanopy_discovery_terminal_total",
        Unit::Count,
        "Discovery sessions ended, by terminal phase and reason"
    );
    describe_histogram!(
        "scanopy_discovery_session_duration_seconds",
        Unit::Seconds,
        "Discovery session wall-clock time from start to finish"
    );
    describe_gauge!(
        "scanopy_discovery_sessions_active",
        Unit::Count,
        "Live discovery sessions by phase, refreshed every 60s"
    );
    describe_gauge!(
        "scanopy_daemons",
        Unit::Count,
        "Daemons seen in the last 24h by the exact version each reported, refreshed every 15m"
    );
    describe_counter!(
        "scanopy_discovery_finish_errors_total",
        Unit::Count,
        "Finished discovery sessions the server failed to record, by step (finalize_scan, history_record, warning_records, discovery_fks, digest)"
    );
    describe_counter!(
        "scanopy_dependency_requests_total",
        Unit::Count,
        "Outbound calls to external dependencies (OIDC, SMTP, SNS, Stripe, PostHog, license server, GitHub), by dependency and outcome"
    );
    describe_counter!(
        "scanopy_email_feedback_total",
        Unit::Count,
        "SES bounce and complaint recipients reported over SNS, by type and bounce type"
    );
    describe_histogram!(
        "scanopy_link_resolution_duration_seconds",
        Unit::Seconds,
        "Post-scan link resolution pass time by protocol, including passes that hit the budget"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every configured histogram renders as `_bucket` series, not as a summary's quantiles.
    #[test]
    fn configured_histograms_export_buckets() {
        let recorder = prometheus_builder().build_recorder();
        let handle = recorder.handle();
        metrics::with_local_recorder(&recorder, || {
            metrics::histogram!("http_request_duration_seconds", "path" => "/api/hosts")
                .record(0.2);
            metrics::histogram!("scanopy_link_resolution_duration_seconds", "protocol" => "lldp")
                .record(3.0);
            metrics::histogram!("http_response_size_bytes").record(2000.0);
            metrics::histogram!("scanopy_db_pool_acquire_seconds", "scope" => "request")
                .record(0.002);
            metrics::histogram!("scanopy_lock_wait_seconds", "lock" => "host").record(0.3);
            metrics::histogram!(
                "scanopy_dependency_request_duration_seconds",
                "dependency" => "stripe"
            )
            .record(0.4);
        });
        let rendered = handle.render();
        for name in [
            "http_request_duration_seconds",
            "scanopy_link_resolution_duration_seconds",
            "http_response_size_bytes",
            "scanopy_db_pool_acquire_seconds",
            "scanopy_lock_wait_seconds",
            "scanopy_dependency_request_duration_seconds",
        ] {
            assert!(
                rendered.contains(&format!("{name}_bucket{{")),
                "{name} has no buckets:\n{rendered}"
            );
            assert!(
                !rendered.contains(&format!("{name}{{quantile=")),
                "{name} is still a summary:\n{rendered}"
            );
        }
    }

    /// A 3.5s request lands between bucket edges at 3s and 4s, so a quantile near it is not
    /// interpolated across 2.5–5s.
    #[test]
    fn request_duration_resolves_seconds_between_2_5_and_5() {
        let recorder = prometheus_builder().build_recorder();
        let handle = recorder.handle();
        metrics::with_local_recorder(&recorder, || {
            metrics::histogram!("http_request_duration_seconds", "path" => "/x").record(3.5);
        });
        let rendered = handle.render();
        for (le, count) in [("2.5", 0), ("3", 0), ("4", 1), ("5", 1)] {
            let line =
                format!(r#"http_request_duration_seconds_bucket{{path="/x",le="{le}"}} {count}"#);
            assert!(rendered.contains(&line), "missing {line}:\n{rendered}");
        }
    }
}
