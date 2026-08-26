use axum::extract::{Path, State};
use axum::Json;
use serde_json::{json, Value};
use uuid::Uuid;

use aads_core::state::AppState;
use aads_core::error::AppError;
use aads_core::models::{CreateAgentRequest, UpdateAgentRequest, AiAgent, AiAgentRun};
use aads_engine::agent_runner::AgentRunner;

pub async fn list_agents(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let agents = sqlx::query_as::<_, AiAgent>("SELECT * FROM ai_agents ORDER BY created_at DESC")
        .fetch_all(&state.db)
        .await?;

    Ok(Json(json!({
        "success": true,
        "data": agents
    })))
}

pub async fn get_agent(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let agent = sqlx::query_as::<_, AiAgent>("SELECT * FROM ai_agents WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Agent {} not found", id)))?;

    Ok(Json(json!({
        "success": true,
        "data": agent
    })))
}

pub async fn create_agent(
    State(state): State<AppState>,
    Json(req): Json<CreateAgentRequest>,
) -> Result<Json<Value>, AppError> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let agent_type = req.agent_type.unwrap_or_else(|| "analyzer".to_string());
    let config = req.config.unwrap_or_else(|| "{}".to_string());

    sqlx::query(
        r#"INSERT INTO ai_agents (id, name, description, persona_id, agent_type, enabled, config, schedule, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, true, ?, ?, ?, ?)"#
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.persona_id)
    .bind(&agent_type)
    .bind(&config)
    .bind(&req.schedule)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await?;

    let agent = sqlx::query_as::<_, AiAgent>("SELECT * FROM ai_agents WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(json!({
        "success": true,
        "data": agent
    })))
}

pub async fn update_agent(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateAgentRequest>,
) -> Result<Json<Value>, AppError> {
    let existing = sqlx::query_as::<_, AiAgent>("SELECT * FROM ai_agents WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Agent {} not found", id)))?;

    let name = req.name.unwrap_or(existing.name);
    let description = req.description.or(existing.description);
    let persona_id = req.persona_id.unwrap_or(existing.persona_id);
    let agent_type = req.agent_type.unwrap_or(existing.agent_type);
    let enabled = req.enabled.unwrap_or(existing.enabled);
    let config = req.config.unwrap_or(existing.config);
    let schedule = req.schedule.or(existing.schedule);
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"UPDATE ai_agents SET name=?, description=?, persona_id=?, agent_type=?, enabled=?, config=?, schedule=?, updated_at=?
           WHERE id=?"#
    )
    .bind(&name)
    .bind(&description)
    .bind(&persona_id)
    .bind(&agent_type)
    .bind(enabled)
    .bind(&config)
    .bind(&schedule)
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await?;

    let agent = sqlx::query_as::<_, AiAgent>("SELECT * FROM ai_agents WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(json!({
        "success": true,
        "data": agent
    })))
}

pub async fn delete_agent(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let result = sqlx::query("DELETE FROM ai_agents WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("Agent {} not found", id)));
    }

    Ok(Json(json!({
        "success": true,
        "message": format!("Agent {} deleted", id)
    })))
}

pub async fn run_agent_now(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let agent = sqlx::query_as::<_, AiAgent>("SELECT * FROM ai_agents WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Agent {} not found", id)))?;

    let runner = AgentRunner::new(state.db.clone());
    let run = runner.run_agent(&agent).await?;

    Ok(Json(json!({
        "success": true,
        "data": run
    })))
}

pub async fn list_agent_runs(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let runs = sqlx::query_as::<_, AiAgentRun>(
        "SELECT * FROM ai_agent_runs WHERE agent_id = ? ORDER BY started_at DESC LIMIT 50"
    )
    .bind(&id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({
        "success": true,
        "data": runs
    })))
}
