use std::collections::HashMap;
use std::sync::Arc;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use tracing::{info, warn, error};
use uuid::Uuid;
use serde_json::{json, Value};
use serde::Serialize;

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

/// Metadata describing which data source served a log-browsing query.
#[derive(Debug, Clone)]
pub struct LogSourceMeta {
    pub id: String,
    pub name: String,
    pub r#type: String,
}

impl LogSourceMeta {
    fn from_ds(ds: &DataSource) -> Self {
        Self {
            id: ds.id.clone(),
            name: ds.name.clone(),
            r#type: ds.r#type.clone(),
        }
    }
}

/// A field (column / mapped path) exposed by a data source's schema.
#[derive(Debug, Clone, Serialize)]
pub struct DatasourceField {
    pub name: String,
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub standard: Option<String>,
}

/// Filters for log browsing / search.
#[derive(Debug, Clone, Default)]
pub struct LogFilter {
    pub from: Option<String>,
    pub to: Option<String>,
    pub q: Option<String>,
    pub source: Option<String>,
    pub client_ip: Option<String>,
    pub method: Option<String>,
    pub path: Option<String>,
    pub status_code: Option<u16>,
    pub user_id: Option<String>,
    pub user_agent: Option<String>,
    /// Excluded (NOT) filters, keyed by canonical field name.
    /// Supported keys: path, client_ip, method, source, status_code.
    pub excludes: Vec<(String, String)>,
    pub limit: usize,
    pub offset: usize,
    pub order: String,
}

/// Map a canonical field to the datasource-specific column/path.
fn mapped_col<'a>(field_mapping: &'a FieldMapping, standard: &str, default: &'a str) -> &'a str {
    field_mapping.get(standard).map(|s| s.as_str()).unwrap_or(default)
}

/// Escape a string literal for use inside a ClickHouse single-quoted string.
fn esc(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "''")
}

/// Escape a string for a ClickHouse ILIKE pattern (wildcards and quotes).
fn esc_like(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "''").replace('%', "\\%").replace('_', "\\_")
}

/// Parse an RFC3339 timestamp into epoch seconds (UTC) if possible.
fn epoch_sec_opt(value: Option<&str>) -> Option<i64> {
    let value = value.filter(|v| !v.is_empty())?;
    DateTime::parse_from_rfc3339(value).map(|dt| dt.timestamp()).ok()
}

/// Default ClickHouse column name for a canonical field (flat schema).
fn flat_default(standard: &str) -> &str {
    match standard {
        "http.request.path" => "path",
        "http.request.query" => "query",
        "http.request.method" => "method",
        "source" => "source",
        "http.user_agent.original" => "user_agent",
        "app.user.id" => "user_id",
        "network.client.ip" => "client_ip",
        _ => standard,
    }
}

/// Build the WHERE conditions shared by ClickHouse browsing queries.
fn clickhouse_conditions(field_mapping: &FieldMapping, filter: &LogFilter) -> Vec<String> {
    let col = |standard: &str, default: &str| mapped_col(field_mapping, standard, default).to_string();

    let mut conds: Vec<String> = Vec::new();
    if let Some(secs) = epoch_sec_opt(filter.from.as_deref()) {
        conds.push(format!("{} >= fromUnixTimestamp({})", col("@timestamp", "timestamp"), secs));
    }
    if let Some(secs) = epoch_sec_opt(filter.to.as_deref()) {
        conds.push(format!("{} <= fromUnixTimestamp({})", col("@timestamp", "timestamp"), secs));
    }
    if let Some(v) = filter.source.as_deref().filter(|s| !s.is_empty()) {
        conds.push(format!("{} = '{}'", col("source", "source"), esc(v)));
    }
    if let Some(v) = filter.client_ip.as_deref().filter(|s| !s.is_empty()) {
        conds.push(format!("{} ILIKE '%{}%'", col("network.client.ip", "client_ip"), esc_like(v)));
    }
    if let Some(v) = filter.method.as_deref().filter(|s| !s.is_empty()) {
        conds.push(format!("{} = '{}'", col("http.request.method", "method"), esc(v)));
    }
    if let Some(v) = filter.path.as_deref().filter(|s| !s.is_empty()) {
        conds.push(format!("{} ILIKE '%{}%'", col("http.request.path", "path"), esc_like(v)));
    }
    if let Some(code) = filter.status_code {
        conds.push(format!("{} = {}", col("http.response.status_code", "status_code"), code));
    }
    if let Some(v) = filter.user_id.as_deref().filter(|s| !s.is_empty()) {
        conds.push(format!("{} ILIKE '%{}%'", col("app.user.id", "user_id"), esc_like(v)));
    }
    if let Some(v) = filter.user_agent.as_deref().filter(|s| !s.is_empty()) {
        conds.push(format!("{} ILIKE '%{}%'", col("http.user_agent.original", "user_agent"), esc_like(v)));
    }
    if let Some(q) = filter.q.as_deref().filter(|s| !s.is_empty()) {
        let pattern = esc_like(q);
        let patterns: Vec<String> = [
            "http.request.path", "http.request.query", "http.request.method", "source",
            "http.user_agent.original", "app.user.id", "network.client.ip",
        ]
        .into_iter()
        .map(|standard| {
            let c = mapped_col(field_mapping, standard, flat_default(standard)).to_string();
            format!("{} ILIKE '%{}%'", c, pattern)
        })
        .collect();
        conds.push(format!("({})", patterns.join(" OR ")));
    }

    // Excludes (NOT filters) — keyed by canonical field name.
    for (key, value) in &filter.excludes {
        let (standard, matcher) = match key.as_str() {
            "path" => ("http.request.path", format!("{} NOT ILIKE '%{}%'", col("http.request.path", "path"), esc_like(value))),
            "client_ip" => ("network.client.ip", format!("{} NOT ILIKE '%{}%'", col("network.client.ip", "client_ip"), esc_like(value))),
            "method" => ("http.request.method", format!("{} != '{}'", col("http.request.method", "method"), esc(value))),
            "source" => ("source", format!("{} != '{}'", col("source", "source"), esc(value))),
            "status_code" => ("http.response.status_code", format!("{} != {}", col("http.response.status_code", "status_code"), value)),
            _ => continue,
        };
        let _ = standard;
        conds.push(matcher);
    }

    conds
}

