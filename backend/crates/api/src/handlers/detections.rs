use axum::extract::{Path, Query, State};
use axum::Json;
use serde_json::{json, Value};
use uuid::Uuid;

use aads_core::state::AppState;
use aads_core::error::AppError;
use aads_core::models::PaginationParams;

pub async fn list_detections(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, AppError> {
    let page = params.page.unwrap_or(1);
    let size = params.size.unwrap_or(20);

    let detections = sqlx::query_as::<_, aads_core::models::Detection>(
        "SELECT * FROM rule_executions ORDER BY detected_at DESC LIMIT ? OFFSET ?"
    )
    .bind(size as i64)
    .bind(((page - 1) * size) as i64)
    .fetch_all(&state.db)
    .await?;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM rule_executions")
        .fetch_one(&state.db)
        .await?;

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
