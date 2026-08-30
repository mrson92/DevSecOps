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

pub async fn get_mitre_tactics(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    // Each rule can carry multiple tactics (stored as a JSON array string like
    // '["TA0006","TA0007"]'). We explode each detection's rule tactics and count
    // how many detections fall into each MITRE tactic over the last 7 days.
    let mut counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let rule_rows: Vec<(String, String)> = sqlx::query_as(
        r#"SELECT DISTINCT re.id, r.mitre_tactics AS tactics
           FROM rule_executions re
           JOIN rules r ON re.rule_id = r.id
           WHERE re.detected_at >= datetime('now', '-7 days')
             AND r.mitre_tactics IS NOT NULL
             AND r.mitre_tactics <> ''
             AND r.mitre_tactics <> '[]' "#
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to fetch tactic mapping: {}", e)))?;

    for (_detection_id, tactics) in rule_rows {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&tactics) {
            if let Some(arr) = v.as_array() {
                for item in arr {
                    if let Some(tactic) = item.as_str() {
                        *counts.entry(tactic.to_string()).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    let mut data: Vec<Value> = counts
        .iter()
        .map(|(tactic, count)| json!({ "tactic": tactic, "count": count }))
        .collect();
    data.sort_by(|a, b| b["count"].as_i64().unwrap_or(0).cmp(&a["count"].as_i64().unwrap_or(0)));

    Ok(Json(json!({
        "success": true,
        "data": data
    })))
}