/// Convert a raw document (ClickHouse row or ES `_source`) into a `LogEntry`.
fn row_to_log_entry(row: &Value, field_mapping: &FieldMapping) -> LogEntry {
    LogEntry {
        timestamp: opt_str(md(row, field_mapping, "@timestamp")).unwrap_or_default(),
        source: opt_str(md(row, field_mapping, "source")).unwrap_or_default(),
        client_ip: opt_str(md(row, field_mapping, "network.client.ip")).unwrap_or_default(),
        method: opt_str(md(row, field_mapping, "http.request.method")).unwrap_or_default(),
        path: opt_str(md(row, field_mapping, "http.request.path")).unwrap_or_default(),
        query: opt_str(md(row, field_mapping, "http.request.query")),
        status_code: md(row, field_mapping, "http.response.status_code").as_u64().unwrap_or(0) as u16,
        response_size: md(row, field_mapping, "http.response.size").as_u64().unwrap_or(0),
        user_agent: opt_str(md(row, field_mapping, "http.user_agent.original")),
        user_id: opt_str(md(row, field_mapping, "app.user.id")),
        response_time: md(row, field_mapping, "http.response.time").as_f64(),
        extra: None,
    }
}

/// Build a JSON object from (key, value) pairs (for dynamic ES query keys).
fn es_obj(entries: Vec<(&str, Value)>) -> Value {
    let mut map = serde_json::Map::new();
    for (key, value) in entries {
        map.insert(key.to_string(), value);
    }
    Value::Object(map)
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
                logs.push(row_to_log_entry(source, &field_mapping));
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

            logs.push(row_to_log_entry(&row, &field_mapping));
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

    // ------------------------------------------------------------------
    // Log browsing: search / histogram over the resolved data source.
    // ------------------------------------------------------------------

    /// Resolve the data source that should serve a log-browsing query, mirroring
    /// the priority used by `run_all_rules`: explicit id > primary ClickHouse >
    /// primary ES > first ClickHouse > first ES > global (default) Elasticsearch.
    fn resolve_datasource<'a>(
        data_sources: &'a [DataSource],
        datasource_id: Option<&str>,
    ) -> Result<Option<&'a DataSource>, AppError> {
        if let Some(id) = datasource_id {
            return data_sources
                .iter()
                .find(|s| s.id == id)
                .map(Some)
                .ok_or_else(|| AppError::NotFound(format!("Data source not found: {}", id)));
        }

        let ch: Vec<&DataSource> = data_sources.iter().filter(|s| s.r#type == "clickhouse").collect();
        let es: Vec<&DataSource> = data_sources.iter().filter(|s| s.r#type == "elasticsearch").collect();

        if let Some(s) = ch.iter().find(|s| s.is_primary) {
            Ok(Some(s))
        } else if let Some(s) = es.iter().find(|s| s.is_primary) {
            Ok(Some(s))
        } else if let Some(s) = ch.first() {
            Ok(Some(s))
        } else if let Some(s) = es.first() {
            Ok(Some(s))
        } else {
            Ok(None)
        }
    }

    /// Search logs across the resolved data source (or global ES when none).
    /// Returns (logs, total count, source metadata).
    pub async fn search_logs(
        &self,
        filter: &LogFilter,
        datasource_id: Option<&str>,
    ) -> Result<(Vec<LogEntry>, i64, LogSourceMeta), AppError> {
        let data_sources = self.load_data_sources().await?;
        let datasource = Self::resolve_datasource(&data_sources, datasource_id)?;

        match datasource {
            Some(ds) if ds.r#type == "clickhouse" => {
                let (logs, total) = self.search_logs_from_clickhouse(ds, filter).await?;
                Ok((logs, total, LogSourceMeta::from_ds(ds)))
            }
            Some(ds) if ds.r#type == "elasticsearch" => {
                let client = Self::create_es_client_from_config(&ds.config)
                    .map_err(|e| AppError::Internal(format!("ES client init failed: {}", e)))?;
                let (logs, total) = self
                    .search_logs_from_es_with_client(client.as_ref(), Some(ds), filter)
                    .await?;
                Ok((logs, total, LogSourceMeta::from_ds(ds)))
            }
            Some(ds) => Err(AppError::Validation(format!(
                "Unsupported data source type: {}",
                ds.r#type
            ))),
            None => {
                let (logs, total) = self
                    .search_logs_from_es_with_client(self.es.as_ref(), None, filter)
                    .await?;
                Ok((
                    logs,
                    total,
                    LogSourceMeta {
                        id: String::new(),
                        name: "Elasticsearch (default)".to_string(),
                        r#type: "elasticsearch".to_string(),
                    },
                ))
            }
        }
    }

    /// Bucket log counts by a fixed interval (seconds) across the resolved source.
    pub async fn histogram_logs(
        &self,
        filter: &LogFilter,
        interval_secs: u64,
        datasource_id: Option<&str>,
    ) -> Result<(Vec<(i64, u64)>, LogSourceMeta), AppError> {
        let data_sources = self.load_data_sources().await?;
        let datasource = Self::resolve_datasource(&data_sources, datasource_id)?;

        match datasource {
            Some(ds) if ds.r#type == "clickhouse" => {
                let buckets = self.histogram_logs_from_clickhouse(ds, filter, interval_secs).await?;
                Ok((buckets, LogSourceMeta::from_ds(ds)))
            }
            Some(ds) if ds.r#type == "elasticsearch" => {
                let client = Self::create_es_client_from_config(&ds.config)
                    .map_err(|e| AppError::Internal(format!("ES client init failed: {}", e)))?;
                let buckets = self
                    .histogram_logs_from_es_with_client(client.as_ref(), Some(ds), filter, interval_secs)
                    .await?;
                Ok((buckets, LogSourceMeta::from_ds(ds)))
            }
            Some(ds) => Err(AppError::Validation(format!(
                "Unsupported data source type: {}",
                ds.r#type
            ))),
            None => {
                let buckets = self
                    .histogram_logs_from_es_with_client(self.es.as_ref(), None, filter, interval_secs)
                    .await?;
                Ok((
                    buckets,
                    LogSourceMeta {
                        id: String::new(),
                        name: "Elasticsearch (default)".to_string(),
                        r#type: "elasticsearch".to_string(),
                    },
                ))
            }
        }
    }

    /// Return the field/schema list of the resolved data source.
    pub async fn list_fields(
        &self,
        datasource_id: Option<&str>,
    ) -> Result<(Vec<DatasourceField>, LogSourceMeta), AppError> {
        let data_sources = self.load_data_sources().await?;
        let datasource = Self::resolve_datasource(&data_sources, datasource_id)?;

        match datasource {
            Some(ds) if ds.r#type == "clickhouse" => {
                let fields = self.list_fields_from_clickhouse(ds).await?;
                Ok((fields, LogSourceMeta::from_ds(ds)))
            }
            Some(ds) if ds.r#type == "elasticsearch" => {
                let client = Self::create_es_client_from_config(&ds.config)
                    .map_err(|e| AppError::Internal(format!("ES client init failed: {}", e)))?;
                let fields = Self::list_fields_from_es(client.as_ref(), &ds.target).await?;
                Ok((fields, LogSourceMeta::from_ds(ds)))
            }
            Some(ds) => Err(AppError::Validation(format!(
                "Unsupported data source type: {}",
                ds.r#type
            ))),
            None => {
                let fields = Self::list_fields_from_es(self.es.as_ref(), "logs").await?;
                Ok((
                    fields,
                    LogSourceMeta {
                        id: String::new(),
                        name: "Elasticsearch (default)".to_string(),
                        r#type: "elasticsearch".to_string(),
                    },
                ))
            }
        }
    }

    async fn list_fields_from_clickhouse(&self, ds: &DataSource) -> Result<Vec<DatasourceField>, AppError> {
        let parsed: Value = serde_json::from_str(&ds.config)
            .map_err(|e| AppError::Internal(format!("Invalid ClickHouse config JSON: {}", e)))?;

        let url = parsed["url"].as_str()
            .unwrap_or("http://localhost:8123")
            .trim_end_matches('/');
        let database = parsed["database"].as_str().unwrap_or("default");
        let user = parsed["user"].as_str().unwrap_or("default");
        let password = parsed["password"].as_str().unwrap_or("");
        let table = if ds.target.trim().is_empty() {
            "logs".to_string()
        } else {
            ds.target.trim().to_string()
        };

        let field_mapping = parse_field_mapping(&ds.field_mapping);
        let reverse: HashMap<&str, &str> = field_mapping
            .iter()
            .map(|(k, v)| (v.as_str(), k.as_str()))
            .collect();

        let query_sql = format!(
            "SELECT name, type FROM system.columns WHERE database = '{}' AND table = '{}' ORDER BY position FORMAT JSONEachRow",
            database.replace('\'', "\\'"), table.replace('\'', "\\'")
        );

        let client = clickhouse_http_client()?;
        let request_url = format!("{}/?user={}&password={}", url, user, password);
        let resp = client
            .post(&request_url)
            .body(query_sql)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("ClickHouse fields query failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "ClickHouse returned status {}: {}", status, body
            )));
        }
        let body = resp.text().await
            .map_err(|e| AppError::Internal(format!("Failed to read ClickHouse response: {}", e)))?;

        let mut fields = Vec::new();
        for line in body.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let Ok(row) = serde_json::from_str::<Value>(line) else { continue; };
            let Some(name) = row["name"].as_str().map(|s| s.to_string()) else { continue; };
            let r#type = row["type"].as_str().unwrap_or("").to_string();
            let standard = reverse.get(name.as_str()).map(|s| s.to_string());
            fields.push(DatasourceField { name, r#type, standard });
        }
        Ok(fields)
    }

    async fn list_fields_from_es(
        es_client: &dyn ElasticSearchClientTrait,
        index_suffix: &str,
    ) -> Result<Vec<DatasourceField>, AppError> {
        let index = if index_suffix.trim().is_empty() {
            "logs"
        } else {
            index_suffix.trim()
        };
        let mappings = es_client.get_index_mapping(index).await?;

        let mut fields = Vec::new();
        if let Some(indices) = mappings.as_object() {
            for body in indices.values() {
                let Some(props) = body.get("mappings").and_then(|m| m.get("properties")) else {
                    continue;
                };
                Self::collect_es_props(props, "", &mut fields);
            }
        }
        fields.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(fields)
    }

    fn collect_es_props(props: &Value, prefix: &str, out: &mut Vec<DatasourceField>) {
        let Some(obj) = props.as_object() else {
            return;
        };
        let mut sorted: Vec<_> = obj.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(b.0));
        for (name, def) in sorted {
            let full = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{}.{}", prefix, name)
            };
            if let Some(t) = def.get("type").and_then(|t| t.as_str()) {
                out.push(DatasourceField {
                    name: full.clone(),
                    r#type: t.to_string(),
                    standard: None,
                });
                if let Some(subfields) = def.get("fields").and_then(|f| f.as_object()) {
                    for (sub, subdef) in subfields {
                        if let Some(st) = subdef.get("type").and_then(|t| t.as_str()) {
                            out.push(DatasourceField {
                                name: format!("{}.{}", full, sub),
                                r#type: st.to_string(),
                                standard: None,
                            });
                        }
                    }
                }
            } else if def.get("properties").is_some() {
                Self::collect_es_props(def.get("properties").unwrap(), &full, out);
            }
        }
    }

    /// Escape a ClickHouse identifier (column/table) for use in SQL.
