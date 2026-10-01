pub mod handlers;
pub mod service;
pub mod subscriber;

use metrics::{Unit, describe_counter, describe_gauge, describe_histogram};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder};

/// The exporter configuration: every histogram gets explicit buckets, so it is exported as a
/// Prometheus histogram rather than the exporter's default summary. Summaries can't be aggregated
/// across servers or time ranges; buckets can.
pub fn prometheus_builder() -> PrometheusBuilder {
    let buckets: [(&str, &[f64]); 5] = [
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
        (
            "http_request_duration_seconds",
            &[
                0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ],
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
        "HTTP request latency"
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
        "Event bus subscriber failures; the event was not applied by that subscriber"
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
        });
        let rendered = handle.render();
        for name in [
            "http_request_duration_seconds",
            "scanopy_link_resolution_duration_seconds",
            "http_response_size_bytes",
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
}
