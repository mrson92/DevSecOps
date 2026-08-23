use std::sync::Arc;
use chrono::Utc;
use sqlx::SqlitePool;
use tracing::{info, warn, error};
use uuid::Uuid;

use crate::cel::CelEvaluator;
use crate::types::{DetectionResult, LogEntry, RuleContext};
use aads_core::error::AppError;
use aads_core::models::Rule;
use aads_core::state::ElasticSearchClientTrait;

pub struct RuleEngine {
    db: SqlitePool,
    es: Arc<dyn ElasticSearchClientTrait>,
    evaluator: CelEvaluator,
}

impl RuleEngine {
    pub fn new(db: SqlitePool, es: Arc<dyn ElasticSearchClientTrait>) -> Self {
        Self {
            db,
            es,
            evaluator: CelEvaluator::new(),
        }
    }

    pub async fn load_rules(&mut self) -> Result<Vec<Rule>, AppError> {
        let rules = sqlx::query_as::<_, Rule>("SELECT * FROM rules WHERE enabled = 1")
            .fetch_all(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to load rules: {}", e)))?;

        for rule in &rules {
            if let Err(e) = self.evaluator.compile(&rule.id, &rule.condition) {
                warn!("Failed to compile rule {}: {}", rule.id, e);
            }
        }

        info!("Loaded {} enabled rules", rules.len());
        Ok(rules)
    }

    pub async fn execute_rule(&self, rule: &Rule, logs: Vec<LogEntry>) -> Result<DetectionResult, AppError> {
        let log_values: Vec<serde_json::Value> = logs.iter().map(|l| {
            serde_json::to_value(l).unwrap_or_default()
        }).collect();

        let window_end = Utc::now().to_rfc3339();
        let window_start = Utc::now()
            .checked_sub_signed(chrono::Duration::seconds(rule.window_sec as i64))
            .unwrap_or_default()
            .to_rfc3339();

        let context = RuleContext {
            logs: log_values,
            window_start,
            window_end,
            group_key: None,
        };

        let detected = self.evaluator.evaluate(&rule.id, &context)
            .map_err(|e| AppError::RuleEngine(format!("Evaluation failed: {}", e)))?;

        Ok(DetectionResult {
            rule_id: rule.id.clone(),
            rule_name: rule.name.clone(),
            severity: rule.severity.clone(),
            detected,
            matched_count: if detected { logs.len() as u32 } else { 0 },
            group_key: context.group_key,
            matched_entries: if detected { logs } else { vec![] },
            timestamp: Utc::now().to_rfc3339(),
        })
    }

    pub async fn save_detection(&self, result: &DetectionResult) -> Result<String, AppError> {
        if !result.detected {
            return Ok(String::new());
        }

        let detection_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"INSERT INTO detections (id, rule_id, rule_version, detected_at, window_start, window_end, matched_count, group_key, context, status, created_at)
               VALUES (?, ?, 1, ?, ?, ?, ?, ?, ?, 'open', ?)"#
        )
        .bind(&detection_id)
        .bind(&result.rule_id)
        .bind(&result.timestamp)
        .bind(&result.timestamp)
        .bind(&result.timestamp)
        .bind(result.matched_count as i32)
        .bind(&result.group_key)
        .bind(serde_json::to_string(&result.matched_entries).unwrap_or_default())
        .bind(&now)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to save detection: {}", e)))?;

        info!("Saved detection {} for rule {}", detection_id, result.rule_id);
        Ok(detection_id)
    }

    pub async fn fetch_logs_from_es(&self, rule: &Rule) -> Result<Vec<LogEntry>, AppError> {
        let query = serde_json::json!({
            "query": {
                "bool": {
                    "filter": [
                        {
                            "range": {
                                "@timestamp": {
                                    "gte": format!("now-{}s", rule.window_sec)
                                }
                            }
                        }
                    ]
                }
            },
            "size": 10000
        });

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.es.search("logs", query)
        )
        .await
        .map_err(|_| AppError::ElasticSearch("ES search timeout".to_string()))?
        .map_err(|e| AppError::ElasticSearch(format!("ES search error: {}", e)))?;
        
        let hits = result["hits"]["hits"].as_array()
            .map(|arr| arr.clone())
            .unwrap_or_default();

        let mut logs = Vec::new();
        for hit in hits {
            if let Some(source) = hit.get("_source") {
                let log_entry = LogEntry {
                    timestamp: source["@timestamp"].as_str().unwrap_or("").to_string(),
                    source: source["source"].as_str().unwrap_or("").to_string(),
                    client_ip: source["network"]["client"]["ip"].as_str().unwrap_or("").to_string(),
                    method: source["http"]["request"]["method"].as_str().unwrap_or("").to_string(),
                    path: source["http"]["request"]["path"].as_str().unwrap_or("").to_string(),
                    query: source["http"]["request"]["query"].as_str().map(|s| s.to_string()),
                    status_code: source["http"]["response"]["status_code"].as_u64().unwrap_or(0) as u16,
                    response_size: source["http"]["response"]["size"].as_u64().unwrap_or(0),
                    user_agent: source["http"]["user_agent"]["original"].as_str().map(|s| s.to_string()),
                    user_id: source["app"]["user"]["id"].as_str().map(|s| s.to_string()),
                    response_time: source["http"]["response"]["time"].as_f64(),
                    extra: None,
                };
                logs.push(log_entry);
            }
        }

        Ok(logs)
    }

    pub async fn run_all_rules(&mut self) -> Result<Vec<DetectionResult>, AppError> {
        let rules = self.load_rules().await?;
        let mut results = Vec::new();

        for rule in &rules {
            match self.fetch_logs_from_es(rule).await {
                Ok(logs) => {
                    match self.execute_rule(rule, logs).await {
                        Ok(result) => {
                            if result.detected {
                                if let Err(e) = self.save_detection(&result).await {
                                    error!("Failed to save detection for rule {}: {}", rule.id, e);
                                }
                            }
                            results.push(result);
                        }
                        Err(e) => {
                            warn!("Failed to execute rule {}: {}", rule.id, e);
                            results.push(DetectionResult {
                                rule_id: rule.id.clone(),
                                rule_name: rule.name.clone(),
                                severity: rule.severity.clone(),
                                detected: false,
                                matched_count: 0,
                                group_key: None,
                                matched_entries: vec![],
                                timestamp: Utc::now().to_rfc3339(),
                            });
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to fetch logs for rule {}: {}", rule.id, e);
                    results.push(DetectionResult {
                        rule_id: rule.id.clone(),
                        rule_name: rule.name.clone(),
                        severity: rule.severity.clone(),
                        detected: false,
                        matched_count: 0,
                        group_key: None,
                        matched_entries: vec![],
                        timestamp: Utc::now().to_rfc3339(),
                    });
                }
            }
        }

        Ok(results)
    }
}
