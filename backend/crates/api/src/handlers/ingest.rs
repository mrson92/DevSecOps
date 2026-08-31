use axum::extract::State;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use uuid::Uuid;

use aads_core::error::AppError;
use aads_core::state::AppState;

const MAX_BATCH_SIZE: usize = 10_000;
const LOGS_INDEX: &str = "logs";

#[derive(Debug, Deserialize)]
pub struct IngestRequest {
    #[serde(default)]
    pub log_type: Option<String>,
    pub logs: Vec<Value>,
}

#[derive(Debug, serde::Serialize)]
pub struct IngestResponse {
    pub success: bool,
    pub ingested: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

pub async fn ingest_logs(
    State(state): State<AppState>,
    Json(req): Json<IngestRequest>,
) -> Result<Json<IngestResponse>, AppError> {
    if req.logs.is_empty() {
        return Err(AppError::Validation("No logs provided".into()));
    }
    if req.logs.len() > MAX_BATCH_SIZE {
        return Err(AppError::Validation(format!(
            "Batch too large (max {}): got {}",
            MAX_BATCH_SIZE,
            req.logs.len()
        )));
    }

    ensure_logs_index(&state).await?;

    let default_log_type = req.log_type.as_deref().unwrap_or("access");

    let mut docs: Vec<(String, Value)> = Vec::with_capacity(req.logs.len());
    let mut errors: Vec<String> = Vec::new();

    for raw in &req.logs {
        match normalize_log(raw, default_log_type) {
            Ok(normalized) => {
                let id = format!("{}-{}", Uuid::new_v4(), gen_seq());
                docs.push((id, normalized));
            }
            Err(e) => {
                errors.push(e);
            }
        }
    }

    let (ingested, failed) = if docs.is_empty() {
        (0usize, errors.len())
    } else {
        let result = state.es.bulk_index(LOGS_INDEX, docs).await?;
        let (ok, fail) = count_result(&result);
        (ok, fail + errors.len())
    };

    Ok(Json(IngestResponse {
        success: errors.is_empty() && failed == 0,
        ingested,
        failed,
        errors,
    }))
}

fn gen_seq() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    format!("{:09}", nanos)
}

fn count_result(result: &Value) -> (usize, usize) {
    match result["items"].as_array() {
        Some(items) => {
            let failed = items
                .iter()
                .filter(|it| {
                    let status = it["index"]["status"].as_u64().unwrap_or(0);
                    status == 0 || status >= 400
                })
                .count();
            (items.len() - failed, failed)
        }
        None => (0, 0),
    }
}

fn normalize_log(raw: &Value, default_log_type: &str) -> Result<Value, String> {
    let mut doc = match raw {
        Value::String(s) => parse_raw_line(s, default_log_type)?,
        Value::Object(_) => raw.clone(),
        _ => return Err("Unsupported log entry type: expected object or string".into()),
    };

    let obj = doc
        .as_object_mut()
        .ok_or_else(|| "Normalized log must be an object".to_string())?;

    let log_type = obj
        .get("log")
        .and_then(|l| l.get("type"))
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| default_log_type.to_string());

    if obj.get("@timestamp").is_none() {
        obj.insert(
            "@timestamp".to_string(),
            json!(Utc::now().to_rfc3339()),
        );
    }

    let log = obj
        .entry("log")
        .or_insert_with(|| json!({}));
    if let Value::Object(log_map) = log {
        log_map
            .entry("type")
            .or_insert_with(|| json!(log_type));
        log_map.entry("ingested_at").or_insert_with(|| {
            json!(Utc::now().to_rfc3339())
        });
    }

    validate_fields(obj)?;

    Ok(doc)
}

fn validate_fields(obj: &Map<String, Value>) -> Result<(), String> {
    let ts = obj
        .get("@timestamp")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing @timestamp".to_string())?;
    if DateTime::parse_from_rfc3339(ts).is_err() {
        return Err(format!("Invalid @timestamp format: {}", ts));
    }

    if obj.get("network").is_none() && obj.get("http").is_none() && obj.get("log").is_none() {
        return Err("Log must contain at least one of network/http/log section".into());
    }

    Ok(())
}

