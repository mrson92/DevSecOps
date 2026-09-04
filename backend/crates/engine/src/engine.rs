use std::collections::HashMap;
use std::sync::Arc;
use chrono::Utc;
use sqlx::SqlitePool;
use tracing::{info, warn, error};
use uuid::Uuid;
use serde_json::Value;

use crate::rule_eval::NativeRuleEvaluator;
use crate::types::{DetectionResult, LogEntry};
use aads_core::error::AppError;
use aads_core::models::{Rule, DataSource};
use aads_core::state::ElasticSearchClientTrait;

/// HTTP client for ClickHouse queries (shared across requests).
fn clickhouse_http_client() -> Result<reqwest::Client, AppError> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to create HTTP client: {}", e)))
}

/// Deserialized field mapping: standard system field path -> external ES field path.
type FieldMapping = HashMap<String, String>;

/// Parse a stored field_mapping JSON of the form
/// `{ "<standard path>": { "es_field": "<external path>", "es_type": "..." } }`
/// into a simple standard-path -> external-path map. Entries that are plain
/// strings (`"<standard>": "<external>"`) are also accepted.
fn parse_field_mapping(raw: &str) -> FieldMapping {
    let mut out = HashMap::new();
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return out;
    };
    let Some(obj) = value.as_object() else {
        return out;
    };
    for (standard, val) in obj {
        let external = if let Some(es_field) = val.get("es_field").and_then(|v| v.as_str()) {
            es_field
        } else if let Some(s) = val.as_str() {
            s
        } else {
            continue;
        };
        out.insert(standard.clone(), external.to_string());
    }
    out
}

/// Extract the value at the given dotted path from `source`.
fn get_at_path<'a>(source: &'a Value, path: &str) -> &'a Value {
    let mut current = source;
    for segment in path.split('.') {
        current = match current.get(segment) {
            Some(v) => v,
            None => return &Value::Null,
        };
    }
    current
}

/// "mapping-read": read `standard_field` from `source`, applying the field
/// mapping when present (mapping overrides the default path).
fn md<'a>(source: &'a Value, field_mapping: &FieldMapping, standard_field: &str) -> &'a Value {
    let path = field_mapping.get(standard_field).map(|s| s.as_str()).unwrap_or(standard_field);
    get_at_path(source, path)
}

