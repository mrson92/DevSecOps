use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::Datelike;
use serde::Deserialize;
use serde_json::{json, Value};

use aads_core::error::AppError;
use aads_core::state::AppState;
use aads_engine::ReportGenerator;

#[derive(Debug, Deserialize)]
pub struct ReportQueryParams {
    pub page: Option<u32>,
    pub size: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct GenerateReportRequest {
    pub report_type: String,
    pub date: Option<String>,
}

pub async fn list_reports(
    State(state): State<AppState>,
    Query(params): Query<ReportQueryParams>,
) -> Result<Json<Value>, AppError> {
    let page = params.page.unwrap_or(1);
    let size = params.size.unwrap_or(20);

    let generator = ReportGenerator::new(state.db.clone());
    let (reports, total) = generator.list_reports(page, size).await?;

    Ok(Json(json!({
        "success": true,
        "data": reports,
        "meta": {
            "page": page,
            "size": size,
            "total": total
        }
    })))
}

pub async fn get_report(
    State(state): State<AppState>,
    Path(id): axum::extract::Path<String>,
) -> Result<Json<Value>, AppError> {
    let generator = ReportGenerator::new(state.db.clone());
    let report = generator.get_report(&id).await?
        .ok_or_else(|| AppError::NotFound(format!("Report {} not found", id)))?;

    Ok(Json(json!({
        "success": true,
        "data": report
    })))
}

pub async fn generate_report(
    State(state): State<AppState>,
    Json(req): Json<GenerateReportRequest>,
) -> Result<Json<Value>, AppError> {
    let generator = ReportGenerator::new(state.db.clone());

    let report = match req.report_type.as_str() {
        "daily" => {
            let date = req.date.as_deref()
                .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
                .unwrap_or_else(|| chrono::Utc::now().date_naive());
            generator.generate_daily_report(date).await?
        }
        "weekly" => {
            let date = req.date.as_deref()
                .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
                .unwrap_or_else(|| chrono::Utc::now().date_naive());
            generator.generate_weekly_report(date).await?
        }
        "monthly" => {
            let now = chrono::Utc::now();
            generator.generate_monthly_report(now.year(), now.month()).await?
        }
        _ => return Err(AppError::Validation(format!("Invalid report type: {}", req.report_type))),
    };

    Ok(Json(json!({
        "success": true,
        "data": report
    })))
}