fn parse_raw_line(line: &str, default_log_type: &str) -> Result<Value, String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err("Empty log line".into());
    }

    if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
        if parsed.is_object() {
            return Ok(parsed);
        }
    }

    if default_log_type.eq_ignore_ascii_case("process") || detect_process_line(trimmed) {
        return parse_process_line(trimmed);
    }

    parse_access_line(trimmed)
}

fn detect_process_line(line: &str) -> bool {
    line.starts_with(|c: char| c.is_ascii_digit())
        && line.chars().filter(|&c| c == '[').count() == 1
}

fn parse_process_line(line: &str) -> Result<Value, String> {
    let re = regex::Regex::new(
        r#"^(?P<ts>\S+ \S+ \S+) (?P<svc>\S+) (?P<session>\S+) (?P<user>\S+) \[(?P<thread>[^\]]+)\] (?P<level>\S+)\s+(?P<logger>\S+)\[(?P<method>\S+):(?P<line>\d+)\] - (?P<message>.*)$"#,
    )
    .map_err(|e| format!("Regex compile error: {}", e))?;

    let caps = re
        .captures(line)
        .ok_or_else(|| "Failed to parse process log line".to_string())?;

    let mut doc = Map::new();
    doc.insert(
        "@timestamp".to_string(),
        json!(normalize_timestamp(&caps["ts"])),
    );

    let mut app = Map::new();
    app.insert("service".to_string(), json!(caps["svc"].to_string()));
    let mut user = Map::new();
    let user_val = caps["user"].to_string();
    user.insert(
        "id".to_string(),
        json!(if user_val == "-" { Value::Null } else { Value::String(user_val) }),
    );
    app.insert("user".to_string(), Value::Object(user));
    let mut session = Map::new();
    session.insert(
        "session_id".to_string(),
        json!(caps["session"].to_string()),
    );
    app.insert("session".to_string(), Value::Object(session));
    doc.insert("app".to_string(), Value::Object(app));

    let mut log = Map::new();
    log.insert("level".to_string(), json!(caps["level"].to_string().to_uppercase()));
    log.insert(
        "logger".to_string(),
        json!(format!("{}[{}:{}]", &caps["logger"], &caps["method"], &caps["line"])),
    );
    log.insert("message".to_string(), json!(caps["message"].to_string()));
    log.insert("type".to_string(), json!("process"));
    doc.insert("log".to_string(), Value::Object(log));

    Ok(Value::Object(doc))
}

fn parse_access_line(line: &str) -> Result<Value, String> {
    let re = regex::Regex::new(
        r#"^(?P<ip>\S+) (?P<svc>\S+) (?P<inst>\S+) (?P<user>\S+)\s+\[(?P<ts>[^\]]+)\] (?P<method>\S+) (?P<path>\S+)(?: \S+)? (?P<status>\d{3}) (?P<bytes>\S+) "(?P<referer>[^"]*)" "(?P<ua>[^"]*)" "" (?P<latency>\S+)$"#,
    )
    .map_err(|e| format!("Regex compile error: {}", e))?;

    let caps = re
        .captures(line)
        .ok_or_else(|| "Failed to parse access log line".to_string())?;

    let mut doc = Map::new();
    doc.insert("@timestamp".to_string(), json!(normalize_timestamp(&caps["ts"])));

    let mut network = Map::new();
    let mut client = Map::new();
    client.insert("ip".to_string(), json!(caps["ip"].to_string()));
    network.insert("client".to_string(), Value::Object(client));
    doc.insert("network".to_string(), Value::Object(network));

    let mut app = Map::new();
    app.insert("service".to_string(), json!(caps["svc"].to_string()));
    app.insert("instance".to_string(), json!(caps["inst"].to_string()));
    let mut user = Map::new();
    let user_val = caps["user"].to_string();
    user.insert(
        "id".to_string(),
        json!(if user_val == "-" { Value::Null } else { Value::String(user_val) }),
    );
    app.insert("user".to_string(), Value::Object(user));
    doc.insert("app".to_string(), Value::Object(app));

    let mut http = Map::new();
    let mut request = Map::new();
    request.insert("method".to_string(), json!(caps["method"].to_string().to_uppercase()));
    request.insert("path".to_string(), json!(caps["path"].to_string()));
    http.insert("request".to_string(), Value::Object(request));

    let mut response = Map::new();
    let status = caps["status"].parse::<u16>().unwrap_or(0);
    let bytes = parse_number(&caps["bytes"]);
    let latency_s = caps["latency"].parse::<f64>().unwrap_or(0.0);
    response.insert("status_code".to_string(), json!(status));
    if let Some(b) = bytes {
        response.insert("size".to_string(), json!(b));
    }
    response.insert("latency_ms".to_string(), json!((latency_s * 1000.0) as u64));
    http.insert("response".to_string(), Value::Object(response));

    let referer = caps["referer"].to_string();
    if !referer.is_empty() {
        let mut headers = Map::new();
        headers.insert("referer".to_string(), json!(referer));
        http.insert("request_headers".to_string(), Value::Object(headers));
    }

    let ua = caps["ua"].to_string();
    if !ua.is_empty() {
        let mut user_agent = Map::new();
        user_agent.insert("original".to_string(), json!(ua));
        http.insert("user_agent".to_string(), Value::Object(user_agent));
    }

    doc.insert("http".to_string(), Value::Object(http));

    let mut log = Map::new();
    log.insert("type".to_string(), json!("access"));
    doc.insert("log".to_string(), Value::Object(log));

    Ok(Value::Object(doc))
}

