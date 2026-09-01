use serde_json::Value;
use tracing::{error, info};

use aads_core::error::AppError;
use aads_core::state::ElasticSearchClientTrait;

use crate::ml_supervised::{FEATURES, FpFilterModel, FpLabel, FpPrediction};
use crate::stat::SecurityStat;
use crate::types::LogEntry;

/// 5.5 오탐 필터의 ES 연동 헬퍼.
///
/// - 라벨 수집: 분석가가 검출을 판정하면 `fp_labels` 인덱스에 학습 샘플을 적재한다
///   (false_positive/suppressed → 라벨 1, resolved → 라벨 0).
/// - 예측: 인덱스에 쌓인 라벨로 로지스틱 모델을 학습시켜 새 security_stat의
///   오탐 확률을 예측한다.
///
/// 모든 ES 호출은 best-effort로 동작해, ES 장애 시에도 기존 탐지/리포트 흐름을
/// 차단하지 않는다.
pub const FP_LABELS_INDEX: &str = "fp_labels";

/// 오탐으로 간주할 detection status 목록 (라벨 1).
pub const FP_STATUSES: &[&str] = &["false_positive", "suppressed"];

/// 진탐으로 간주할 detection status 목록 (라벨 0).
///
/// 분석가가 조사를 마치고 `resolved`로 처리하면 진양성으로 간주해 지도학습의
/// 반대 클래스(T)를 제공한다. 오탐 필터가 두 클래스 모두 학습해야 분리가 가능하다.
pub const TP_STATUSES: &[&str] = &["resolved"];

/// 주어진 detection status가 라벨 1(오탐)에 해당하면 1, 라벨 0(진탐)이면 0,
/// 해당 없으면 None을 반환한다.
pub fn label_for_status(status: &str) -> Option<u8> {
    if FP_STATUSES.contains(&status) {
        Some(1)
    } else if TP_STATUSES.contains(&status) {
        Some(0)
    } else {
        None
    }
}

/// 라벨 한 건을 `fp_labels` 인덱스에 적재한다. 실패해도 오류를 무시한다.
pub async fn collect_label(
    es: &dyn ElasticSearchClientTrait,
    label: &FpLabel,
) -> Result<(), AppError> {
    ensure_index(es).await?;
    let doc = serde_json::to_value(label).unwrap_or_default();
    let _ = es.index_document(FP_LABELS_INDEX, &label.label_id, doc).await?;
    info!("Collected fp label {} (label={})", label.label_id, label.label);
    Ok(())
}

/// `fp_labels` 인덱스에서 라벨을 모두 로드한다.
pub async fn load_labels(es: &dyn ElasticSearchClientTrait) -> Vec<FpLabel> {
    if !es.index_exists(FP_LABELS_INDEX).await.unwrap_or(false) {
        return Vec::new();
    }

    let query = serde_json::json!({
        "query": { "match_all": {} },
        "size": 10000
    });

    match es.search(FP_LABELS_INDEX, query).await {
        Ok(resp) => {
            let hits = resp["hits"]["hits"].as_array().cloned().unwrap_or_default();
            hits.into_iter()
                .filter_map(|hit| {
                    let src = hit.get("_source")?;
                    serde_json::from_value::<FpLabel>(src.clone()).ok()
                })
                .collect()
        }
        Err(e) => {
            error!("Failed to load fp labels from ES: {}", e);
            Vec::new()
        }
    }
}

/// 라벨로 모델을 학습한다. 샘플이 2개 미만이면 기본(사전) 모델을 반환한다.
pub async fn train_model(es: &dyn ElasticSearchClientTrait) -> FpFilterModel {
    let labels = load_labels(es).await;
    let mut model = FpFilterModel::default();
    if labels.len() < 2 {
        return model;
    }
    model.train(&labels, 0.05, 150);
    info!(
        "Trained supervised fp filter on {} samples (fp_prior={:.3})",
        model.train_samples, model.fp_prior
    );
    model
}

/// 여러 security_stat에 대한 오탐 예측을 일괄 생성한다.
pub async fn predict_batch(
    es: &dyn ElasticSearchClientTrait,
    stats: &[SecurityStat],
) -> Vec<FpPrediction> {
    let model = train_model(es).await;
    stats.iter().map(|s| model.predict(s)).collect()
}

/// `FP_LABELS_INDEX` 설명이 담긴 매핑을 전달받아 인덱스를 보장한다.
async fn ensure_index(es: &dyn ElasticSearchClientTrait) -> Result<(), AppError> {
    if !es.index_exists(FP_LABELS_INDEX).await? {
        let _ = es.create_index(FP_LABELS_INDEX, FpFilterModel::fp_labels_mapping()).await?;
    }
    Ok(())
}

