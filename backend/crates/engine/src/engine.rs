use std::sync::Arc;
use chrono::Utc;
use sqlx::SqlitePool;
use tracing::{info, warn, error};
use uuid::Uuid;

use crate::rule_eval::NativeRuleEvaluator;
use crate::types::{DetectionResult, LogEntry};
use aads_core::error::AppError;
use aads_core::models::{Rule, DataSource};
use aads_core::state::ElasticSearchClientTrait;

pub struct RuleEngine {
    db: SqlitePool,
    es: Arc<dyn ElasticSearchClientTrait>,
}

impl RuleEngine {
    pub fn new(db: SqlitePool, es: Arc<dyn ElasticSearchClientTrait>) -> Self {
        Self { db, es }
    }

    pub async fn load_rules(&self) -> Result<Vec<Rule>, AppError> {
        let rules = sqlx::query_as::<_, Rule>("SELECT * FROM rules WHERE enabled = 1")
            .fetch_all(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to load rules: {}", e)))?;

        info!("Loaded {} enabled rules", rules.len());
        Ok(rules)
    }

    pub async fn load_data_sources(&self) -> Result<Vec<DataSource>, AppError> {
        let sources = sqlx::query_as::<_, DataSource>(
            "SELECT * FROM data_sources WHERE enabled = true ORDER BY is_primary DESC"
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to load data sources: {}", e)))?;

        info!("Loaded {} enabled data sources", sources.len());
        Ok(sources)
    }

    fn create_es_client_from_config(config: &str) -> Result<Box<dyn ElasticSearchClientTrait>, AppError> {
        let parsed: serde_json::Value = serde_json::from_str(config)
            .map_err(|e| AppError::Internal(format!("Invalid config JSON: {}", e)))?;
        let url = parsed["url"].as_str()
            .ok_or_else(|| AppError::Internal("Missing 'url' in ES config".into()))?;

        let es_config = aads_core::config::ElasticsearchConfig {
            url: url.to_string(),
            username: parsed["username"].as_str().map(|s| s.to_string()),
            password: parsed["password"].as_str().map(|s| s.to_string()),
            index_prefix: parsed["index_prefix"].as_str().unwrap_or("aads").to_string(),
            request_timeout_secs: parsed["request_timeout_secs"].as_u64().unwrap_or(30),
        };

        let client = aads_es::client::ElasticSearchClient::new(&es_config)?;
        Ok(Box::new(client))
    }

    pub async fn execute_rule(&self, rule: &Rule, logs: Vec<LogEntry>) -> Result<DetectionResult, AppError> {
        let evaluator = NativeRuleEvaluator::compile(&rule.condition)
            .map_err(|e| AppError::RuleEngine(format!("Rule compile error: {}", e)))?;

        let threshold = evaluator.get_threshold(&rule.condition);
        let matched_count = evaluator.count_matched(&logs);
        let detected = matched_count as u32 >= threshold;

        let matched_entries = if detected {
            logs.into_iter()
                .filter(|log| evaluator.evaluate(&[log.clone()]))
                .collect()
        } else {
            vec![]
        };

        Ok(DetectionResult {
            rule_id: rule.id.clone(),
            rule_name: rule.name.clone(),
            severity: rule.severity.clone(),
            detected,
            matched_count: matched_count as u32,
            group_key: None,
            matched_entries,
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
            r#"INSERT INTO rule_executions (id, rule_id, rule_version, detected_at, window_start, window_end, matched_count, group_key, context, status, created_at)
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
        self.fetch_logs_from_es_with_client(rule, self.es.as_ref()).await
    }

    async fn fetch_logs_from_es_with_client(&self, rule: &Rule, es_client: &dyn ElasticSearchClientTrait) -> Result<Vec<LogEntry>, AppError> {
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
            es_client.search("aads-logs", query)
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

    pub async fn run_all_rules(&self) -> Result<Vec<DetectionResult>, AppError> {
        let rules = self.load_rules().await?;
        let data_sources = self.load_data_sources().await?;

        let es_sources: Vec<&DataSource> = data_sources.iter()
            .filter(|s| s.r#type == "elasticsearch")
            .collect();

        let mut results = Vec::new();

        for rule in &rules {
            let fetch_result = if let Some(primary) = es_sources.iter().find(|s| s.is_primary) {
                match Self::create_es_client_from_config(&primary.config) {
                    Ok(client) => self.fetch_logs_from_es_with_client(rule, client.as_ref()).await,
                    Err(e) => {
                        warn!("Failed to create ES client from data source '{}': {}", primary.name, e);
                        self.fetch_logs_from_es(rule).await
                    }
                }
            } else if let Some(first) = es_sources.first() {
                match Self::create_es_client_from_config(&first.config) {
                    Ok(client) => self.fetch_logs_from_es_with_client(rule, client.as_ref()).await,
                    Err(e) => {
                        warn!("Failed to create ES client from data source '{}': {}", first.name, e);
                        self.fetch_logs_from_es(rule).await
                    }
                }
            } else {
                self.fetch_logs_from_es(rule).await
            };

            match fetch_result {
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