fn ch_ident(name: &str) -> String {
    format!("`{}`", name.replace('`', "``"))
}

/// Top field values (group-by) of the resolved data source within the filter window.
pub async fn field_values(
    &self,
    filter: &LogFilter,
    field: &str,
    q: Option<&str>,
    size: usize,
    datasource_id: Option<&str>,
) -> Result<(Vec<(String, i64)>, LogSourceMeta), AppError> {
    let data_sources = self.load_data_sources().await?;
    let datasource = Self::resolve_datasource(&data_sources, datasource_id)?;

    match datasource {
        Some(ds) if ds.r#type == "clickhouse" => {
            let values = self.field_values_from_clickhouse(ds, filter, field, q, size).await?;
            Ok((values, LogSourceMeta::from_ds(ds)))
        }
        Some(ds) if ds.r#type == "elasticsearch" => {
            let client = Self::create_es_client_from_config(&ds.config)
                .map_err(|e| AppError::Internal(format!("ES client init failed: {}", e)))?;
            let values = Self::field_values_from_es(client.as_ref(), Some(ds), filter, field, q, size).await?;
            Ok((values, LogSourceMeta::from_ds(ds)))
        }
        Some(ds) => Err(AppError::Validation(format!(
            "Unsupported data source type: {}",
            ds.r#type
        ))),
        None => {
            let values = Self::field_values_from_es(self.es.as_ref(), None, filter, field, q, size).await?;
            Ok((
                values,
                LogSourceMeta {
                    id: String::new(),
                    name: "Elasticsearch (default)".to_string(),
                    r#type: "elasticsearch".to_string(),
                },
            ))
        }
    }
}

