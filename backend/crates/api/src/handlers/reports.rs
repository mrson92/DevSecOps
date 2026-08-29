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
    pub report_type: Option<String>,
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
    let report_type = params.report_type.as_deref();

    let generator = ReportGenerator::new(state.db.clone());
    let (reports, total) = generator.list_reports(page, size, report_type).await?;

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

pub async fn delete_report(
    State(state): State<AppState>,
    Path(id): axum::extract::Path<String>,
) -> Result<Json<Value>, AppError> {
    let generator = ReportGenerator::new(state.db.clone());
    let deleted = generator.delete_report(&id).await?;

    if !deleted {
        return Err(AppError::NotFound(format!("Report {} not found", id)));
    }

    Ok(Json(json!({
        "success": true,
        "data": { "id": id, "deleted": true }
    })))
}

pub async fn generate_report(
    State(state): State<AppState>,
    Json(req): Json<GenerateReportRequest>,
) -> Result<Json<Value>, AppError> {
    let generator = ReportGenerator::new(state.db.clone());

    let date = req.date.as_deref()
        .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());

    // Determine the period start marker so we can detect duplicates
    let period_marker: String = match req.report_type.as_str() {
        "daily" => {
            let day = date.unwrap_or_else(|| chrono::Utc::now().date_naive());
            format!("{}T00:00:00", day)
        }
        "weekly" => {
            let start = date.unwrap_or_else(|| chrono::Utc::now().date_naive());
            format!("{}T00:00:00", start)
        }
        "monthly" => {
            let (year, month) = match date {
                Some(d) => (d.year(), d.month()),
                None => {
                    let now = chrono::Utc::now();
                    (now.year(), now.month())
                }
            };
            let start_date = chrono::NaiveDate::from_ymd_opt(year, month, 1)
                .ok_or_else(|| AppError::Validation(format!("Invalid month for report: {}-{:02}", year, month)))?;
            format!("{}T00:00:00", start_date)
        }
        _ => return Err(AppError::Validation(format!("Invalid report type: {}", req.report_type))),
    };

    // Idempotency: if a report for this period already exists, return it.
    if let Some(existing) = generator.find_by_period(&req.report_type, &period_marker).await? {
        return Ok(Json(json!({
            "success": true,
            "data": existing,
            "duplicate": true
        })));
    }

    let report = match req.report_type.as_str() {
        "daily" => {
            let day = date.unwrap_or_else(|| chrono::Utc::now().date_naive());
            generator.generate_daily_report(day).await?
        }
        "weekly" => {
            let start = date.unwrap_or_else(|| chrono::Utc::now().date_naive());
            generator.generate_weekly_report(start).await?
        }
        "monthly" => {
            let (year, month) = match date {
                Some(d) => (d.year(), d.month()),
                None => {
                    let now = chrono::Utc::now();
                    (now.year(), now.month())
                }
            };
            generator.generate_monthly_report(year, month).await?
        }
        _ => unreachable!(),
    };

    Ok(Json(json!({
        "success": true,
        "data": report,
        "duplicate": false
    })))
}
