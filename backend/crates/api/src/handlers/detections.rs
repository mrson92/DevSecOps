use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use aads_core::state::AppState;
use aads_core::error::AppError;
use aads_core::models::{Detection, UpdateDetectionRequest};
use aads_engine::types::LogEntry;

use aads_engine::fp_filter::{build_fp_label, collect_label, label_for_status};
#[derive(Debug, Deserialize, Default)]
pub struct DetectionFilterParams {
    pub page: Option<u32>,
    pub size: Option<u32>,
    pub severity: Option<String>,
    pub status: Option<String>,
    pub rule_id: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub tactic: Option<String>,
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
    if let Some(ref tactic) = params.tactic {
        conditions.push("r.mitre_tactics LIKE ?");
        bind_values.push(format!("%{}\"%", tactic));
    }

    let needs_rules_join = params.severity.is_some() || params.tactic.is_some();

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let join_clause = if needs_rules_join {
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
    let _existing: Detection = sqlx::query_as::<_, Detection>(
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

    let detection = sqlx::query_as::<_, Detection>(
        "SELECT * FROM rule_executions WHERE id = ?"
    )
    .bind(id.to_string())
    .fetch_one(&state.db)
    .await?;

    // 5.5 지도학습 오탐 필터: 검출 상태가 오탐/진탐으로 최종 판정되면
    // 해당 검출의 피처를 `fp_labels` 인덱스에 학습 샘플로 수집한다.
    // - false_positive/suppressed → 라벨 1 (오탐)
    // - resolved                  → 라벨 0 (진탐)
    if let Some(label) = req.status.as_deref().and_then(label_for_status) {
        collect_label_for_detection(&state, &detection, label).await;
    }

    Ok(Json(json!({
        "success": true,
        "data": detection
    })))
}

/// 검출이 오탐(1)/진탐(0)으로 판정된 경우, 매칭된 로그에서 피처를 추출해
/// `fp_labels`에 학습 샘플로 적재한다.
///
/// `rule_executions.context`는 매칭된 `LogEntry` JSON 배열이므로 이를 파싱해
/// `build_fp_label`로 집계 피처를 만든다. 라벨 id는 검출 id 기반으로 고정해
/// 같은 검출을 여러 번 재라벨링해도 중복 적재를 방지한다. (best-effort)
async fn collect_label_for_detection(state: &AppState, updated: &Detection, label: u8) {
    let entries: Vec<LogEntry> = updated
        .context
        .as_deref()
        .map(|c| serde_json::from_str::<Vec<LogEntry>>(c).unwrap_or_default())
        .unwrap_or_default();

    if entries.is_empty() {
        return;
    }

    let prefix = if label == 1 { "fp" } else { "tp" };
    let label_id = format!("{}-{}", prefix, updated.id);
    let Some(label_doc) = build_fp_label(
        &label_id,
        &updated.id,
        &updated.rule_id,
        label,
        &updated.detected_at,
        &entries,
    ) else {
        return;
    };

    match collect_label(state.es.as_ref(), &label_doc).await {
        Ok(_) => {}
        Err(e) => {
            tracing::warn!(
                "Failed to collect fp label for detection {}: {}",
                updated.id,
                e
            );
        }
    }
}
