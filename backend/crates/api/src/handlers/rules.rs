use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use aads_core::state::AppState;
use aads_core::error::AppError;
use aads_core::models::{CreateRuleRequest, UpdateRuleRequest, TestRuleRequest};

#[derive(Debug, Deserialize, Default)]
pub struct RuleFilterParams {
    pub page: Option<u32>,
    pub size: Option<u32>,
    pub severity: Option<String>,
    pub rule_type: Option<String>,
    pub enabled: Option<bool>,
}

pub async fn list_rules(
    State(state): State<AppState>,
    Query(params): Query<RuleFilterParams>,
) -> Result<Json<Value>, AppError> {
    let page = params.page.unwrap_or(1);
    let size = params.size.unwrap_or(20);

    let mut conditions = Vec::new();
    let mut bind_values: Vec<String> = Vec::new();

    if let Some(enabled) = params.enabled {
        if enabled {
            conditions.push("enabled = true".to_string());
        }
    } else {
        conditions.push("enabled = true".to_string());
    }
    if let Some(ref severity) = params.severity {
        conditions.push("severity = ?".to_string());
        bind_values.push(severity.clone());
    }
    if let Some(ref rule_type) = params.rule_type {
        conditions.push("rule_type = ?".to_string());
        bind_values.push(rule_type.clone());
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let query_str = format!(
        "SELECT * FROM rules {} ORDER BY created_at DESC LIMIT ? OFFSET ?",
        where_clause
    );
    let count_query = format!("SELECT COUNT(*) FROM rules {}", where_clause);

    let mut query = sqlx::query_as::<_, aads_core::models::Rule>(&query_str);
    for val in &bind_values {
        query = query.bind(val);
    }
    query = query.bind(size as i64);
    query = query.bind(((page - 1) * size) as i64);

    let rules = query.fetch_all(&state.db).await?;

    let mut count_query = sqlx::query_as::<_, (i64,)>(&count_query);
    for val in &bind_values {
        count_query = count_query.bind(val);
    }
    let total: (i64,) = count_query.fetch_one(&state.db).await?;

    Ok(Json(json!({
        "success": true,
        "data": rules,
        "meta": {
            "page": page,
            "size": size,
            "total": total.0
        }
    })))
}

pub async fn get_rule(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let rule = sqlx::query_as::<_, aads_core::models::Rule>(
        "SELECT * FROM rules WHERE id = ?"
    )
    .bind(id.to_string())
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Rule {} not found", id)))?;

    Ok(Json(json!({
        "success": true,
        "data": rule
    })))
}

pub async fn create_rule(
    State(state): State<AppState>,
    Json(req): Json<CreateRuleRequest>,
) -> Result<Json<Value>, AppError> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

    sqlx::query(
        r#"INSERT INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, "references", version, created_at, updated_at)
        VALUES (?, ?, ?, ?, true, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, ?, ?)"#
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.severity)
    .bind(&req.rule_type)
    .bind(&req.condition)
    .bind(req.window_sec.unwrap_or(300))
    .bind(req.slide_sec.unwrap_or(60))
    .bind(req.group_by.as_deref().unwrap_or("[]"))
    .bind(req.actions.as_deref().unwrap_or("[]"))
    .bind(req.mitre_tactics.as_deref().unwrap_or("[]"))
    .bind(req.mitre_techniques.as_deref().unwrap_or("[]"))
    .bind(req.references.as_deref().unwrap_or("[]"))
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await?;

    let rule = sqlx::query_as::<_, aads_core::models::Rule>(
        "SELECT * FROM rules WHERE id = ?"
    )
    .bind(&id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({
        "success": true,
        "data": rule
    })))
}

