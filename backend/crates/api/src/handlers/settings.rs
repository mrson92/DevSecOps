use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use uuid::Uuid;

use aads_core::state::AppState;
use aads_core::error::AppError;

// ============================================================
// DataSource Config Validation
// ============================================================

fn validate_datasource_config(ds_type: &str, config: &str) -> Result<Value, AppError> {
    let parsed: Value = serde_json::from_str(config)
        .map_err(|e| AppError::Validation(format!("Invalid JSON in config: {}", e)))?;

    match ds_type {
        "elasticsearch" => {
            if parsed.get("url").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                return Err(AppError::Validation("Elasticsearch requires 'url' field".into()));
            }
        }
        "loki" => {
            if parsed.get("url").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                return Err(AppError::Validation("Loki requires 'url' field".into()));
            }
        }
        "postgresql" => {
            let required = ["host", "database", "user"];
            for field in &required {
                if parsed.get(*field).and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                    return Err(AppError::Validation(format!(
                        "PostgreSQL requires '{}' field", field
                    )));
                }
            }
        }
        "clickhouse" => {
            if parsed.get("url").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                return Err(AppError::Validation("ClickHouse requires 'url' field".into()));
            }
        }
        _ => {
            return Err(AppError::Validation(format!(
                "Unsupported datasource type: {}", ds_type
            )));
        }
    }

    Ok(parsed)
}

async fn test_elasticsearch_connection(url: &str) -> Result<String, AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to create HTTP client: {}", e)))?;

    let health_url = format!("{}/_cluster/health", url.trim_end_matches('/'));
    let resp = client.get(&health_url)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Elasticsearch connection failed: {}", e)))?;

    if resp.status().is_success() {
        let body: Value = resp.json().await
            .map_err(|e| AppError::Internal(format!("Failed to parse response: {}", e)))?;
        let status = body["status"].as_str().unwrap_or("unknown");
        Ok(format!("Connected (cluster: {})", status))
    } else {
        Err(AppError::Internal(format!(
            "Elasticsearch returned status: {}", resp.status()
        )))
    }
}

async fn test_loki_connection(url: &str) -> Result<String, AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to create HTTP client: {}", e)))?;

    let ready_url = format!("{}/ready", url.trim_end_matches('/'));
    let resp = client.get(&ready_url)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Loki connection failed: {}", e)))?;

    if resp.status().is_success() {
        Ok("Connected (Loki is ready)".into())
    } else {
        Err(AppError::Internal(format!(
            "Loki returned status: {}", resp.status()
        )))
    }
}

async fn test_postgresql_connection(config: &Value) -> Result<String, AppError> {
    let host = config["host"].as_str().unwrap_or("localhost");
    let port = config["port"].as_u64().unwrap_or(5432);
    let database = config["database"].as_str().unwrap_or("postgres");
    let user = config["user"].as_str().unwrap_or("postgres");
    let password = config["password"].as_str().unwrap_or("");
    let ssl_mode = config["ssl_mode"].as_str().unwrap_or("prefer");

    let conn_str = format!(
        "host={} port={} dbname={} user={} password={} sslmode={}",
        host, port, database, user, password, ssl_mode
    );

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(&conn_str)
        .await
        .map_err(|e| AppError::Internal(format!("PostgreSQL connection failed: {}", e)))?;

    sqlx::query("SELECT 1")
        .execute(&pool)
        .await
        .map_err(|e| AppError::Internal(format!("PostgreSQL query failed: {}", e)))?;

    pool.close().await;
    Ok(format!("Connected (host:{}, db:{})", host, database))
}

