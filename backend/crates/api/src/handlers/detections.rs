use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use aads_core::state::AppState;
use aads_core::error::AppError;
use aads_core::models::UpdateDetectionRequest;

#[derive(Debug, Deserialize, Default)]
pub struct DetectionFilterParams {
    pub page: Option<u32>,
    pub size: Option<u32>,
    pub severity: Option<String>,
    pub status: Option<String>,
    pub rule_id: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

pub async fn list_detections(
    State(state): State<AppState>,
    Query(params): Query<DetectionFilterParams>,
) -> Result<Json<Value>, AppError> {
    let page = params.page.unwrap_or(1);
    let size = params.size.unwrap_or(20);

    let mut conditions = Vec::new();
    let mut bind_values: Vec<String> = Vec::new();

    if let Some(ref status) = params.status {
        conditions.push("re.status = ?");
        bind_values.push(status.clone());
    }
    if let Some(ref rule_id) = params.rule_id {
        conditions.push("re.rule_id = ?");
        bind_values.push(rule_id.clone());
    }
    if let Some(ref start_date) = params.start_date {
        conditions.push("re.detected_at >= ?");
        bind_values.push(start_date.clone());
    }
    if let Some(ref end_date) = params.end_date {
        conditions.push("re.detected_at <= ?");
        bind_values.push(end_date.clone());
    }
    if let Some(ref severity) = params.severity {
        conditions.push("r.severity = ?");
        bind_values.push(severity.clone());
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let join_clause = if params.severity.is_some() {
        "JOIN rules r ON re.rule_id = r.id"
    } else {
        ""
    };

    let query_str = format!(
        "SELECT re.* FROM rule_executions re {} {} ORDER BY re.detected_at DESC LIMIT ? OFFSET ?",
        join_clause, where_clause
    );

    let count_query = format!(
        "SELECT COUNT(*) FROM rule_executions re {} {}",
        join_clause, where_clause
    );

    let mut query = sqlx::query_as::<_, aads_core::models::Detection>(&query_str);
    for val in &bind_values {
        query = query.bind(val);
    }
    query = query.bind(size as i64);
    query = query.bind(((page - 1) * size) as i64);

    let detections = query.fetch_all(&state.db).await?;

    let mut count_query = sqlx::query_as::<_, (i64,)>(&count_query);
    for val in &bind_values {
        count_query = count_query.bind(val);
    }
    let total: (i64,) = count_query.fetch_one(&state.db).await?;

    Ok(Json(json!({
        "success": true,
        "data": detections,
        "meta": {
            "page": page,
            "size": size,
            "total": total.0
        }
    })))
}

pub async fn get_detection(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    let detection = sqlx::query_as::<_, aads_core::models::Detection>(
        "SELECT * FROM rule_executions WHERE id = ?"
    )
    .bind(id.to_string())
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Detection {} not found", id)))?;

    Ok(Json(json!({
        "success": true,
        "data": detection
    })))
}

pub async fn update_detection(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateDetectionRequest>,
) -> Result<Json<Value>, AppError> {
    let _existing = sqlx::query_as::<_, aads_core::models::Detection>(
        "SELECT * FROM rule_executions WHERE id = ?"
    )
    .bind(id.to_string())
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Detection {} not found", id)))?;

    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

    sqlx::query(
        "UPDATE rule_executions SET
            status = COALESCE(?, status),
            assignee = COALESCE(?, assignee),
            resolution_note = COALESCE(?, resolution_note),
            acknowledged_at = CASE WHEN ? = 'acknowledged' AND acknowledged_at IS NULL THEN ? ELSE acknowledged_at END,
            resolved_at = CASE WHEN ? IN ('resolved', 'false_positive') AND resolved_at IS NULL THEN ? ELSE resolved_at END
        WHERE id = ?"
    )
    .bind(&req.status)
    .bind(&req.assignee)
    .bind(&req.resolution_note)
    .bind(&req.status)
    .bind(&now)
    .bind(&req.status)
    .bind(&now)
    .bind(id.to_string())
    .execute(&state.db)
    .await?;

    let detection = sqlx::query_as::<_, aads_core::models::Detection>(
        "SELECT * FROM rule_executions WHERE id = ?"
    )
    .bind(id.to_string())
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({
        "success": true,
        "data": detection
    })))
}
