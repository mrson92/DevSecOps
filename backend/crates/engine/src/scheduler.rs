use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{interval, Duration};
use tracing::{info, error, warn};

use crate::engine::RuleEngine;
use crate::types::DetectionResult;
use aads_core::state::AppState;

pub struct Scheduler {
    state: AppState,
    interval_secs: u64,
    semaphore: Arc<Semaphore>,
}

impl Scheduler {
    pub fn new(state: AppState, interval_secs: u64, max_concurrent: usize) -> Self {
        Self {
            state,
            interval_secs,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    pub async fn start(&self) {
        let mut tick = interval(Duration::from_secs(self.interval_secs));
        info!("Scheduler started: running every {}s", self.interval_secs);

        loop {
            tick.tick().await;
            self.run_cycle().await;
        }
    }

    async fn run_cycle(&self) {
        let permit = match self.semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                warn!("Scheduler: previous cycle still running, skipping");
                return;
            }
        };

        let state = self.state.clone();
        tokio::spawn(async move {
            let _permit = permit;
            let engine = RuleEngine::new(state.db.clone(), state.es.clone());

            match engine.run_all_rules().await {
                Ok(results) => {
                    let detected = results.iter().filter(|r| r.detected).count();
                    let total_detections: u32 = results.iter().map(|r| r.matched_count).sum();
                    info!(
                        "Scheduler cycle: {} rules evaluated, {} detections, {} total matches",
                        results.len(), detected, total_detections
                    );
                }
                Err(e) => {
                    error!("Scheduler cycle failed: {}", e);
                }
            }
        });
    }
}

pub struct NotificationDispatcher {
    webhook_urls: Vec<String>,
}

impl NotificationDispatcher {
    pub fn new(webhook_urls: Vec<String>) -> Self {
        Self { webhook_urls }
    }

    pub async fn dispatch(&self, detection: &DetectionResult) {
        if !detection.detected {
            return;
        }

        let payload = serde_json::json!({
            "text": format!(
                "🚨 [AADS] {} - {} detected!\nSeverity: {}\nMatched: {} entries",
                detection.rule_name, detection.severity, detection.severity, detection.matched_count
            ),
            "rule_id": detection.rule_id,
            "severity": detection.severity,
            "matched_count": detection.matched_count,
            "timestamp": detection.timestamp,
        });

        for url in &self.webhook_urls {
            match reqwest::Client::new()
                .post(url)
                .json(&payload)
                .timeout(Duration::from_secs(10))
                .send()
                .await
            {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        error!("Webhook dispatch failed: status {}", resp.status());
                    }
                }
                Err(e) => {
                    error!("Webhook dispatch error: {}", e);
                }
            }
        }
    }
}
