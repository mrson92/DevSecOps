use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

use aads_core::state::AppState;
use aads_core::error::AppError;

pub async fn get_stats(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM rule_executions")
        .fetch_one(&state.db)
        .await?;

    let open: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM rule_executions WHERE status IN ('open', 'acknowledged', 'investigating')"
    )
    .fetch_one(&state.db)
    .await?;

    let active_rules: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM rules WHERE enabled = true")
        .fetch_one(&state.db)
        .await?;

    Ok(Json(json!({
        "success": true,
        "data": {
            "total_detections": total.0,
            "open_detections": open.0,
            "active_rules": active_rules.0
        }
    })))
}
