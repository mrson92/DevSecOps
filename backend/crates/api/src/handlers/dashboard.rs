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

    let critical: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM rule_executions re JOIN rules r ON re.rule_id = r.id WHERE r.severity = 'critical' AND re.status IN ('open', 'acknowledged', 'investigating')"
    )
    .fetch_one(&state.db)
    .await?;

    let high: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM rule_executions re JOIN rules r ON re.rule_id = r.id WHERE r.severity = 'high' AND re.status IN ('open', 'acknowledged', 'investigating')"
    )
    .fetch_one(&state.db)
    .await?;

    let medium: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM rule_executions re JOIN rules r ON re.rule_id = r.id WHERE r.severity = 'medium' AND re.status IN ('open', 'acknowledged', 'investigating')"
    )
    .fetch_one(&state.db)
    .await?;

    let low: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM rule_executions re JOIN rules r ON re.rule_id = r.id WHERE r.severity = 'low' AND re.status IN ('open', 'acknowledged', 'investigating')"
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({
        "success": true,
        "data": {
            "total_detections": total.0,
            "open_detections": open.0,
            "active_rules": active_rules.0,
            "critical_count": critical.0,
            "high_count": high.0,
            "medium_count": medium.0,
            "low_count": low.0
        }
    })))
}

pub async fn get_timeline(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        r#"SELECT strftime('%Y-%m-%d %H:00:00', detected_at) as hour, COUNT(*) as cnt
           FROM rule_executions
           WHERE detected_at >= datetime('now', '-24 hours')
           GROUP BY hour
           ORDER BY hour"#
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to fetch timeline: {}", e)))?;

    let timeline: Vec<Value> = rows.into_iter().map(|(hour, count)| {
        json!({ "time": hour, "count": count })
    }).collect();

    Ok(Json(json!({
        "success": true,
        "data": timeline
    })))
}

pub async fn get_top_rules(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let rows: Vec<(String, String, String, i64)> = sqlx::query_as(
        r#"SELECT r.id, r.name, r.severity, COUNT(re.id) as cnt
           FROM rule_executions re
           JOIN rules r ON re.rule_id = r.id
           WHERE re.detected_at >= datetime('now', '-7 days')
           GROUP BY r.id
           ORDER BY cnt DESC
           LIMIT 10"#
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to fetch top rules: {}", e)))?;

    let top_rules: Vec<Value> = rows.into_iter().map(|(id, name, severity, count)| {
        json!({ "id": id, "name": name, "severity": severity, "count": count })
    }).collect();

    Ok(Json(json!({
        "success": true,
        "data": top_rules
    })))
}

pub async fn get_top_ips(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        r#"SELECT group_key as ip, COUNT(*) as cnt
           FROM rule_executions
           WHERE detected_at >= datetime('now', '-7 days') AND group_key IS NOT NULL
           GROUP BY group_key
           ORDER BY cnt DESC
           LIMIT 10"#
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to fetch top IPs: {}", e)))?;

    let top_ips: Vec<Value> = rows.into_iter().map(|(ip, count)| {
        json!({ "ip": ip, "count": count })
    }).collect();

    Ok(Json(json!({
        "success": true,
        "data": top_ips
    })))
}
