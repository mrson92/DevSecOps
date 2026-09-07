use axum::extract::{Query, State};
use axum::Json;
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::{json, Value};

use aads_core::error::AppError;
use aads_core::state::AppState;
use aads_engine::engine::LogFilter;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ListLogsParams {
    pub datasource: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub q: Option<String>,
    pub source: Option<String>,
    pub client_ip: Option<String>,
    pub method: Option<String>,
    pub path: Option<String>,
    pub status_code: Option<i64>,
    pub user_id: Option<String>,
    pub user_agent: Option<String>,
    pub exclude_path: Option<String>,
    pub exclude_client_ip: Option<String>,
    pub exclude_method: Option<String>,
    pub exclude_source: Option<String>,
    pub exclude_status_code: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub order: Option<String>,
    pub interval: Option<u64>,
}

fn collect_excludes(p: &ListLogsParams) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut push = |key: &'static str, v: &Option<String>| {
        if let Some(s) = v.as_deref().filter(|s| !s.is_empty()) {
            out.push((key.to_string(), s.to_string()));
        }
    };
    push("path", &p.exclude_path);
    push("client_ip", &p.exclude_client_ip);
    push("method", &p.exclude_method);
    push("source", &p.exclude_source);
    push("status_code", &p.exclude_status_code);
    out
}

fn build_filter(p: &ListLogsParams, default_span_hours: i64) -> Result<LogFilter, AppError> {
    if let Some(code) = p.status_code {
        if !(0..=65535).contains(&code) {
            return Err(AppError::Validation(format!("Invalid status_code: {}", code)));
        }
    }

    let now = Utc::now();
    Ok(LogFilter {
        from: p.from.clone().filter(|s| !s.is_empty())
            .or_else(|| Some((now - Duration::hours(default_span_hours)).to_rfc3339())),
        to: p.to.clone().filter(|s| !s.is_empty())
            .or_else(|| Some(now.to_rfc3339())),
        q: p.q.clone().filter(|s| !s.is_empty()),
        source: p.source.clone().filter(|s| !s.is_empty()),
        client_ip: p.client_ip.clone().filter(|s| !s.is_empty()),
        method: p.method.clone().filter(|s| !s.is_empty()),
        path: p.path.clone().filter(|s| !s.is_empty()),
        status_code: p.status_code.map(|v| v as u16),
        user_id: p.user_id.clone().filter(|s| !s.is_empty()),
        user_agent: p.user_agent.clone().filter(|s| !s.is_empty()),
        excludes: collect_excludes(p),
        limit: p.limit.unwrap_or(100).clamp(1, 1000),
        offset: p.offset.unwrap_or(0),
        order: p.order.clone().filter(|s| !s.is_empty()).unwrap_or_else(|| "desc".to_string()),
    })
}

pub async fn search_logs(
    State(state): State<AppState>,
    Query(params): Query<ListLogsParams>,
) -> Result<Json<Value>, AppError> {
    let filter = build_filter(&params, 1)?;
    let engine = aads_engine::RuleEngine::new(state.db.clone(), state.es.clone());
    let (logs, total, meta) = engine.search_logs(&filter, params.datasource.as_deref()).await?;

    let has_more = filter.offset + logs.len() < total as usize;

    Ok(Json(json!({
        "success": true,
        "data": logs,
        "meta": {
            "datasource_id": meta.id,
            "datasource_name": meta.name,
            "datasource_type": meta.r#type,
            "count": logs.len(),
            "total": total,
            "has_more": has_more,
            "limit": filter.limit,
            "offset": filter.offset,
            "from": filter.from.as_deref(),
            "to": filter.to.as_deref(),
        }
    })))
}

pub async fn log_histogram(
    State(state): State<AppState>,
    Query(params): Query<ListLogsParams>,
) -> Result<Json<Value>, AppError> {
    let filter = build_filter(&params, 1)?;
    let interval = params.interval.unwrap_or(60).clamp(10, 86400);
    let engine = aads_engine::RuleEngine::new(state.db.clone(), state.es.clone());
    let (buckets, meta) = engine.histogram_logs(&filter, interval, params.datasource.as_deref()).await?;

    let data: Vec<Value> = buckets
        .into_iter()
        .map(|(ts, count)| json!({ "ts": ts, "count": count }))
        .collect();

    Ok(Json(json!({
        "success": true,
        "data": data,
        "meta": {
            "datasource_id": meta.id,
            "datasource_name": meta.name,
            "datasource_type": meta.r#type,
            "interval": interval,
        }
    })))
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct FieldValuesParams {
    pub datasource: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub field: String,
    pub q: Option<String>,
    pub size: Option<usize>,
}

pub async fn log_field_values(
    State(state): State<AppState>,
    Query(params): Query<FieldValuesParams>,
) -> Result<Json<Value>, AppError> {
    let field = params.field.trim().to_string();
    if field.is_empty() {
        return Err(AppError::Validation("field is required".into()));
    }
    let size = params.size.unwrap_or(10).clamp(1, 100);

    let now = Utc::now();
    let filter = LogFilter {
        from: params.from.clone().filter(|s| !s.is_empty())
            .or_else(|| Some((now - Duration::hours(1)).to_rfc3339())),
        to: params.to.clone().filter(|s| !s.is_empty())
            .or_else(|| Some(now.to_rfc3339())),
        ..Default::default()
    };

    let engine = aads_engine::RuleEngine::new(state.db.clone(), state.es.clone());
    let (values, meta) = engine
        .field_values(&filter, &field, params.q.as_deref(), size, params.datasource.as_deref())
        .await?;

    let data: Vec<Value> = values
        .into_iter()
        .map(|(value, count)| json!({ "value": value, "count": count }))
        .collect();

    Ok(Json(json!({
        "success": true,
        "data": data,
        "meta": {
            "datasource_id": meta.id,
            "datasource_name": meta.name,
            "datasource_type": meta.r#type,
            "field": field,
            "size": size,
        }
    })))
}

pub async fn log_fields(
    State(state): State<AppState>,
    Query(params): Query<ListLogsParams>,
) -> Result<Json<Value>, AppError> {
    let engine = aads_engine::RuleEngine::new(state.db.clone(), state.es.clone());
    let (fields, meta) = engine.list_fields(params.datasource.as_deref()).await?;

    Ok(Json(json!({
        "success": true,
        "data": fields,
        "meta": {
            "datasource_id": meta.id,
            "datasource_name": meta.name,
            "datasource_type": meta.r#type,
        }
    })))
}