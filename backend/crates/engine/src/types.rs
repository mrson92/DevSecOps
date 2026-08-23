use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub source: String,
    pub client_ip: String,
    pub method: String,
    pub path: String,
    pub query: Option<String>,
    pub status_code: u16,
    pub response_size: u64,
    pub user_agent: Option<String>,
    pub user_id: Option<String>,
    pub response_time: Option<f64>,
    pub extra: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    pub rule_id: String,
    pub rule_name: String,
    pub severity: String,
    pub detected: bool,
    pub matched_count: u32,
    pub group_key: Option<String>,
    pub matched_entries: Vec<LogEntry>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleContext {
    pub logs: Vec<Value>,
    pub window_start: String,
    pub window_end: String,
    pub group_key: Option<String>,
}
