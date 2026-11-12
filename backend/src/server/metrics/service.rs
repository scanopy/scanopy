use metrics_exporter_prometheus::PrometheusHandle;

use crate::server::shared::storage::factory::PoolStats;

#[derive(Clone)]
pub struct MetricsService {
    pub handle: PrometheusHandle,
    pool: PoolStats,
}

impl MetricsService {
    pub fn new(handle: PrometheusHandle, pool: PoolStats) -> Self {
        Self { handle, pool }
    }

    /// The metrics as Prometheus text, with the pool and runtime gauges read now.
    pub fn render(&self) -> String {
        self.record_pool_state();
        record_runtime_state();
        self.handle.render()
    }

    fn record_pool_state(&self) {
        let open = self.pool.open();
        let idle = self.pool.idle().min(open);
        metrics::gauge!("scanopy_db_pool_connections", "state" => "idle").set(f64::from(idle));
        metrics::gauge!("scanopy_db_pool_connections", "state" => "in_use")
            .set(f64::from(open - idle));
        metrics::gauge!("scanopy_db_pool_max_connections").set(f64::from(self.pool.max()));
    }
}

/// Tokio's stable runtime metrics. Busy time is cumulative per worker, so it is exported as a
/// counter; its rate divided by the worker count is the share of time workers were busy.
fn record_runtime_state() {
    let Ok(runtime) = tokio::runtime::Handle::try_current() else {
        return;
    };
    let metrics = runtime.metrics();
    let workers = metrics.num_workers();
    let busy: f64 = (0..workers)
        .map(|worker| metrics.worker_total_busy_duration(worker).as_secs_f64())
        .sum();
    metrics::gauge!("scanopy_runtime_workers").set(workers as f64);
    metrics::gauge!("scanopy_runtime_alive_tasks").set(metrics.num_alive_tasks() as f64);
    metrics::gauge!("scanopy_runtime_global_queue_depth").set(metrics.global_queue_depth() as f64);
    // Counters hold u64, so this truncates to whole seconds: under 1s of error per window.
    metrics::counter!("scanopy_runtime_worker_busy_seconds_total").absolute(busy as u64);
}
