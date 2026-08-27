use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use aads_core::error::AppError;
use aads_core::models::{AiAgent, Persona};
use aads_core::state::AppState;

#[derive(Deserialize)]
pub struct ChatRequest {
    pub agent_id: String,
    pub message: String,
}

#[derive(Serialize)]
pub struct ChatResponse {
    pub reply: String,
    pub agent_id: String,
    pub agent_name: String,
    pub model: String,
    pub tokens_used: Option<i32>,
}

pub async fn chat(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<Value>, AppError> {
    let agent = sqlx::query_as::<_, AiAgent>("SELECT * FROM ai_agents WHERE id = ? AND enabled = true")
        .bind(&req.agent_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Agent '{}' not found or disabled", req.agent_id)))?;

    let persona = sqlx::query_as::<_, Persona>("SELECT * FROM personas WHERE id = ? AND enabled = true")
        .bind(&agent.persona_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Persona '{}' not found", agent.persona_id)))?;

    let api_key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| AppError::Validation("OPENAI_API_KEY not configured".to_string()))?;

    let client = reqwest::Client::new();

    let messages = vec![
        json!({
            "role": "system",
            "content": persona.system_prompt
        }),
        json!({
            "role": "user",
            "content": req.message
        }),
    ];

    let payload = json!({
        "model": persona.model,
        "messages": messages,
        "max_tokens": persona.max_tokens,
        "temperature": persona.temperature,
    });

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await
        .map_err(|e| AppError::Validation(format!("API request failed: {}", e)))?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    if !status.is_success() {
        let error_msg = parse_error_message(&body);
        return Err(AppError::Validation(format!("OpenAI API error: {}", error_msg)));
    }

    let parsed: Value = serde_json::from_str(&body)
        .map_err(|e| AppError::Validation(format!("Failed to parse response: {}", e)))?;

    let reply = parsed["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("No response")
        .to_string();

    let tokens_used = parsed["usage"]["total_tokens"]
        .as_i64()
        .map(|t| t as i32);

    let response = ChatResponse {
        reply,
        agent_id: agent.id.clone(),
        agent_name: agent.name.clone(),
        model: persona.model.clone(),
        tokens_used,
    };

    Ok(Json(json!({
        "success": true,
        "data": response
    })))
}

fn parse_error_message(body: &str) -> String {
    if let Ok(parsed) = serde_json::from_str::<Value>(body) {
        if let Some(msg) = parsed["error"]["message"].as_str() {
            return msg.to_string();
        }
    }
    body.to_string()
}