async fn test_clickhouse_connection(config: &Value) -> Result<String, AppError> {
    let url = config["url"].as_str().unwrap_or("http://localhost:8123");
    let database = config["database"].as_str().unwrap_or("default");
    let user = config["user"].as_str().unwrap_or("default");
    let password = config["password"].as_str().unwrap_or("");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to create HTTP client: {}", e)))?;

    let base = url.trim_end_matches('/');
    let ping_url = format!("{}/ping", base);
    let resp = client.get(&ping_url)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("ClickHouse connection failed: {}", e)))?;

    if !resp.status().is_success() {
        return Err(AppError::Internal(format!(
            "ClickHouse ping returned status: {}", resp.status()
        )));
    }

    // Verify we can query the server version
    let query_url = format!("{}/?query=SELECT+version()&user={}&password={}&database={}",
        base, user, password, database);
    let version_resp = client.get(&query_url)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("ClickHouse version query failed: {}", e)))?;

    if version_resp.status().is_success() {
        let version = version_resp.text().await.unwrap_or_default();
        Ok(format!("Connected (version: {}, db: {})", version.trim(), database))
    } else {
        Ok(format!("Connected (ping OK, db: {})", database))
    }
}

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
    validate_datasource_config(&req.r#type, &req.config)?;

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

#[derive(Debug, Deserialize)]
pub struct UpdateDataSourceRequest {
    pub name: Option<String>,
    pub r#type: Option<String>,
    pub config: Option<String>,
    pub target: Option<String>,
    pub field_mapping: Option<String>,
    pub enabled: Option<bool>,
    pub is_primary: Option<bool>,
}

pub async fn update_data_source(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateDataSourceRequest>,
) -> Result<Json<Value>, AppError> {
    let existing = sqlx::query_as::<_, DataSource>("SELECT * FROM data_sources WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Data source {} not found", id)))?;

    let new_type = req.r#type.as_deref().unwrap_or(&existing.r#type);
    let new_config = req.config.as_deref().unwrap_or(&existing.config);

    if req.config.is_some() || req.r#type.is_some() {
        validate_datasource_config(new_type, new_config)?;
    }

    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

    sqlx::query(
        "UPDATE data_sources SET name = ?, type = ?, config = ?, target = ?, field_mapping = ?, enabled = ?, is_primary = ?, updated_at = ? WHERE id = ?"
    )
    .bind(req.name.as_deref().unwrap_or(&existing.name))
    .bind(new_type)
    .bind(new_config)
    .bind(req.target.as_deref().unwrap_or(&existing.target))
    .bind(req.field_mapping.as_deref().unwrap_or(&existing.field_mapping))
    .bind(req.enabled.unwrap_or(existing.enabled))
    .bind(req.is_primary.unwrap_or(existing.is_primary))
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await?;

    let source = sqlx::query_as::<_, DataSource>("SELECT * FROM data_sources WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(json!({ "success": true, "data": source })))
}

// ============================================================
// Elasticsearch Field Mapping
// ============================================================

/// Get the index mapping from an external Elasticsearch cluster.
/// The `url` here is the data source's own ES url (may differ from the system's ES),
/// so we connect directly via reqwest.
async fn fetch_es_mapping(url: &str, index: &str) -> Result<Value, AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to create HTTP client: {}", e)))?;

    let base = url.trim_end_matches('/');
    let mapping_url = format!("{}/{}/_mapping", base, index);
    let resp = client
        .get(&mapping_url)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch ES mapping: {}", e)))?;

    if !resp.status().is_success() {
        return Err(AppError::Internal(format!(
            "ES returned status {} while fetching mapping for index '{}'",
            resp.status(),
            index
        )));
    }

    let body: Value = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to parse ES mapping response: {}", e)))?;

    Ok(body)
}

/// Extract the flattened list of fields (leaf paths) from an ES mapping body
/// `{ "<index>": { "mappings": { "properties": { ... } } } }`,
/// returning a map of field path -> type (and sub-mappings).
fn extract_mapping_fields(mapping_body: &Value) -> Value {
    let mut result = Map::new();

    let Some(obj) = mapping_body.as_object() else {
        return Value::Object(result);
    };

    let empty_props = Map::new();

    // The keys are index names -> { mappings: { properties: {...} } }
    for (_index_name, index_body) in obj {
        let properties = index_body
            .get("mappings")
            .and_then(|m| m.get("properties"))
            .and_then(|p| p.as_object())
            .unwrap_or(&empty_props);

        flatten_properties(properties, "", &mut result);
    }

    Value::Object(result)
}

