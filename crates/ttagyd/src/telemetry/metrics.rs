//! TTAgy Prometheus Metrics Collector & OpenMetrics Formatter

use serde::Serialize;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[derive(Default)]
pub struct MetricsCollector {
    start_time: Option<Instant>,
    pub total_requests: AtomicU64,
    pub successful_requests: AtomicU64,
    pub failed_requests: AtomicU64,
    pub total_prompt_tokens: AtomicU64,
    pub total_output_tokens: AtomicU64,
    pub total_thinking_tokens: AtomicU64,
    pub active_workers: AtomicUsize,
    pub idle_workers: AtomicUsize,
    pub deadlocks_prevented: AtomicUsize,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricsSnapshot {
    pub uptime_secs: u64,
    pub total_requests: u64,
    pub total_prompt_tokens: u64,
    pub total_output_tokens: u64,
    pub total_thinking_tokens: u64,
    pub active_workers: usize,
    pub idle_workers: usize,
    pub available_permits: usize,
    pub deadlocks_prevented: usize,
}

impl MetricsCollector {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            start_time: Some(Instant::now()),
            ..Default::default()
        })
    }

    pub fn record_request(&self, success: bool, prompt_tokens: usize, output_tokens: usize, thinking_tokens: usize) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        if success {
            self.successful_requests.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_requests.fetch_add(1, Ordering::Relaxed);
        }
        self.total_prompt_tokens.fetch_add(prompt_tokens as u64, Ordering::Relaxed);
        self.total_output_tokens.fetch_add(output_tokens as u64, Ordering::Relaxed);
        self.total_thinking_tokens.fetch_add(thinking_tokens as u64, Ordering::Relaxed);
    }

    pub fn get_snapshot(&self, available_permits: usize) -> MetricsSnapshot {
        let uptime_secs = self.start_time.map(|t| t.elapsed().as_secs()).unwrap_or(0);
        MetricsSnapshot {
            uptime_secs,
            total_requests: self.total_requests.load(Ordering::Relaxed),
            total_prompt_tokens: self.total_prompt_tokens.load(Ordering::Relaxed),
            total_output_tokens: self.total_output_tokens.load(Ordering::Relaxed),
            total_thinking_tokens: self.total_thinking_tokens.load(Ordering::Relaxed),
            active_workers: self.active_workers.load(Ordering::Relaxed),
            idle_workers: self.idle_workers.load(Ordering::Relaxed),
            available_permits,
            deadlocks_prevented: self.deadlocks_prevented.load(Ordering::Relaxed),
        }
    }

    pub fn render_prometheus(&self, available_permits: usize) -> String {
        let snap = self.get_snapshot(available_permits);
        let mut out = String::new();

        out.push_str("# HELP ttagy_uptime_seconds Daemon uptime in seconds\n");
        out.push_str("# TYPE ttagy_uptime_seconds gauge\n");
        out.push_str(&format!("ttagy_uptime_seconds {}\n\n", snap.uptime_secs));

        out.push_str("# HELP ttagy_requests_total Total number of processed chat requests\n");
        out.push_str("# TYPE ttagy_requests_total counter\n");
        out.push_str(&format!("ttagy_requests_total {}\n\n", snap.total_requests));

        out.push_str("# HELP ttagy_tokens_total Total tokens processed\n");
        out.push_str("# TYPE ttagy_tokens_total counter\n");
        out.push_str(&format!("ttagy_tokens_total{{type=\"prompt\"}} {}\n", snap.total_prompt_tokens));
        out.push_str(&format!("ttagy_tokens_total{{type=\"output\"}} {}\n", snap.total_output_tokens));
        out.push_str(&format!("ttagy_tokens_total{{type=\"thinking\"}} {}\n\n", snap.total_thinking_tokens));

        out.push_str("# HELP ttagy_workers Current worker counts\n");
        out.push_str("# TYPE ttagy_workers gauge\n");
        out.push_str(&format!("ttagy_workers{{state=\"active\"}} {}\n", snap.active_workers));
        out.push_str(&format!("ttagy_workers{{state=\"idle\"}} {}\n\n", snap.idle_workers));

        out.push_str("# HELP ttagy_concurrency_available_permits Available semaphore permits\n");
        out.push_str("# TYPE ttagy_concurrency_available_permits gauge\n");
        out.push_str(&format!("ttagy_concurrency_available_permits {}\n\n", snap.available_permits));

        out.push_str("# HELP ttagy_deadlocks_prevented_total Total deadlocks prevented in Subagent Mesh\n");
        out.push_str("# TYPE ttagy_deadlocks_prevented_total counter\n");
        out.push_str(&format!("ttagy_deadlocks_prevented_total {}\n", snap.deadlocks_prevented));

        out
    }
}
