use std::sync::Arc;

use chrono::Utc;
use sqlx::SqlitePool;
use tracing::{info, error};
use uuid::Uuid;

use aads_core::models::{AiAgent, AiAgentRun, Persona};
use aads_core::error::AppError;
use aads_core::state::ElasticSearchClientTrait;

use crate::stat::SecurityStat;

#[derive(Clone)]
pub struct AgentRunner {
    db: SqlitePool,
    es: Arc<dyn ElasticSearchClientTrait>,
    http_client: reqwest::Client,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AgentConfig {
    pub api_url: Option<String>,
    pub api_key: Option<String>,
    pub model_override: Option<String>,
}

impl AgentRunner {
    pub fn new(db: SqlitePool, es: Arc<dyn ElasticSearchClientTrait>) -> Self {
        Self {
            db,
            es,
            http_client: reqwest::Client::new(),
        }
    }

    pub async fn run_scheduled_agents(&self) -> Result<Vec<AiAgentRun>, AppError> {
        let agents = sqlx::query_as::<_, AiAgent>(
            "SELECT * FROM ai_agents WHERE enabled = true AND schedule IS NOT NULL"
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Database(e))?;

        let mut runs = Vec::new();
        for agent in agents {
            match self.run_agent(&agent).await {
                Ok(run) => runs.push(run),
                Err(e) => {
                    error!("Failed to run agent '{}': {}", agent.name, e);
                }
            }
        }
        Ok(runs)
    }

    pub async fn run_agent(&self, agent: &AiAgent) -> Result<AiAgentRun, AppError> {
        let persona = self.get_persona(&agent.persona_id).await?;
        let run_id = Uuid::new_v4().to_string();
        let started_at = Utc::now().to_rfc3339();

        let config: AgentConfig = serde_json::from_str(&agent.config)
            .unwrap_or_default();

        let api_url = config.api_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1/chat/completions");
        let api_key = match config.api_key.as_deref() {
            Some(key) => key.to_string(),
            None => std::env::var("OPENAI_API_KEY").unwrap_or_default(),
        };

        let model = config.model_override.as_deref().unwrap_or(&persona.model);

        let input_context = self.prepare_input_context(agent).await?;

        let messages = vec![
            serde_json::json!({
                "role": "system",
                "content": persona.system_prompt
            }),
            serde_json::json!({
                "role": "user",
                "content": input_context
            }),
        ];

        let payload = serde_json::json!({
            "model": model,
            "messages": messages,
            "max_tokens": persona.max_tokens,
            "temperature": persona.temperature,
        });

        info!("Running agent '{}' with model {}", agent.name, model);

        let response = self.http_client
            .post(api_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .timeout(std::time::Duration::from_secs(120))
            .send()
            .await;

        let (status, output, error_message, token_usage) = match response {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let body = resp.text().await.unwrap_or_default();

                if status == 200 {
                    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
                    let content = parsed["choices"][0]["message"]["content"]
                        .as_str()
                        .unwrap_or("No response content")
                        .to_string();
                    let tokens = parsed["usage"]["total_tokens"]
                        .as_i64()
                        .map(|t| t as i32)
                        .unwrap_or(0);
                    (status, Some(content), None, Some(tokens))
                } else {
                    let error_msg = parsed_error_message(&body);
                    (status, None, Some(error_msg), None)
                }
            }
            Err(e) => {
                error!("Agent '{}' request failed: {}", agent.name, e);
                (0, None, Some(format!("Request failed: {}", e)), None)
            }
        };

        let completed_at = Utc::now().to_rfc3339();
        let run_status = if status == 200 { "completed" } else { "failed" };

        let run = sqlx::query_as::<_, AiAgentRun>(
            r#"INSERT INTO ai_agent_runs (id, agent_id, started_at, completed_at, status, input, output, error_message, token_usage, duration_ms)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             RETURNING *"#,
        )
        .bind(&run_id)
        .bind(&agent.id)
        .bind(&started_at)
        .bind(&completed_at)
        .bind(run_status)
        .bind(&input_context)
        .bind(&output)
        .bind(&error_message)
        .bind(&token_usage)
        .bind(0i32)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Database(e))?;

        info!(
            "Agent '{}' run completed: status={}, tokens={:?}",
            agent.name, run_status, token_usage
        );

        Ok(run)
    }

    async fn get_persona(&self, persona_id: &str) -> Result<Persona, AppError> {
        sqlx::query_as::<_, Persona>("SELECT * FROM personas WHERE id = ? AND enabled = true")
            .bind(persona_id)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| AppError::Database(e))?
            .ok_or_else(|| AppError::NotFound(format!("Persona '{}' not found", persona_id)))
    }

    async fn prepare_input_context(&self, agent: &AiAgent) -> Result<String, AppError> {
        let recent_logs = sqlx::query_scalar::<_, String>(
            r#"SELECT message FROM logs ORDER BY timestamp DESC LIMIT 50"#,
        )
        .fetch_all(&self.db)
        .await
        .unwrap_or_default();

        let recent_detections = sqlx::query_scalar::<_, String>(
            r#"SELECT json_extract(rule_data, '$.rule_name') || ': ' || json_extract(rule_data, '$.message')
               FROM detections ORDER BY created_at DESC LIMIT 20"#,
        )
        .fetch_all(&self.db)
        .await
        .unwrap_or_default();

        // 2차 분석 입력: ES의 security_stat(고위험군 집계 이벤트)을 조회한다.
        // raw 로그가 아닌 AI 검토용 최적화 통계만을 LLM에 전달한다.
        let security_stats = self.load_security_stats().await;

        let context = serde_json::json!({
            "agent": {
                "name": agent.name,
                "type": agent.agent_type,
                "description": agent.description,
            },
            "recent_logs_count": recent_logs.len(),
            "recent_logs_sample": recent_logs.iter().take(10).cloned().collect::<Vec<_>>(),
            "recent_detections": recent_detections,
            "security_stats": security_stats,
            "timestamp": Utc::now().to_rfc3339(),
        });

        Ok(serde_json::to_string_pretty(&context).unwrap_or_default())
    }

    /// ES `security_stat` 인덱스에서 최근 고위험군 집계 통계를 조회한다.
    ///
    /// 최신순 20건을 가져와 LLM 판단 입력으로 사용한다. 인덱스가 없거나
    /// 조회에 실패하면 빈 벡터를 돌려 AI 실행 자체는 차단하지 않는다.
    async fn load_security_stats(&self) -> Vec<SecurityStat> {
        let index = "security_stat";
        if !self.es.index_exists(index).await.unwrap_or(false) {
            return Vec::new();
        }

        let query = serde_json::json!({
            "sort": [{ "timestamp": "desc" }],
            "size": 20
        });

        match self.es.search(index, query).await {
            Ok(resp) => {
                let hits = resp["hits"]["hits"].as_array().cloned().unwrap_or_default();
                hits.into_iter()
                    .filter_map(|hit| {
                        let src = hit.get("_source")?;
                        serde_json::from_value::<SecurityStat>(src.clone()).ok()
                    })
                    .collect()
            }
            Err(e) => {
                error!("Failed to load security_stats for agent context: {}", e);
                Vec::new()
            }
        }
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            api_url: None,
            api_key: None,
            model_override: None,
        }
    }
}

fn parsed_error_message(body: &str) -> String {
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(msg) = parsed["error"]["message"].as_str() {
            return msg.to_string();
        }
        if let Some(msg) = parsed["message"].as_str() {
            return msg.to_string();
        }
    }
    body.to_string()
}
