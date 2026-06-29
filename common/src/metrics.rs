use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder};
use std::net::SocketAddr;

/// Install the Prometheus exporter, process collector, and tokio runtime
/// metrics collector. Call once at startup before any `metrics::*` macros.
pub fn init(port: u16) {
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();

    PrometheusBuilder::new()
        // Without explicit buckets, histogram! renders as a client-side
        // sliding-window summary whose quantiles decay to 0 between samples.
        // That's wrong for low-frequency operations (bet_scan/housekeeping
        // run every few minutes) — fixed buckets make this a real,
        // non-decaying Prometheus histogram instead.
        .set_buckets_for_metric(
            Matcher::Full("bot_operation_duration_seconds".to_string()),
            &[
                0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0, 120.0,
            ],
        )
        .expect("invalid bucket config for bot_operation_duration_seconds")
        .with_http_listener(addr)
        .install()
        .expect("failed to install Prometheus exporter");

    // Process-level metrics (RSS, CPU, open FDs, threads, etc.)
    let collector = metrics_process::Collector::default();
    collector.describe();
    collector.collect();

    tracing::info!(%addr, "Prometheus metrics server started");
}

/// Spawn a background task that polls tokio runtime metrics every 10s
/// and records them into the `metrics` facade.
pub fn spawn_tokio_collector(runtime_monitor: tokio_metrics::RuntimeMonitor) {
    tokio::spawn(async move {
        for interval in runtime_monitor.intervals() {
            // Point-in-time values → gauges
            gauge!("tokio_workers_count").set(interval.workers_count as f64);
            gauge!("tokio_live_tasks_count").set(interval.live_tasks_count as f64);
            gauge!("tokio_global_queue_depth").set(interval.global_queue_depth as f64);
            gauge!("tokio_total_local_queue_depth").set(interval.total_local_queue_depth as f64);
            gauge!("tokio_blocking_queue_depth").set(interval.blocking_queue_depth as f64);
            gauge!("tokio_blocking_threads_count").set(interval.blocking_threads_count as f64);
            gauge!("tokio_mean_poll_duration_seconds")
                .set(interval.mean_poll_duration.as_secs_f64());
            gauge!("tokio_busy_ratio").set(interval.busy_ratio());

            // Per-interval deltas → counters (so rate() works in dashboards)
            counter!("tokio_total_park_count").increment(interval.total_park_count);
            counter!("tokio_total_noop_count").increment(interval.total_noop_count);
            counter!("tokio_total_steal_count").increment(interval.total_steal_count);
            counter!("tokio_total_steal_operations").increment(interval.total_steal_operations);
            counter!("tokio_total_polls_count").increment(interval.total_polls_count);
            counter!("tokio_total_local_schedule_count")
                .increment(interval.total_local_schedule_count);
            counter!("tokio_num_remote_schedules").increment(interval.num_remote_schedules);
            counter!("tokio_total_overflow_count").increment(interval.total_overflow_count);
            counter!("tokio_budget_forced_yield_count")
                .increment(interval.budget_forced_yield_count);
            counter!("tokio_io_driver_ready_count").increment(interval.io_driver_ready_count);
            gauge!("tokio_total_busy_duration_seconds")
                .set(interval.total_busy_duration.as_secs_f64());

            // Refresh process metrics each cycle
            metrics_process::Collector::default().collect();

            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    });
}

// ---------------------------------------------------------------------------
// Application-level metric helpers
// ---------------------------------------------------------------------------

/// Record a completed scan cycle.
pub fn record_scan(markets_scanned: u64, news_total: u64, news_new: u64, signals: u64) {
    counter!("bot_scans_total").increment(1);
    counter!("bot_markets_scanned_total").increment(markets_scanned);
    counter!("bot_news_fetched_total").increment(news_total);
    counter!("bot_news_new_total").increment(news_new);
    counter!("bot_signals_found_total").increment(signals);
}

/// Record a bet placement.
pub fn record_bet(strategy: &str, source: &str, cost: f64) {
    counter!("bot_bets_placed_total", "strategy" => strategy.to_string(), "source" => source.to_string()).increment(1);
    histogram!("bot_bet_cost_eur", "strategy" => strategy.to_string()).record(cost);
}

/// Record a bet resolution.
pub fn record_resolution(strategy: &str, won: bool, pnl: f64) {
    let outcome = if won { "win" } else { "loss" };
    counter!("bot_bets_resolved_total", "strategy" => strategy.to_string(), "outcome" => outcome.to_string()).increment(1);
    gauge!("bot_last_pnl_eur", "strategy" => strategy.to_string()).set(pnl);
}

/// Update bankroll gauges.
pub fn record_bankroll(strategy: &str, bankroll: f64) {
    gauge!("bot_bankroll_eur", "strategy" => strategy.to_string()).set(bankroll);
}

/// Update total bankroll gauge.
pub fn record_total_bankroll(total: f64) {
    gauge!("bot_bankroll_total_eur").set(total);
}

/// Record open bets count.
pub fn record_open_bets(count: u64) {
    gauge!("bot_open_bets").set(count as f64);
}

/// Record unrealized PnL.
pub fn record_unrealized_pnl(pnl: f64) {
    gauge!("bot_unrealized_pnl_eur").set(pnl);
}

/// Record a housekeeping cycle.
pub fn record_housekeeping() {
    counter!("bot_housekeeping_cycles_total").increment(1);
}

/// Record a heartbeat.
pub fn record_heartbeat() {
    counter!("bot_heartbeats_total").increment(1);
}

/// Record WS alert processing.
pub fn record_ws_alert(had_signal: bool) {
    counter!("bot_ws_alerts_total").increment(1);
    if had_signal {
        counter!("bot_ws_signals_total").increment(1);
    }
}

/// Record ML model sidecar status: age in seconds and whether it's reachable.
pub fn record_model_status(age_secs: Option<f64>) {
    match age_secs {
        Some(age) => {
            gauge!("bot_model_age_seconds").set(age);
            gauge!("bot_model_up").set(1.0);
        }
        None => {
            gauge!("bot_model_up").set(0.0);
        }
    }
}

/// Record runtime_config polling health.
pub fn record_runtime_config_status(ok: bool, changed: bool) {
    gauge!("bot_runtime_config_stale").set(if ok { 0.0 } else { 1.0 });
    if ok {
        counter!("bot_runtime_config_reload_checks_total").increment(1);
    } else {
        counter!("bot_runtime_config_reload_errors_total").increment(1);
    }
    if changed {
        counter!("bot_runtime_config_reloads_total").increment(1);
    }
}

/// Record a duration histogram for the given metric name.
pub fn record_duration(name: &'static str, duration: std::time::Duration) {
    histogram!(name).record(duration.as_secs_f64());
}

/// Record the duration of a named operation/step as
/// `bot_operation_duration_seconds{operation, step}`. Additive to the
/// per-loop `bot_*_duration_seconds` histograms above — gives per-substep
/// granularity (fetch_markets, predict_batch, place_bet, ...) for the
/// dashboard's latency card.
pub fn record_operation_duration(operation: &'static str, step: &'static str, duration_secs: f64) {
    histogram!(
        "bot_operation_duration_seconds",
        "operation" => operation,
        "step" => step,
    )
    .record(duration_secs);
}
