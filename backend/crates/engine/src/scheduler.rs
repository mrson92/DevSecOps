use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{interval, Duration};
use tracing::{info, error, warn};

use crate::agent_runner::AgentRunner;
use crate::engine::RuleEngine;
use crate::stat::{build_security_stats, security_stat_mapping};
use crate::types::DetectionResult;
use aads_core::error::AppError;
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

                    // 검출 결과를 AI 검토용 최적화 통계(security_stat)로 변환해 ES에 적재.
                    // raw 로그가 아닌 집계·요약 데이터만 적재한다.
                    if let Err(e) = index_security_stats(&state, &results).await {
                        error!("Failed to index security stats: {}", e);
                    }
                }
                Err(e) => {
                    error!("Scheduler cycle failed: {}", e);
                }
            }

            let agent_runner = AgentRunner::new(state.db.clone());
            match agent_runner.run_scheduled_agents().await {
                Ok(runs) => {
                    if !runs.is_empty() {
                        info!("Scheduler: {} agents executed", runs.len());
                    }
                }
                Err(e) => {
                    error!("Scheduler agent execution failed: {}", e);
                }
            }
        });
    }
}

/// 검출 결과들을 AI 검토용 `security_stat` 인덱스로 적재한다.
///
/// 인덱스가 없으면 생성한 뒤 bulk 적재한다. 실패해도 검출/리포트 흐름은
/// 중단하지 않도록 호출부는 best-effort로 처리한다.
async fn index_security_stats(
    state: &AppState,
    results: &[DetectionResult],
) -> Result<(), AppError> {
    let engine = RuleEngine::new(state.db.clone(), state.es.clone());
    let rules = engine.load_rules().await?;

    let stats = build_security_stats(results, &rules);
    if stats.is_empty() {
        return Ok(());
    }

    // 인덱스가 없으면 생성.
    if !state.es.index_exists("security_stat").await? {
        let _ = state.es.create_index("security_stat", security_stat_mapping()).await?;
    }

    let docs: Vec<(String, serde_json::Value)> = stats
        .iter()
        .map(|s| {
            let id = s.stat_id.clone();
            let doc = serde_json::to_value(s).unwrap_or_default();
            (id, doc)
        })
        .collect();

    let resp = state.es.bulk_index("security_stat", docs).await?;
    let ok = resp.get("errors").and_then(|e| e.as_bool()).unwrap_or(true);
    if !ok {
        // 첫 번째 오류 항목의 사유를 로그로 남겨 디버깅을 돕는다.
        if let Some(items) = resp.get("items").and_then(|v| v.as_array()) {
            for item in items {
                if let Some(err) = item["index"]["error"]["reason"].as_str() {
                    return Err(AppError::Internal(format!(
                        "Bulk index security_stat error: {}",
                        err
                    )));
                }
            }
        }
        return Err(AppError::Internal("Bulk index reported errors".into()));
    }

    info!("Indexed {} security stats", stats.len());
    Ok(())
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