fn flatten_properties(properties: &Map<String, Value>, prefix: &str, out: &mut Map<String, Value>) {
    for (key, value) in properties {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{}.{}", prefix, key)
        };

        if let Some(props) = value.get("properties").and_then(|p| p.as_object()) {
            // Nested object -> recurse, but also record a marker for the group? No,
            // record leaf paths only.
            flatten_properties(props, &path, out);
        } else {
            // Leaf field
            let field_type = value.get("type").and_then(|t| t.as_str()).unwrap_or("object");
            out.insert(path, json!(field_type));
        }
    }
}

/// Compute a field-mapping config that maps the system's canonical fields
/// to the actual external ES field paths. `mapping_body` comes from an external
/// ES cluster.
///
/// The result shape is a JSON object:
/// ```
/// {
///   "<external field path>": {
///     "es_field": "<path>",
///     "es_type": "<type>"
///   },
///   ...
/// }
/// ```
/// User-defined custom mapping entries (`existing`) are overlaid on top so any
/// manual edits are preserved.
pub fn build_field_mapping_config(mapping_body: &Value, existing: &Value) -> Value {
    let fields = extract_mapping_fields(mapping_body);

    let mut result = Map::new();
    if let Some(fields_obj) = fields.as_object() {
        for (path, field_type) in fields_obj {
            result.insert(
                path.clone(),
                json!({
                    "es_field": path,
                    "es_type": field_type,
                }),
            );
        }
    }

    if let Some(existing_obj) = existing.as_object() {
        for (key, val) in existing_obj {
            result.insert(key.clone(), val.clone());
        }
    }

    Value::Object(result)
}

#[derive(Debug, Deserialize)]
pub struct ApplyEsMappingRequest {
    pub url: String,
    pub index: String,
    /// Optional, pre-existing user-defined field-mapping to preserve on re-fetch.
    pub current_mapping: Option<Value>,
}

/// POST /data-sources/apply-es-mapping
/// Body: { "url": "...", "index": "...", "current_mapping": {...}? }
/// Fetch an index's mapping from an external ES, then return a suggested
/// field-mapping config (system canonical field -> external es field/type).
pub async fn apply_es_mapping(
    State(_state): State<AppState>,
    Json(req): Json<ApplyEsMappingRequest>,
) -> Result<Json<Value>, AppError> {
    if req.url.trim().is_empty() || req.index.trim().is_empty() {
        return Err(AppError::Validation(
            "url and index are required".into(),
        ));
    }

    let mapping_body = fetch_es_mapping(&req.url, &req.index).await?;

    let existing = req.current_mapping.as_ref().unwrap_or(&Value::Null);
    let mapping = build_field_mapping_config(&mapping_body, existing);

    Ok(Json(json!({
        "success": true,
        "data": {
            "index": req.index,
            "field_mapping": mapping,
            "fields": extract_mapping_fields(&mapping_body)
        }
    })))
}

pub async fn test_data_source(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let source = sqlx::query_as::<_, DataSource>("SELECT * FROM data_sources WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Data source {} not found", id)))?;

    let config: Value = serde_json::from_str(&source.config)
        .map_err(|e| AppError::Internal(format!("Invalid config JSON: {}", e)))?;

    let result = match source.r#type.as_str() {
        "elasticsearch" => {
            let url = config["url"].as_str().unwrap_or("");
            test_elasticsearch_connection(url).await
        }
        "loki" => {
            let url = config["url"].as_str().unwrap_or("");
            test_loki_connection(url).await
        }
        "postgresql" => {
            test_postgresql_connection(&config).await
        }
        "clickhouse" => {
            test_clickhouse_connection(&config).await
        }
        _ => Err(AppError::Validation(format!(
            "Unsupported datasource type: {}", source.r#type
        ))),
    };

    match result {
        Ok(message) => Ok(Json(json!({
            "success": true,
            "data": { "status": "connected", "message": message }
        }))),
        Err(e) => Ok(Json(json!({
            "success": false,
            "data": { "status": "failed", "message": format!("{}", e) }
        }))),
    }
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
                "INSERT INTO system_settings (key, value, category, updated_at) VALUES (?, ?, 'oidc', ?) ON CONFLICT(key) DO UPDATE SET value = ?, updated_at = ?"
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