async fn field_values_from_clickhouse(
    &self,
    ds: &DataSource,
    filter: &LogFilter,
    field: &str,
    q: Option<&str>,
    size: usize,
) -> Result<Vec<(String, i64)>, AppError> {
    let parsed: Value = serde_json::from_str(&ds.config)
        .map_err(|e| AppError::Internal(format!("Invalid ClickHouse config JSON: {}", e)))?;

    let url = parsed["url"].as_str()
        .unwrap_or("http://localhost:8123")
        .trim_end_matches('/');
    let database = parsed["database"].as_str().unwrap_or("default");
    let user = parsed["user"].as_str().unwrap_or("default");
    let password = parsed["password"].as_str().unwrap_or("");
    let table = if ds.target.trim().is_empty() {
        "logs".to_string()
    } else {
        ds.target.trim().to_string()
    };

    let field_mapping = parse_field_mapping(&ds.field_mapping);
    let col = Self::ch_ident(field);
    let mut conditions = clickhouse_conditions(&field_mapping, filter);
    if let Some(term) = q.as_deref().filter(|s| !s.is_empty()) {
        conditions.push(format!("{} ILIKE '%{}%'", col, esc_like(term)));
    }
    let where_sql = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };

    let query_sql = format!(
        "SELECT {} AS v, count() AS c FROM {}.{} {} GROUP BY v ORDER BY c DESC LIMIT {} FORMAT JSONEachRow",
        col, database, table, where_sql, size
    );

    let client = clickhouse_http_client()?;
    let request_url = format!("{}/?user={}&password={}", url, user, password);

    let resp = client.post(&request_url).body(query_sql).send().await
        .map_err(|e| AppError::Internal(format!("ClickHouse values query failed: {}", e)))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!(
            "ClickHouse returned status {}: {}", status, body
        )));
    }
    let body = resp.text().await
        .map_err(|e| AppError::Internal(format!("Failed to read ClickHouse response: {}", e)))?;

    let mut values = Vec::new();
    for line in body.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(row) = serde_json::from_str::<Value>(line) else { continue; };
        let v = if row["v"].is_null() {
            "(empty)".to_string()
        } else if let Some(s) = row["v"].as_str() {
            s.to_string()
        } else {
            row["v"].to_string()
        };
        if let Some(c) = row["c"].as_i64() {
            values.push((v, c));
        }
    }
    Ok(values)
}

