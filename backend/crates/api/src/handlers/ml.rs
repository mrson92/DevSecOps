use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

use aads_core::state::AppState;
use aads_core::error::AppError;
use aads_engine::ml_supervised::FpFilterModel;
use aads_engine::fp_filter::{load_labels, predictions_to_json};

/// 오탐 필터(지도학습) 모델 상태를 조회한다.
///
/// - 학습된 샘플 수, 오탐 사전 확률, 학습 여부를 반환한다.
/// - 현재 `security_stat`의 오탐 예측 결과도 함께 반환해 UI/디버깅에 활용.
pub async fn get_fp_model(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let labels = load_labels(state.es.as_ref()).await;

    let mut model = FpFilterModel::default();
    if labels.len() >= 2 {
        model.train(&labels, 0.05, 150);
    }

    let fp_count = labels.iter().filter(|l| l.label == 1).count();
    let tp_count = labels.len() - fp_count;

    Ok(Json(json!({
        "success": true,
        "data": {
            "trained": model.train_samples >= 2,
            "train_samples": model.train_samples,
            "fp_prior": model.fp_prior,
            "labeled": {
                "total": labels.len(),
                "false_positive": fp_count,
                "true_positive": tp_count
            }
        }
    })))
}

/// 라벨링된 학습 데이터 목록을 반환한다.
pub async fn list_fp_labels(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let labels = load_labels(state.es.as_ref()).await;
    let data: Vec<Value> = labels
        .iter()
        .map(|l| json!({
            "label_id": l.label_id,
            "detection_id": l.detection_id,
            "rule_id": l.rule_id,
            "label": l.label,
            "matched_count": l.matched_count,
            "unique_ips": l.unique_ips,
            "error_rate": l.error_rate,
            "timestamp": l.timestamp
        }))
        .collect();

    Ok(Json(json!({
        "success": true,
        "data": data
    })))
}

/// 현재 security_stat 배치에 대한 오탐 예측을 반환한다.
///
/// 주로 agent 컨텍스트에서 사용하지만, 별도 확인용 엔드포인트로도 노출한다.
pub async fn predict_current_stats(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let index = "security_stat";
    if !state.es.index_exists(index).await.unwrap_or(false) {
        return Ok(Json(json!({ "success": true, "data": [] })));
    }

    let query = json!({ "sort": [{ "timestamp": "desc" }], "size": 50 });
    let resp = match state.es.search(index, query).await {
        Ok(r) => r,
        Err(e) => return Err(AppError::ElasticSearch(e.to_string())),
    };

    let hits = resp["hits"]["hits"].as_array().cloned().unwrap_or_default();
    let stats: Vec<_> = hits
        .into_iter()
        .filter_map(|hit| {
            let src = hit.get("_source")?;
            serde_json::from_value::<aads_engine::SecurityStat>(src.clone()).ok()
        })
        .collect();

    let model = aads_engine::fp_filter::train_model(state.es.as_ref()).await;
    let preds: Vec<_> = stats.iter().map(|s| model.predict(s)).collect();

    Ok(Json(json!({
        "success": true,
        "data": predictions_to_json(&preds)
    })))
}