fn parse_number(s: &str) -> Option<i64> {
    match s {
        "-" | "" => None,
        s => s.parse::<i64>().ok(),
    }
}

fn normalize_timestamp(ts: &str) -> String {
    let ts = ts.trim();
    if DateTime::parse_from_rfc3339(ts).is_ok() {
        return ts.to_string();
    }

    if ts.contains('/') || (ts.contains(':') && ts.contains(' ')) {
        if let Some(dt) = parse_apache_combined(ts) {
            return dt;
        }
    }

    if let Ok(dt) = DateTime::parse_from_str(ts, "%Y-%m-%d %H:%M:%S%.3f") {
        return dt.to_rfc3339();
    }

    ts.to_string()
}

fn parse_apache_combined(ts: &str) -> Option<String> {
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun",
        "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let mut normalized = ts.to_string();
    for (i, m) in months.iter().enumerate() {
        normalized = normalized.replace(m, &format!("{:02}", i + 1));
    }
    DateTime::parse_from_str(&normalized, "%d/%m/%Y:%H:%M:%S %z")
        .ok()
        .map(|dt| dt.to_rfc3339())
}

async fn ensure_logs_index(state: &AppState) -> Result<(), AppError> {
    let exists = state.es.index_exists(LOGS_INDEX).await?;
    if exists {
        return Ok(());
    }

    let mapping = json!({
        "mappings": {
            "properties": {
                "@timestamp": { "type": "date" },
                "log": {
                    "properties": {
                        "type": { "type": "keyword" },
                        "level": { "type": "keyword" },
                        "logger": { "type": "keyword" },
                        "message": { "type": "text" },
                        "ingested_at": { "type": "date" }
                    }
                },
                "network": {
                    "properties": {
                        "client": {
                            "properties": {
                                "ip": { "type": "ip" }
                            }
                        }
                    }
                },
                "app": {
                    "properties": {
                        "service": { "type": "keyword" },
                        "instance": { "type": "keyword" },
                        "user": {
                            "properties": {
                                "id": { "type": "keyword" },
                                "session_id": { "type": "keyword" }
                            }
                        },
                        "session": {
                            "properties": {
                                "session_id": { "type": "keyword" }
                            }
                        }
                    }
                },
                "http": {
                    "properties": {
                        "request": {
                            "properties": {
                                "method": { "type": "keyword" },
                                "path": { "type": "keyword" },
                                "query": { "type": "keyword" }
                            }
                        },
                        "request_headers": {
                            "properties": {
                                "referer": { "type": "keyword" }
                            }
                        },
                        "response": {
                            "properties": {
                                "status_code": { "type": "short" },
                                "size": { "type": "long" },
                                "latency_ms": { "type": "integer" }
                            }
                        },
                        "user_agent": {
                            "properties": {
                                "original": { "type": "keyword" }
                            }
                        }
                    }
                },
                "source": { "type": "keyword" }
            }
        }
    });

    state
        .es
        .create_index(LOGS_INDEX, mapping)
        .await
        .map_err(|e| AppError::ElasticSearch(format!("Failed to create logs index: {}", e)))?;

    Ok(())
}
