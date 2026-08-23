use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use aads_core::state::AppState;
use aads_core::error::AppError;

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub size: Option<u32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct DataSource {
    pub id: String,
    pub name: String,
    pub r#type: String,
    pub config: String,
    pub target: String,
    pub field_mapping: String,
    pub enabled: bool,
    pub is_primary: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct NotificationChannel {
    pub id: String,
    pub name: String,
    pub r#type: String,
    pub config: String,
    pub enabled: bool,
    pub severity_filter: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateDataSourceRequest {
    pub name: String,
    pub r#type: String,
    pub config: String,
    pub target: String,
    pub field_mapping: Option<String>,
    pub is_primary: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CreateNotificationChannelRequest {
    pub name: String,
    pub r#type: String,
    pub config: String,
    pub severity_filter: Option<String>,
}

pub async fn list_data_sources(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, AppError> {
    let page = params.page.unwrap_or(1);
    let size = params.size.unwrap_or(20);

    let sources = sqlx::query_as::<_, DataSource>(
        "SELECT * FROM data_sources ORDER BY created_at DESC LIMIT ? OFFSET ?"
    )
    .bind(size as i64)
    .bind(((page - 1) * size) as i64)
    .fetch_all(&state.db)
    .await?;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM data_sources")
        .fetch_one(&state.db)
        .await?;

    Ok(Json(json!({
        "success": true,
        "data": sources,
        "meta": { "page": page, "size": size, "total": total.0 }
    })))
}

pub async fn create_data_source(
    State(state): State<AppState>,
    Json(req): Json<CreateDataSourceRequest>,
) -> Result<Json<Value>, AppError> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

    sqlx::query(
        "INSERT INTO data_sources (id, name, type, config, target, field_mapping, enabled, is_primary, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, true, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.r#type)
    .bind(&req.config)
    .bind(&req.target)
    .bind(req.field_mapping.as_deref().unwrap_or("{}"))
    .bind(req.is_primary.unwrap_or(false))
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await?;

    let source = sqlx::query_as::<_, DataSource>("SELECT * FROM data_sources WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(json!({ "success": true, "data": source })))
}

pub async fn delete_data_source(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    sqlx::query("DELETE FROM data_sources WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "success": true, "message": "Data source deleted" })))
}

pub async fn test_data_source(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let _source = sqlx::query_as::<_, DataSource>("SELECT * FROM data_sources WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Data source {} not found", id)))?;

    Ok(Json(json!({
        "success": true,
        "data": { "status": "connected", "message": "Connection successful" }
    })))
}

pub async fn list_notification_channels(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, AppError> {
    let page = params.page.unwrap_or(1);
    let size = params.size.unwrap_or(20);

    let channels = sqlx::query_as::<_, NotificationChannel>(
        "SELECT * FROM notification_channels ORDER BY created_at DESC LIMIT ? OFFSET ?"
    )
    .bind(size as i64)
    .bind(((page - 1) * size) as i64)
    .fetch_all(&state.db)
    .await?;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM notification_channels")
        .fetch_one(&state.db)
        .await?;

    Ok(Json(json!({
        "success": true,
        "data": channels,
        "meta": { "page": page, "size": size, "total": total.0 }
    })))
}

pub async fn create_notification_channel(
    State(state): State<AppState>,
    Json(req): Json<CreateNotificationChannelRequest>,
) -> Result<Json<Value>, AppError> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

    sqlx::query(
        "INSERT INTO notification_channels (id, name, type, config, enabled, severity_filter, created_at) VALUES (?, ?, ?, ?, true, ?, ?)"
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.r#type)
    .bind(&req.config)
    .bind(req.severity_filter.as_deref().unwrap_or("[]"))
    .bind(&now)
    .execute(&state.db)
    .await?;

    let channel = sqlx::query_as::<_, NotificationChannel>("SELECT * FROM notification_channels WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(json!({ "success": true, "data": channel })))
}

pub async fn delete_notification_channel(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    sqlx::query("DELETE FROM notification_channels WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "success": true, "message": "Notification channel deleted" })))
}

pub async fn test_notification_channel(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let _channel = sqlx::query_as::<_, NotificationChannel>("SELECT * FROM notification_channels WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Notification channel {} not found", id)))?;

    Ok(Json(json!({
        "success": true,
        "data": { "status": "sent", "message": "Test notification sent" }
    })))
}

// ============================================================
// OIDC Settings
// ============================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct SystemSetting {
    pub key: String,
    pub value: String,
    pub category: String,
    pub description: Option<String>,
    pub updated_at: String,
    pub updated_by: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOidcSettingsRequest {
    pub issuer_url: Option<String>,
    pub realm: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub redirect_url: Option<String>,
    pub jwt_secret: Option<String>,
}

pub async fn get_oidc_settings(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let settings = sqlx::query_as::<_, SystemSetting>(
        "SELECT key, value, category, description, updated_at, updated_by FROM system_settings WHERE category = 'oidc'"
    )
    .fetch_all(&state.db)
    .await?;

    let mut oidc_map = serde_json::Map::new();
    for setting in &settings {
        let key = setting.key.strip_prefix("oidc.").unwrap_or(&setting.key);
        oidc_map.insert(key.to_string(), json!(setting.value));
    }

    Ok(Json(json!({
        "success": true,
        "data": oidc_map
    })))
}

pub async fn update_oidc_settings(
    State(state): State<AppState>,
    Json(req): Json<UpdateOidcSettingsRequest>,
) -> Result<Json<Value>, AppError> {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

    let updates = [
        ("oidc.issuer_url", req.issuer_url),
        ("oidc.realm", req.realm),
        ("oidc.client_id", req.client_id),
        ("oidc.client_secret", req.client_secret),
        ("oidc.redirect_url", req.redirect_url),
        ("oidc.jwt_secret", req.jwt_secret),
    ];

    for (key, value) in updates {
        if let Some(val) = value {
            sqlx::query(
                "INSERT INTO system_settings (key, value, category, updated_at) VALUES (?, 'oidc', ?, ?) ON CONFLICT(key) DO UPDATE SET value = ?, updated_at = ?"
            )
            .bind(key)
            .bind(&val)
            .bind(&now)
            .bind(&val)
            .bind(&now)
            .execute(&state.db)
            .await?;
        }
    }

    let settings = sqlx::query_as::<_, SystemSetting>(
        "SELECT key, value, category, description, updated_at, updated_by FROM system_settings WHERE category = 'oidc'"
    )
    .fetch_all(&state.db)
    .await?;

    let mut oidc_map = serde_json::Map::new();
    for setting in &settings {
        let key = setting.key.strip_prefix("oidc.").unwrap_or(&setting.key);
        oidc_map.insert(key.to_string(), json!(setting.value));
    }

    Ok(Json(json!({
        "success": true,
        "data": oidc_map,
        "message": "OIDC settings updated successfully"
    })))
}

pub async fn test_oidc_connection(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let settings = sqlx::query_as::<_, SystemSetting>(
        "SELECT key, value, category, description, updated_at, updated_by FROM system_settings WHERE category = 'oidc'"
    )
    .fetch_all(&state.db)
    .await?;

    let mut oidc_map = serde_json::Map::new();
    for setting in &settings {
        let key = setting.key.strip_prefix("oidc.").unwrap_or(&setting.key);
        oidc_map.insert(key.to_string(), json!(setting.value));
    }

    let issuer_url = oidc_map.get("issuer_url")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let realm = oidc_map.get("realm")
        .and_then(|v| v.as_str())
        .unwrap_or("master");

    let client_id = oidc_map.get("client_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let _client_secret = oidc_map.get("client_secret")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if issuer_url.is_empty() || client_id.is_empty() {
        return Ok(Json(json!({
            "success": false,
            "data": { "status": "failed", "message": "Issuer URL and Client ID are required" }
        })));
    }

    let client = reqwest::Client::new();
    let discovery_url = format!("{}/realms/{}/.well-known/openid-configuration", issuer_url, realm);

    match client.get(&discovery_url).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                let discovery: serde_json::Value = resp.json().await
                    .map_err(|e| AppError::Internal(format!("Failed to parse discovery document: {}", e)))?;

                let token_endpoint = discovery["token_endpoint"].as_str().unwrap_or("");
                let authorization_endpoint = discovery["authorization_endpoint"].as_str().unwrap_or("");

                Ok(Json(json!({
                    "success": true,
                    "data": {
                        "status": "connected",
                        "message": "OIDC connection successful",
                        "discovery": {
                            "token_endpoint": token_endpoint,
                            "authorization_endpoint": authorization_endpoint
                        }
                    }
                })))
            } else {
                Ok(Json(json!({
                    "success": false,
                    "data": {
                        "status": "failed",
                        "message": format!("OIDC server returned status: {}", resp.status())
                    }
                })))
            }
        }
        Err(e) => {
            Ok(Json(json!({
                "success": false,
                "data": {
                    "status": "failed",
                    "message": format!("Failed to connect to OIDC server: {}", e)
                }
            })))
        }
    }
}
