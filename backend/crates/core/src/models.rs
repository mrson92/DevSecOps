use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub severity: String,
    pub enabled: bool,
    pub rule_type: String,
    pub condition: String,
    pub window_sec: i32,
    pub slide_sec: i32,
    pub group_by: String,
    pub actions: String,
    pub mitre_tactics: Option<String>,
    pub mitre_techniques: Option<String>,
    pub references: Option<String>,
    pub version: i32,
    pub parent_rule_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Detection {
    pub id: String,
    pub rule_id: String,
    pub rule_version: i32,
    pub detected_at: String,
    pub window_start: String,
    pub window_end: String,
    pub matched_count: i32,
    pub group_key: Option<String>,
    pub context: Option<String>,
    pub status: String,
    pub assignee: Option<String>,
    pub acknowledged_at: Option<String>,
    pub resolved_at: Option<String>,
    pub resolution_note: Option<String>,
    pub notifications: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Report {
    pub id: String,
    #[sqlx(rename = "type")]
    pub report_type: String,
    pub title: String,
    pub period_start: String,
    pub period_end: String,
    pub content: String,
    pub summary: Option<String>,
    pub format: String,
    pub status: String,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStats {
    pub total_detections: i64,
    pub open_detections: i64,
    pub critical_count: i64,
    pub high_count: i64,
    pub medium_count: i64,
    pub low_count: i64,
    pub active_rules: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRuleRequest {
    pub name: String,
    pub description: Option<String>,
    pub severity: String,
    pub rule_type: String,
    pub condition: String,
    pub window_sec: Option<i32>,
    pub slide_sec: Option<i32>,
    pub group_by: Option<String>,
    pub actions: Option<String>,
    pub mitre_tactics: Option<String>,
    pub mitre_techniques: Option<String>,
    pub references: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRuleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub severity: Option<String>,
    pub enabled: Option<bool>,
    pub condition: Option<String>,
    pub window_sec: Option<i32>,
    pub slide_sec: Option<i32>,
    pub group_by: Option<String>,
    pub actions: Option<String>,
    pub mitre_tactics: Option<String>,
    pub mitre_techniques: Option<String>,
    pub references: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDetectionRequest {
    pub status: Option<String>,
    pub assignee: Option<String>,
    pub resolution_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRuleRequest {
    pub test_type: String,
    pub time_range_start: Option<String>,
    pub time_range_end: Option<String>,
    pub sample_logs: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRuleResult {
    pub id: String,
    pub rule_id: String,
    pub test_type: String,
    pub matched_count: u32,
    pub matched_logs: Vec<serde_json::Value>,
    pub execution_time_ms: u64,
    pub status: String,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub size: Option<u32>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub success: bool,
    pub data: Vec<T>,
    pub meta: PaginationMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationMeta {
    pub page: u32,
    pub size: u32,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DataSource {
    pub id: String,
    pub name: String,
    pub r#type: String,
    pub config: String,
    pub target: String,
    pub field_mapping: String,
    pub enabled: bool,
    pub is_primary: bool,
    pub created_at: String,
    pub updated_at: String,
}
