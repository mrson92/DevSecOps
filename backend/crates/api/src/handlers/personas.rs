use axum::extract::{Path, State};
use axum::Json;
use serde_json::{json, Value};
use uuid::Uuid;

use aads_core::state::AppState;
use aads_core::error::AppError;
use aads_core::models::{CreatePersonaRequest, UpdatePersonaRequest, Persona};

pub async fn list_personas(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let personas = sqlx::query_as::<_, Persona>("SELECT * FROM personas ORDER BY created_at DESC")
        .fetch_all(&state.db)
        .await?;

    Ok(Json(json!({
        "success": true,
        "data": personas
    })))
}

pub async fn get_persona(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let persona = sqlx::query_as::<_, Persona>("SELECT * FROM personas WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Persona {} not found", id)))?;

    Ok(Json(json!({
        "success": true,
        "data": persona
    })))
}

pub async fn create_persona(
    State(state): State<AppState>,
    Json(req): Json<CreatePersonaRequest>,
) -> Result<Json<Value>, AppError> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let model = req.model.unwrap_or_else(|| "gpt-4".to_string());
    let temperature = req.temperature.unwrap_or(0.7);
    let max_tokens = req.max_tokens.unwrap_or(4096);
    let tools = req.tools.unwrap_or_else(|| "[]".to_string());

    sqlx::query(
        r#"INSERT INTO personas (id, name, description, system_prompt, model, temperature, max_tokens, tools, enabled, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, true, ?, ?)"#
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.system_prompt)
    .bind(&model)
    .bind(temperature)
    .bind(max_tokens)
    .bind(&tools)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await?;

    let persona = sqlx::query_as::<_, Persona>("SELECT * FROM personas WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(json!({
        "success": true,
        "data": persona
    })))
}

pub async fn update_persona(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdatePersonaRequest>,
) -> Result<Json<Value>, AppError> {
    let existing = sqlx::query_as::<_, Persona>("SELECT * FROM personas WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Persona {} not found", id)))?;

    let name = req.name.unwrap_or(existing.name);
    let description = req.description.or(existing.description);
    let system_prompt = req.system_prompt.unwrap_or(existing.system_prompt);
    let model = req.model.unwrap_or(existing.model);
    let temperature = req.temperature.unwrap_or(existing.temperature);
    let max_tokens = req.max_tokens.unwrap_or(existing.max_tokens);
    let tools = req.tools.unwrap_or(existing.tools);
    let enabled = req.enabled.unwrap_or(existing.enabled);
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"UPDATE personas SET name=?, description=?, system_prompt=?, model=?, temperature=?, max_tokens=?, tools=?, enabled=?, updated_at=?
           WHERE id=?"#
    )
    .bind(&name)
    .bind(&description)
    .bind(&system_prompt)
    .bind(&model)
    .bind(temperature)
    .bind(max_tokens)
    .bind(&tools)
    .bind(enabled)
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await?;

    let persona = sqlx::query_as::<_, Persona>("SELECT * FROM personas WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(json!({
        "success": true,
        "data": persona
    })))
}

pub async fn delete_persona(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let agents_using = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM ai_agents WHERE persona_id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await?;

    if agents_using.0 > 0 {
        return Err(AppError::Validation(format!(
            "Cannot delete persona: {} agent(s) are using it", agents_using.0
        )));
    }

    let result = sqlx::query("DELETE FROM personas WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("Persona {} not found", id)));
    }

    Ok(Json(json!({
        "success": true,
        "message": format!("Persona {} deleted", id)
    })))
}