async fn field_values_from_es(
    es_client: &dyn ElasticSearchClientTrait,
    ds: Option<&DataSource>,
    filter: &LogFilter,
    field: &str,
    q: Option<&str>,
    size: usize,
) -> Result<Vec<(String, i64)>, AppError> {
    let index = ds
        .map(|d| d.target.trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| "logs".to_string());

    let field_mapping = ds
        .map(|d| parse_field_mapping(&d.field_mapping))
        .unwrap_or_default();
    let ts_path = field_mapping
        .get("@timestamp")
        .cloned()
        .unwrap_or_else(|| "@timestamp".to_string());

    let from_secs = epoch_sec_opt(filter.from.as_deref());
    let to_secs = epoch_sec_opt(filter.to.as_deref());

    let search = |field_name: &str| {
        let mut must: Vec<Value> = Vec::new();
        if let Some(term) = q.as_deref().filter(|s| !s.is_empty()) {
            must.push(json!({ "match_phrase": { field_name: { "query": term } } }));
        }
        let query = if must.is_empty() {
            json!({ "bool": { "filter": [] } })
        } else {
            json!({ "bool": { "filter": [], "must": must } })
        };
        let mut query_obj = query;
        if let Some(f) = &mut query_obj["bool"]["filter"].as_array_mut() {
            let mut range = serde_json::Map::new();
            if let Some(secs) = from_secs {
                range.insert("gte".to_string(), json!(secs * 1000));
            }
            if let Some(secs) = to_secs {
                range.insert("lte".to_string(), json!(secs * 1000));
            }
            if !range.is_empty() {
                f.push(json!({ "range": { ts_path.clone(): range } }));
            }
        }
        json!({
            "size": 0,
            "query": query_obj,
            "aggs": {
                "vals": {
                    "terms": { "field": field_name, "size": size }
                }
            }
        })
    };

    let (response, used_field) = {
        let query = search(field);
        let result = match tokio::time::timeout(std::time::Duration::from_secs(8), es_client.search(&index, query)).await {
            Ok(res) => res.map_err(|e| AppError::ElasticSearch(format!("ES field values error: {}", e)))?,
            Err(_) => return Err(AppError::ElasticSearch("ES field values timeout".to_string())),
        };
        if result.get("error").is_some() && !field.ends_with(".keyword") {
            let retry = format!("{}.keyword", field);
            let query = search(&retry);
            let result2 = match tokio::time::timeout(std::time::Duration::from_secs(8), es_client.search(&index, query)).await {
                Ok(res) => res.map_err(|e| AppError::ElasticSearch(format!("ES field values error: {}", e)))?,
                Err(_) => return Err(AppError::ElasticSearch("ES field values timeout".to_string())),
            };
            (result2, retry)
        } else {
            (result, field.to_string())
        }
    };

    let mut values = Vec::new();
    let buckets = response["aggregations"]["vals"]["buckets"].as_array();
    if let Some(buckets) = buckets {
        for bucket in buckets {
            let v = if bucket["key"].is_null() {
                "(empty)".to_string()
            } else if let Some(s) = bucket["key"].as_str() {
                s.to_string()
            } else {
                bucket["key"].to_string()
            };
            if let Some(c) = bucket["doc_count"].as_i64() {
                values.push((v, c));
            }
        }
    }
    if values.is_empty() && used_field != field {
        warn!("ES field {} fell back to {} but returned no values", field, used_field);
    }
    Ok(values)
}

