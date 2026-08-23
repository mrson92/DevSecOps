use axum::{extract::State, Json};
use serde_json::{json, Value};

use aads_core::error::AppError;
use aads_core::state::AppState;
use aads_engine::RuleEngine;

pub async fn run_rules(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let mut engine = RuleEngine::new(state.db.clone(), state.es.clone());
    
    let results = engine.run_all_rules().await
        .map_err(|e| AppError::RuleEngine(format!("Failed to run rules: {}", e)))?;

    let total_rules = results.len();
    let detected_rules = results.iter().filter(|r| r.detected).count();
    let total_detections: u32 = results.iter().map(|r| r.matched_count).sum();

    Ok(Json(json!({
        "success": true,
        "data": {
            "total_rules": total_rules,
            "detected_rules": detected_rules,
            "total_detections": total_detections,
            "results": results
        }
    })))
}

pub async fn run_single_rule(
    State(state): State<AppState>,
    axum::extract::Path(rule_id): axum::extract::Path<String>,
) -> Result<Json<Value>, AppError> {
    let mut engine = RuleEngine::new(state.db.clone(), state.es.clone());
    
    let rules = engine.load_rules().await
        .map_err(|e| AppError::RuleEngine(format!("Failed to load rules: {}", e)))?;

    let rule = rules.iter().find(|r| r.id == rule_id)
        .ok_or_else(|| AppError::NotFound(format!("Rule {} not found", rule_id)))?;

    let logs = engine.fetch_logs_from_es(rule).await
        .map_err(|e| AppError::RuleEngine(format!("Failed to fetch logs: {}", e)))?;

    let result = engine.execute_rule(rule, logs).await
        .map_err(|e| AppError::RuleEngine(format!("Failed to execute rule: {}", e)))?;

    if result.detected {
        engine.save_detection(&result).await
            .map_err(|e| AppError::RuleEngine(format!("Failed to save detection: {}", e)))?;
    }

    Ok(Json(json!({
        "success": true,
        "data": result
    })))
}