pub async fn update_rule(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateRuleRequest>,
) -> Result<Json<Value>, AppError> {
    let rule_version = sqlx::query_as::<_, aads_core::models::Rule>(
        "SELECT * FROM rules WHERE id = ?"
    )
    .bind(id.to_string())
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Rule {} not found", id)))?;

    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    let new_version = rule_version.version + 1;

    sqlx::query(
        r#"UPDATE rules SET
            name = COALESCE(?, name),
            description = COALESCE(?, description),
            severity = COALESCE(?, severity),
            enabled = COALESCE(?, enabled),
            condition = COALESCE(?, condition),
            window_sec = COALESCE(?, window_sec),
            slide_sec = COALESCE(?, slide_sec),
            group_by = COALESCE(?, group_by),
            actions = COALESCE(?, actions),
            mitre_tactics = COALESCE(?, mitre_tactics),
            mitre_techniques = COALESCE(?, mitre_techniques),
            "references" = COALESCE(?, "references"),
            version = ?,
            updated_at = ?
        WHERE id = ?"#
    )
    .bind(req.name)
    .bind(req.description)
    .bind(req.severity)
    .bind(req.enabled)
    .bind(req.condition)
    .bind(req.window_sec)
    .bind(req.slide_sec)
    .bind(req.group_by)
    .bind(req.actions)
    .bind(req.mitre_tactics)
    .bind(req.mitre_techniques)
    .bind(req.references)
    .bind(new_version)
    .bind(&now)
    .bind(id.to_string())
    .execute(&state.db)
    .await?;

    let rule = sqlx::query_as::<_, aads_core::models::Rule>(
        "SELECT * FROM rules WHERE id = ?"
    )
    .bind(id.to_string())
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({
        "success": true,
        "data": rule
    })))
}

pub async fn delete_rule(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let _existing = sqlx::query_as::<_, aads_core::models::Rule>(
        "SELECT * FROM rules WHERE id = ?"
    )
    .bind(id.to_string())
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Rule {} not found", id)))?;

    sqlx::query("UPDATE rules SET enabled = false, updated_at = ? WHERE id = ?")
        .bind(chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
        .bind(id.to_string())
        .execute(&state.db)
        .await?;

    Ok(Json(json!({
        "success": true,
        "message": "Rule deleted successfully"
    })))
}

pub async fn test_rule(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<TestRuleRequest>,
) -> Result<Json<Value>, AppError> {
    let rule = sqlx::query_as::<_, aads_core::models::Rule>(
        "SELECT * FROM rules WHERE id = ?"
    )
    .bind(id.to_string())
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Rule {} not found", id)))?;

    let start_time = std::time::Instant::now();
    let engine = aads_engine::RuleEngine::new(state.db.clone(), state.es.clone());

    let logs = engine.fetch_logs_from_es(&rule).await
        .map_err(|e| AppError::RuleEngine(format!("Failed to fetch logs: {}", e)))?;

    let result = engine.execute_rule(&rule, logs).await
        .map_err(|e| AppError::RuleEngine(format!("Failed to execute rule: {}", e)))?;

    let execution_time_ms = start_time.elapsed().as_millis() as u64;

    let test_id = uuid::Uuid::new_v4().to_string();
    let test_result = serde_json::json!({
        "id": test_id,
        "rule_id": id.to_string(),
        "test_type": req.test_type,
        "matched_count": result.matched_count,
        "matched_logs": result.matched_entries,
        "execution_time_ms": execution_time_ms,
        "status": "completed",
        "error_message": null
    });

    sqlx::query(
        "INSERT INTO rule_tests (id, rule_id, rule_snapshot, test_type, result, status, completed_at)
        VALUES (?, ?, ?, ?, ?, 'completed', ?)"
    )
    .bind(&test_id)
    .bind(id.to_string())
    .bind(serde_json::to_string(&rule).unwrap_or_default())
    .bind(&req.test_type)
    .bind(test_result.to_string())
    .bind(chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
    .execute(&state.db)
    .await?;

    Ok(Json(json!({
        "success": true,
        "data": test_result
    })))
}