async fn search_logs_from_clickhouse(
        &self,
        ds: &DataSource,
        filter: &LogFilter,
    ) -> Result<(Vec<LogEntry>, i64), AppError> {
        let parsed: Value = serde_json::from_str(&ds.config)
            .map_err(|e| AppError::Internal(format!("Invalid ClickHouse config JSON: {}", e)))?;

        let url = parsed["url"].as_str()
            .unwrap_or("http://localhost:8123")
            .trim_end_matches('/');
        let database = parsed["database"].as_str().unwrap_or("default");
        let user = parsed["user"].as_str().unwrap_or("default");
        let password = parsed["password"].as_str().unwrap_or("");
        let table = if ds.target.trim().is_empty() {
            "logs".to_string()
        } else {
            ds.target.trim().to_string()
        };

        let field_mapping = parse_field_mapping(&ds.field_mapping);
        let ts_col = mapped_col(&field_mapping, "@timestamp", "timestamp");
        let conditions = clickhouse_conditions(&field_mapping, filter);
        let where_sql = if conditions.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conditions.join(" AND "))
        };

        let client = clickhouse_http_client()?;
        let request_url = format!("{}/?user={}&password={}", url, user, password);

        // Total count for pagination.
        let count_sql = format!(
            "SELECT count() AS cnt FROM {}.{} {} FORMAT JSONEachRow",
            database, table, where_sql
        );
        let total = client
            .post(&request_url)
            .body(count_sql)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("ClickHouse count query failed: {}", e)))?
            .text()
            .await
            .ok()
            .and_then(|body| {
                body.lines().next()
                    .and_then(|line| serde_json::from_str::<Value>(line).ok())
            })
            .and_then(|v| v["cnt"].as_u64())
            .unwrap_or(0) as i64;

        let order = if filter.order.eq_ignore_ascii_case("asc") { "ASC" } else { "DESC" };
        let query_sql = format!(
            "SELECT * FROM {}.{} {} ORDER BY {} {} LIMIT {}, {} FORMAT JSONEachRow",
            database, table, where_sql, ts_col, order, filter.offset, filter.limit
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
            logs.push(row_to_log_entry(&row, &field_mapping));
        }

        info!("Fetched {} logs from ClickHouse table {}", logs.len(), table);
        Ok((logs, total))
    }

    async fn histogram_logs_from_clickhouse(
        &self,
        ds: &DataSource,
        filter: &LogFilter,
        interval_secs: u64,
    ) -> Result<Vec<(i64, u64)>, AppError> {
        let parsed: Value = serde_json::from_str(&ds.config)
            .map_err(|e| AppError::Internal(format!("Invalid ClickHouse config JSON: {}", e)))?;

        let url = parsed["url"].as_str()
            .unwrap_or("http://localhost:8123")
            .trim_end_matches('/');
        let database = parsed["database"].as_str().unwrap_or("default");
        let user = parsed["user"].as_str().unwrap_or("default");
        let password = parsed["password"].as_str().unwrap_or("");
        let table = if ds.target.trim().is_empty() {
            "logs".to_string()
        } else {
            ds.target.trim().to_string()
        };

        let field_mapping = parse_field_mapping(&ds.field_mapping);
        let ts_col = mapped_col(&field_mapping, "@timestamp", "timestamp");
        let conditions = clickhouse_conditions(&field_mapping, filter);
        let where_sql = if conditions.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conditions.join(" AND "))
        };

        let interval = interval_secs.max(1);
        let query_sql = format!(
            "SELECT toUnixTimestamp(toStartOfInterval({}, toIntervalSecond({}))) AS hb, count() AS cnt FROM {}.{} {} GROUP BY hb ORDER BY hb ASC FORMAT JSONEachRow",
            ts_col, interval, database, table, where_sql
        );

        let client = clickhouse_http_client()?;
        let request_url = format!("{}/?user={}&password={}", url, user, password);
        let resp = client
            .post(&request_url)
            .body(query_sql)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("ClickHouse histogram query failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "ClickHouse returned status {}: {}", status, body
            )));
        }

        let body = resp.text().await
            .map_err(|e| AppError::Internal(format!("Failed to read ClickHouse response: {}", e)))?;

        let mut buckets = Vec::new();
        for line in body.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(row) = serde_json::from_str::<Value>(line) {
                if let (Some(ts), Some(cnt)) = (row["hb"].as_i64(), row["cnt"].as_u64()) {
                    buckets.push((ts, cnt));
                }
            }
        }

        Ok(buckets)
    }

    fn build_es_bool_query(&self, field_mapping: &FieldMapping, filter: &LogFilter) -> serde_json::Map<String, Value> {
        let mut filters: Vec<Value> = Vec::new();

        let mut range = serde_json::Map::new();
        if let Some(f) = filter.from.as_deref().filter(|v| !v.is_empty()) {
            range.insert("gte".to_string(), json!(f));
        }
        if let Some(t) = filter.to.as_deref().filter(|v| !v.is_empty()) {
            range.insert("lte".to_string(), json!(t));
        }
        if !range.is_empty() {
            let ts = mapped_col(field_mapping, "@timestamp", "@timestamp").to_string();
            filters.push(es_obj(vec![("range", es_obj(vec![(ts.as_str(), Value::Object(range))]))]));
        }

        let term_field = |standard: &str| mapped_col(field_mapping, standard, standard).to_string();

        if let Some(v) = filter.source.as_deref().filter(|s| !s.is_empty()) {
            let f = term_field("source");
            filters.push(es_obj(vec![("term", es_obj(vec![(f.as_str(), json!(v))]))]));
        }
        if let Some(v) = filter.client_ip.as_deref().filter(|s| !s.is_empty()) {
            let f = term_field("network.client.ip");
            filters.push(es_obj(vec![("term", es_obj(vec![(f.as_str(), json!(v))]))]));
        }
        if let Some(v) = filter.method.as_deref().filter(|s| !s.is_empty()) {
            let f = term_field("http.request.method");
            filters.push(es_obj(vec![("term", es_obj(vec![(f.as_str(), json!(v))]))]));
        }
        if let Some(v) = filter.path.as_deref().filter(|s| !s.is_empty()) {
            let f = term_field("http.request.path");
            let wildcard = format!("*{}*", v.replace('*', "").replace('?', ""));
            filters.push(es_obj(vec![(
                "wildcard",
                es_obj(vec![(f.as_str(), es_obj(vec![("value", json!(wildcard)), ("case_insensitive", json!(true))]))]),
            )]));
        }
        if let Some(code) = filter.status_code {
            let f = term_field("http.response.status_code");
            filters.push(es_obj(vec![("term", es_obj(vec![(f.as_str(), json!(code))]))]));
        }
        if let Some(v) = filter.user_id.as_deref().filter(|s| !s.is_empty()) {
            let f = term_field("app.user.id");
            filters.push(es_obj(vec![("term", es_obj(vec![(f.as_str(), json!(v))]))]));
        }
        if let Some(v) = filter.user_agent.as_deref().filter(|s| !s.is_empty()) {
            let f = term_field("http.user_agent.original");
            let wildcard = format!("*{}*", v.replace('*', "").replace('?', ""));
            filters.push(es_obj(vec![(
                "wildcard",
                es_obj(vec![(f.as_str(), es_obj(vec![("value", json!(wildcard)), ("case_insensitive", json!(true))]))]),
            )]));
        }
        if let Some(q) = filter.q.as_deref().filter(|s| !s.is_empty()) {
            let fields: Vec<Value> = [
                "http.request.path", "http.request.query", "http.request.method", "source",
                "http.user_agent.original", "app.user.id", "network.client.ip",
            ]
            .into_iter()
            .map(|s| Value::String(mapped_col(field_mapping, s, s).to_string()))
            .collect();
            filters.push(es_obj(vec![(
                "multi_match",
                es_obj(vec![("query", json!(q)), ("fields", Value::Array(fields))]),
            )]));
        }

        // Excludes (NOT filters) — emitted as `must_not` clauses.
        let mut must_not: Vec<Value> = Vec::new();
        for (key, value) in &filter.excludes {
            match key.as_str() {
                "path" => {
                    let f = mapped_col(field_mapping, "http.request.path", "http.request.path").to_string();
                    let wildcard = format!("*{}*", value.replace('*', "").replace('?', ""));
                    must_not.push(es_obj(vec![(
                        "wildcard",
                        es_obj(vec![(f.as_str(), es_obj(vec![("value", json!(wildcard)), ("case_insensitive", json!(true))]))]),
                    )]));
                }
                "client_ip" => {
                    let f = mapped_col(field_mapping, "network.client.ip", "network.client.ip").to_string();
                    must_not.push(es_obj(vec![("term", es_obj(vec![(f.as_str(), json!(value))]))]));
                }
                "method" => {
                    let f = mapped_col(field_mapping, "http.request.method", "http.request.method").to_string();
                    must_not.push(es_obj(vec![("term", es_obj(vec![(f.as_str(), json!(value))]))]));
                }
                "source" => {
                    let f = mapped_col(field_mapping, "source", "source").to_string();
                    must_not.push(es_obj(vec![("term", es_obj(vec![(f.as_str(), json!(value))]))]));
                }
                "status_code" => {
                    let f = mapped_col(field_mapping, "http.response.status_code", "http.response.status_code").to_string();
                    if let Ok(code) = value.parse::<u64>() {
                        must_not.push(es_obj(vec![("term", es_obj(vec![(f.as_str(), json!(code))]))]));
                    }
                }
                _ => {}
            }
        }

        let mut bool_map = serde_json::Map::new();
        bool_map.insert("filter".to_string(), Value::Array(filters));
        if !must_not.is_empty() {
            bool_map.insert("must_not".to_string(), Value::Array(must_not));
        }
        bool_map
    }

    async fn search_logs_from_es_with_client(
        &self,
        es_client: &dyn ElasticSearchClientTrait,
        ds: Option<&DataSource>,
        filter: &LogFilter,
    ) -> Result<(Vec<LogEntry>, i64), AppError> {
        let index = ds.map(|d| d.target.trim().to_string())
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| "logs".to_string());

        let field_mapping = ds.map(|d| parse_field_mapping(&d.field_mapping)).unwrap_or_default();
        let timestamp_path = mapped_col(&field_mapping, "@timestamp", "@timestamp").to_string();
        let order = if filter.order.eq_ignore_ascii_case("asc") { "asc" } else { "desc" };

        let bool_query = self.build_es_bool_query(&field_mapping, filter);
        let sort = Value::Array(vec![es_obj(vec![(
            timestamp_path.as_str(),
            es_obj(vec![("order", json!(order))]),
        )])]);
        let body = es_obj(vec![
            ("query", es_obj(vec![("bool", Value::Object(bool_query))])),
            ("from", json!(filter.offset)),
            ("size", json!(filter.limit.clamp(1, 1000))),
            ("sort", sort),
        ]);

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            es_client.search(&index, body),
        )
        .await
        .map_err(|_| AppError::ElasticSearch("ES search timeout".to_string()))?
        .map_err(|e| AppError::ElasticSearch(format!("ES search error: {}", e)))?;

        let total = result["hits"]["total"]["value"]
            .as_u64()
            .or_else(|| result["hits"]["total"].as_u64())
            .unwrap_or(0) as i64;

        let hits = result["hits"]["hits"].as_array().cloned().unwrap_or_default();
        let mut logs = Vec::new();
        for hit in &hits {
            if let Some(source) = hit.get("_source") {
                logs.push(row_to_log_entry(source, &field_mapping));
            }
        }

        Ok((logs, total))
    }

    async fn histogram_logs_from_es_with_client(
        &self,
        es_client: &dyn ElasticSearchClientTrait,
        ds: Option<&DataSource>,
        filter: &LogFilter,
        interval_secs: u64,
    ) -> Result<Vec<(i64, u64)>, AppError> {
        let index = ds.map(|d| d.target.trim().to_string())
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| "logs".to_string());

        let field_mapping = ds.map(|d| parse_field_mapping(&d.field_mapping)).unwrap_or_default();
        let timestamp_path = mapped_col(&field_mapping, "@timestamp", "@timestamp").to_string();
        let interval = interval_secs.max(1);

        let bool_query = self.build_es_bool_query(&field_mapping, filter);
        let body = es_obj(vec![
            ("query", es_obj(vec![("bool", Value::Object(bool_query))])),
            ("size", json!(0)),
            (
                "aggs",
                es_obj(vec![(
                    "hist",
                    es_obj(vec![(
                        "date_histogram",
                        es_obj(vec![
                            ("field", json!(timestamp_path)),
                            ("fixed_interval", json!(format!("{}s", interval))),
                        ]),
                    )]),
                )]),
            ),
        ]);

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            es_client.search(&index, body),
        )
        .await
        .map_err(|_| AppError::ElasticSearch("ES search timeout".to_string()))?
        .map_err(|e| AppError::ElasticSearch(format!("ES search error: {}", e)))?;

        let mut buckets = Vec::new();
        if let Some(arr) = result["aggregations"]["hist"]["buckets"].as_array() {
            for b in arr {
                let key_ms = b["key"].as_i64().unwrap_or(0);
                let cnt = b["doc_count"].as_u64().unwrap_or(0);
                buckets.push((key_ms / 1000, cnt));
            }
        }

        Ok(buckets)
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