/// `FpPrediction` 결과를 JSON 배열로 직렬화한다.
pub fn predictions_to_json(preds: &[FpPrediction]) -> Value {
    serde_json::to_value(preds).unwrap_or_else(|_| Value::Array(Vec::new()))
}

/// `fp_labels` 피처들을 편의상 노출 (외부에서 피처명 확인용).
pub fn feature_names() -> &'static [&'static str] {
    FEATURES
}

/// 검출의 매칭된 로그 목록으로부터 오탐/진탐 라벨 한 건을 만든다.
///
/// `stat::build_security_stat`와 동일한 집계 규칙을 따른다 (unique ips/paths,
/// 4xx/5xx, error_rate). 라벨값은 호출부가 결정한다 (false_positive -> 1).
pub fn build_fp_label(
    label_id: &str,
    detection_id: &str,
    rule_id: &str,
    label: u8,
    detected_at: &str,
    entries: &[LogEntry],
) -> Option<FpLabel> {
    if entries.is_empty() {
        return None;
    }

    let mut ips = std::collections::HashSet::new();
    let mut paths = std::collections::HashSet::new();
    let mut methods = std::collections::HashSet::new();
    let mut status4xx = 0usize;
    let mut status5xx = 0usize;

    for log in entries {
        if !log.client_ip.is_empty() {
            ips.insert(log.client_ip.clone());
        }
        if !log.path.is_empty() {
            paths.insert(log.path.clone());
        }
        if !log.method.is_empty() {
            methods.insert(log.method.clone());
        }
        match log.status_code {
            400..=499 => status4xx += 1,
            500..=599 => status5xx += 1,
            _ => {}
        }
    }

    let total = entries.len().max(1);
    Some(FpLabel {
        label_id: label_id.to_string(),
        detection_id: detection_id.to_string(),
        rule_id: rule_id.to_string(),
        label,
        matched_count: entries.len() as f64,
        unique_ips: ips.len() as f64,
        unique_paths: paths.len() as f64,
        unique_methods: methods.len() as f64,
        status_4xx: status4xx as f64,
        status_5xx: status5xx as f64,
        error_rate: (status4xx + status5xx) as f64 / total as f64,
        timestamp: detected_at.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_log(ip: &str, path: &str, method: &str, status: u16) -> LogEntry {
        LogEntry {
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            source: "web-server".to_string(),
            client_ip: ip.to_string(),
            method: method.to_string(),
            path: path.to_string(),
            query: Some(String::new()),
            status_code: status,
            response_size: 100,
            user_agent: Some("Mozilla".to_string()),
            user_id: None,
            response_time: Some(1.0),
            extra: None,
        }
    }

    #[test]
    fn builds_label_from_entries() {
        let entries = vec![
            sample_log("1.2.3.4", "/admin", "POST", 403),
            sample_log("1.2.3.4", "/admin", "POST", 403),
            sample_log("5.6.7.8", "/wp-login", "POST", 404),
        ];
        let label = build_fp_label("fp-x", "det-x", "rule-011", 1, "2026-01-01T00:00:00Z", &entries).unwrap();
        assert_eq!(label.matched_count, 3.0);
        assert_eq!(label.unique_ips, 2.0);
        assert_eq!(label.unique_paths, 2.0);
        assert_eq!(label.unique_methods, 1.0);
        assert_eq!(label.status_4xx, 3.0);
        assert_eq!(label.status_5xx, 0.0);
        assert_eq!(label.error_rate, 1.0);
        assert_eq!(label.label, 1);
    }

    #[test]
    fn empty_entries_returns_none() {
        assert!(build_fp_label("a", "b", "c", 1, "t", &[]).is_none());
    }

    #[test]
    fn label_serializes_roundtrip() {
        let label = build_fp_label("fp-x", "det-x", "rule-011", 1, "t", &[sample_log("1.1.1.1", "/a", "GET", 200)]).unwrap();
        let v = serde_json::to_value(&label).unwrap();
        let back: FpLabel = serde_json::from_value(v).unwrap();
        assert_eq!(back.label_id, label.label_id);
        assert_eq!(back.unique_ips, 1.0);
    }

    #[test]
    fn map_status_to_label() {
        assert_eq!(label_for_status("false_positive"), Some(1));
        assert_eq!(label_for_status("suppressed"), Some(1));
        assert_eq!(label_for_status("resolved"), Some(0));
        assert_eq!(label_for_status("open"), None);
        assert_eq!(label_for_status("acknowledged"), None);
    }
}