fn opt_str(value: &Value) -> Option<String> {
    value.as_str().map(|s| s.to_string())
}

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

        let group_key = if detected {
            Self::derive_group_key(&rule.group_by, &logs)
        } else {
            None
        };

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
            group_key,
            matched_entries,
            timestamp: Utc::now().to_rfc3339(),
        })
    }

    fn derive_group_key(group_by: &str, entries: &[LogEntry]) -> Option<String> {
        let fields: Vec<String> = serde_json::from_str(group_by).unwrap_or_default();
        let first = entries.first()?;
        if fields.is_empty() {
            return None;
        }

        let mut values = Vec::new();
        for field in fields {
            let value = match field.as_str() {
                "network.client.ip" => first.client_ip.clone(),
                "http.request.path" => first.path.clone(),
                "http.request.method" => first.method.clone(),
                "http.request.query" => first.query.clone().unwrap_or_default(),
                "http.response.status_code" => first.status_code.to_string(),
                "http.response.size" => first.response_size.to_string(),
                "http.user_agent.original" => first.user_agent.clone().unwrap_or_default(),
                "app.user.id" => first.user_id.clone().unwrap_or_default(),
                _ => String::new(),
            };
            if !value.is_empty() {
                values.push(value);
            }
        }

        if values.is_empty() { None } else { Some(values.join("|")) }
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
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to save detection: {}", e)))?;

        info!("Saved detection {} for rule {}", detection_id, result.rule_id);
        Ok(detection_id)
    }

    pub async fn fetch_logs_from_es(&self, rule: &Rule) -> Result<Vec<LogEntry>, AppError> {
        self.fetch_logs_from_es_with_client(rule, self.es.as_ref(), None).await
    }

    async fn fetch_logs_from_es_with_client(
        &self,
        rule: &Rule,
        es_client: &dyn ElasticSearchClientTrait,
        data_source: Option<&DataSource>,
    ) -> Result<Vec<LogEntry>, AppError> {
        // index name: use the data source's target if set, otherwise "logs".
        let index = data_source
            .map(|ds| ds.target.trim().to_string())
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| "logs".to_string());

        // field mapping: standard field path -> external ES field path.
        let field_mapping = data_source
            .map(|ds| parse_field_mapping(&ds.field_mapping))
            .unwrap_or_default();

        let timestamp_path = field_mapping
            .get("@timestamp")
            .cloned()
            .unwrap_or_else(|| "@timestamp".to_string());

        let query = serde_json::json!({
            "query": {
                "bool": {
                    "filter": [
                        {
                            "range": {
                                timestamp_path: {
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
            es_client.search(&index, query)
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
                    timestamp: md(source, &field_mapping, "@timestamp").to_string(),
                    source: md(source, &field_mapping, "source").to_string(),
                    client_ip: md(source, &field_mapping, "network.client.ip").to_string(),
                    method: md(source, &field_mapping, "http.request.method").to_string(),
                    path: md(source, &field_mapping, "http.request.path").to_string(),
                    query: opt_str(md(source, &field_mapping, "http.request.query")),
                    status_code: md(source, &field_mapping, "http.response.status_code").as_u64().unwrap_or(0) as u16,
                    response_size: md(source, &field_mapping, "http.response.size").as_u64().unwrap_or(0),
                    user_agent: opt_str(md(source, &field_mapping, "http.user_agent.original")),
                    user_id: opt_str(md(source, &field_mapping, "app.user.id")),
                    response_time: md(source, &field_mapping, "http.response.time").as_f64(),
                    extra: None,
                };
                logs.push(log_entry);
            }
        }

        Ok(logs)
    }

    /// Fetch logs from a ClickHouse data source via HTTP interface.
    /// Config fields: url, database, user, password, table.
    /// Uses field_mapping to translate canonical fields to ClickHouse column names.
    async fn fetch_logs_from_clickhouse(
        &self,
        rule: &Rule,
        data_source: &DataSource,
    ) -> Result<Vec<LogEntry>, AppError> {
        let parsed: Value = serde_json::from_str(&data_source.config)
            .map_err(|e| AppError::Internal(format!("Invalid ClickHouse config JSON: {}", e)))?;

        let url = parsed["url"].as_str()
            .unwrap_or("http://localhost:8123")
            .trim_end_matches('/');
        let database = parsed["database"].as_str().unwrap_or("default");
        let user = parsed["user"].as_str().unwrap_or("default");
        let password = parsed["password"].as_str().unwrap_or("");
        let table = if data_source.target.is_empty() {
            "logs".to_string()
        } else {
            data_source.target.trim().to_string()
        };

        let field_mapping = parse_field_mapping(&data_source.field_mapping);

        // Determine the timestamp column name (mapped or default).
        let timestamp_col = field_mapping
            .get("@timestamp")
            .cloned()
            .unwrap_or_else(|| "timestamp".to_string());

        // Build a ClickHouse SQL query with time-range filter.
        // ClickHouse uses relative time via now() - INTERVAL.
        let query_sql = format!(
            "SELECT * FROM {}.{} WHERE {} >= now() - INTERVAL {} SECOND LIMIT 10000 FORMAT JSONEachRow",
            database, table, timestamp_col, rule.window_sec
        );

        let client = clickhouse_http_client()?;
        let request_url = format!(
            "{}/?user={}&password={}",
            url, user, password
        );

        let resp = client
            .post(&request_url)
            .body(query_sql)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("ClickHouse query failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "ClickHouse returned status {}: {}", status, body
            )));
        }

        let body = resp.text().await
            .map_err(|e| AppError::Internal(format!("Failed to read ClickHouse response: {}", e)))?;

        let mut logs = Vec::new();
        for line in body.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let row: Value = serde_json::from_str(line)
                .map_err(|e| AppError::Internal(format!("Failed to parse ClickHouse row: {}", e)))?;

            let log_entry = LogEntry {
                timestamp: md(&row, &field_mapping, "@timestamp").to_string(),
                source: md(&row, &field_mapping, "source").to_string(),
                client_ip: md(&row, &field_mapping, "network.client.ip").to_string(),
                method: md(&row, &field_mapping, "http.request.method").to_string(),
                path: md(&row, &field_mapping, "http.request.path").to_string(),
                query: opt_str(md(&row, &field_mapping, "http.request.query")),
                status_code: md(&row, &field_mapping, "http.response.status_code").as_u64().unwrap_or(0) as u16,
                response_size: md(&row, &field_mapping, "http.response.size").as_u64().unwrap_or(0),
                user_agent: opt_str(md(&row, &field_mapping, "http.user_agent.original")),
                user_id: opt_str(md(&row, &field_mapping, "app.user.id")),
                response_time: md(&row, &field_mapping, "http.response.time").as_f64(),
                extra: None,
            };
            logs.push(log_entry);
        }

        info!("Fetched {} logs from ClickHouse table {}", logs.len(), table);
        Ok(logs)
    }

    pub async fn run_all_rules(&self) -> Result<Vec<DetectionResult>, AppError> {
        let rules = self.load_rules().await?;
        let data_sources = self.load_data_sources().await?;

        let es_sources: Vec<&DataSource> = data_sources.iter()
            .filter(|s| s.r#type == "elasticsearch")
            .collect();

        let ch_sources: Vec<&DataSource> = data_sources.iter()
            .filter(|s| s.r#type == "clickhouse")
            .collect();

        let mut results = Vec::new();

        for rule in &rules {
            // Priority: primary ClickHouse > primary ES > first ClickHouse > first ES > global ES
            let fetch_result = if let Some(primary_ch) = ch_sources.iter().find(|s| s.is_primary) {
                match self.fetch_logs_from_clickhouse(rule, primary_ch).await {
                    Ok(logs) => Ok(logs),
                    Err(e) => {
                        warn!("ClickHouse fetch failed for '{}': {}, falling back to ES", primary_ch.name, e);
                        self.fetch_logs_from_es(rule).await
                    }
                }
            } else if let Some(primary) = es_sources.iter().find(|s| s.is_primary) {
                match Self::create_es_client_from_config(&primary.config) {
                    Ok(client) => self.fetch_logs_from_es_with_client(rule, client.as_ref(), Some(primary)).await,
                    Err(e) => {
                        warn!("Failed to create ES client from data source '{}': {}", primary.name, e);
                        self.fetch_logs_from_es(rule).await
                    }
                }
            } else if let Some(first_ch) = ch_sources.first() {
                match self.fetch_logs_from_clickhouse(rule, first_ch).await {
                    Ok(logs) => Ok(logs),
                    Err(e) => {
                        warn!("ClickHouse fetch failed for '{}': {}, falling back to ES", first_ch.name, e);
                        self.fetch_logs_from_es(rule).await
                    }
                }
            } else if let Some(first) = es_sources.first() {
                match Self::create_es_client_from_config(&first.config) {
                    Ok(client) => self.fetch_logs_from_es_with_client(rule, client.as_ref(), Some(first)).await,
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_field_mapping_object_form() {
        let raw = r#"
        {
            "http.request.method": { "es_field": "req_method", "es_type": "keyword" },
            "network.client.ip": { "es_field": "src.ip" }
        }
        "#;
        let mapping = parse_field_mapping(raw);
        assert_eq!(mapping.get("http.request.method").map(|s| s.as_str()), Some("req_method"));
        assert_eq!(mapping.get("network.client.ip").map(|s| s.as_str()), Some("src.ip"));
        assert!(mapping.get("http.request.path").is_none());
    }

    #[test]
    fn parses_field_mapping_plain_form() {
        let raw = r#"{ "method": "request.method" }"#;
        let mapping = parse_field_mapping(raw);
        assert_eq!(mapping.get("method").map(|s| s.as_str()), Some("request.method"));
    }

    #[test]
    fn falls_back_to_default_when_no_mapping() {
        let mapping = parse_field_mapping("not json");
        let source = json!({
            "@timestamp": "2026-09-02T00:00:00Z",
            "http": { "request": { "method": "GET" } }
        });
        assert_eq!(md(&source, &mapping, "@timestamp").as_str(), Some("2026-09-02T00:00:00Z"));
        assert_eq!(md(&source, &mapping, "http.request.method").as_str(), Some("GET"));
    }

    #[test]
    fn applies_mapping_for_read() {
        let mapping = parse_field_mapping(r#"{ "http.request.method": { "es_field": "verb" } }"#);
        let source = json!({ "verb": "POST" });
        assert_eq!(md(&source, &mapping, "http.request.method").as_str(), Some("POST"));
    }

    #[test]
    fn reads_missing_field_as_null() {
        let mapping = HashMap::new();
        let source = json!({});
        assert!(md(&source, &mapping, "network.client.ip").is_null());
    }
}
