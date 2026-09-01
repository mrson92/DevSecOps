use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::stat::SecurityStat;

/// 5.5 지도학습 기반 오탐(false positive) 필터링.
///
/// 1차 무감독 이상탐지(`ml::score_security_stats`)와 달리, 이 모듈은 분석가가
/// 수동으로 라벨링한 검출 기록(`fp_labels`)을 **학습 데이터**로 삼아 "이 검출이
/// 오탐일 확률"을 예측하는 지도학습 분류기다.
///
/// 외부 의존성(다른 사용자는 XGBoost/lightgbm C 바인딩을 원치 않음) 없이
/// 순수 Rust로 구현한다. 표준 **로지스틱 회귀** + 확률적 경사하강법(SGD)을
/// 사용하고, 피처는 표준화(standardization)하여 스케일 차이의 영향을 줄인다.
///
/// 특징 벡터는 `security_stat`의 수치 피처와 동일하게 구성하므로, 학습·예측
/// 모두 같은 `feature_vector()` 헬퍼를 공유한다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FpFilterModel {
    /// 표준화에 사용한 피처별 평균.
    pub mean: Vec<f64>,
    /// 표준화에 사용한 피처별 표준편차.
    pub std: Vec<f64>,
    /// 로지스틱 회귀 가중치 (표준화된 피처에 대한 계수).
    pub weights: Vec<f64>,
    /// 편향.
    pub bias: f64,
    /// 학습에 사용된 샘플 수.
    pub train_samples: usize,
    /// false positive 비율 (예측 확률 보정에 사용).
    pub fp_prior: f64,
}

/// 지도학습 오탐 필터의 예측 결과.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FpPrediction {
    pub stat_id: String,
    pub rule_id: String,
    /// 오탐으로 판정될 확률 (0.0 ~ 1.0).
    pub fp_probability: f64,
    /// true positive일 확률 = 1 - fp_probability.
    pub tp_probability: f64,
    /// 임계치(기본 0.5) 이상이면 오탐 후보로 판정.
    pub is_fp_candidate: bool,
    pub trained: bool,
}

/// 학습 데이터 한 건 (ES `fp_labels` 인덱스의 `_source` 형태와 일치).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FpLabel {
    pub label_id: String,
    pub detection_id: String,
    pub rule_id: String,
    /// 1 = false positive, 0 = true positive.
    pub label: u8,
    pub matched_count: f64,
    pub unique_ips: f64,
    pub unique_paths: f64,
    pub unique_methods: f64,
    pub status_4xx: f64,
    pub status_5xx: f64,
    pub error_rate: f64,
    pub timestamp: String,
}

/// 피처 이름 목록 (security_stat 수치 필드와 1:1 대응).
pub const FEATURES: &[&str] = &[
    "matched_count",
    "unique_ips",
    "unique_paths",
    "unique_methods",
    "status_4xx",
    "status_5xx",
    "error_rate",
];

impl Default for FpFilterModel {
    fn default() -> Self {
        Self {
            mean: vec![0.0; FEATURES.len()],
            std: vec![1.0; FEATURES.len()],
            weights: vec![0.0; FEATURES.len()],
            bias: 0.0,
            train_samples: 0,
            fp_prior: 0.5,
        }
    }
}

impl FpFilterModel {
    /// `SecurityStat` 하나를 특징 벡터로 변환한다.
    pub fn feature_vector(stat: &SecurityStat) -> Vec<f64> {
        vec![
            stat.matched_count as f64,
            stat.unique_ips as f64,
            stat.unique_paths as f64,
            stat.unique_methods as f64,
            stat.status_4xx as f64,
            stat.status_5xx as f64,
            stat.error_rate,
        ]
    }

    /// 라벨 데이터 한 건을 특징 벡터로 변환한다.
    pub fn feature_vector_from_label(label: &FpLabel) -> Vec<f64> {
        vec![
            label.matched_count,
            label.unique_ips,
            label.unique_paths,
            label.unique_methods,
            label.status_4xx,
            label.status_5xx,
            label.error_rate,
        ]
    }

    /// 배치 학습: 라벨 샘플 집합으로 로지스틱 회귀 파라미터를 SGD 갱신한다.
    ///
    /// - 먼저 피처를 표준화한다(mean/std 계산).
    /// - 기본 `lr=0.05`, `epochs=100`로 이진 교차 엔트로피(BCE)를 최소화한다.
    /// - 라벨이 너무 적거나 한쪽 클래스만 있으면 가중치는 거의 학습되지 않으므로,
    ///   여전히 `trained` 여부는 샘플 수로 판단한다.
    pub fn train(&mut self, labels: &[FpLabel], lr: f64, epochs: usize) {
        let n = labels.len();
        if n == 0 {
            self.train_samples = 0;
            return;
        }

        // 피처 행렬 구성.
        let rows: Vec<Vec<f64>> = labels
            .iter()
            .map(Self::feature_vector_from_label)
            .collect();

        // 표준화 파라미터.
        let dim = rows[0].len();
        let mut mean = vec![0.0; dim];
        for row in &rows {
            for (j, v) in row.iter().enumerate() {
                mean[j] += v;
            }
        }
        for m in mean.iter_mut() {
            *m /= n as f64;
        }

        let mut var = vec![0.0; dim];
        for row in &rows {
            for (j, v) in row.iter().enumerate() {
                let d = v - mean[j];
                var[j] += d * d;
            }
        }
        // 인구 표준편차 (n > 0 보장됨). 분모가 0이면 1로 대체해 스케일 보존.
        let mut std = vec![1.0; dim];
        for (j, v) in var.iter().enumerate() {
            let s = (v / n as f64).sqrt();
            std[j] = if s < 1e-9 { 1.0 } else { s };
        }

        let mut weights = vec![0.0; dim];
        let mut bias = 0.0;

        // SGD: 각 에폭마다 전체 샘플을 순회하며 확률적 갱신.
        for _ in 0..epochs {
            for (i, row) in rows.iter().enumerate() {
                let mut z = bias;
                for (j, v) in row.iter().enumerate() {
                    z += weights[j] * (v - mean[j]) / std[j];
                }
                let p = sigmoid(z);
                let y = labels[i].label as f64;
                let err = p - y;
                for j in 0..dim {
                    weights[j] -= lr * err * (row[j] - mean[j]) / std[j];
                }
                bias -= lr * err;
            }
        }

        let fp_count = labels.iter().filter(|l| l.label == 1).count();

        self.mean = mean;
        self.std = std;
        self.weights = weights;
        self.bias = bias;
        self.train_samples = n;
        self.fp_prior = fp_count as f64 / n as f64;
    }

    /// 오탐 확률을 예측한다. 모델이 학습되지 않았으면 사전 확률(fp_prior)을
    /// 그대로 반환하여(또는 0.5) best-effort로 동작하게 한다.
    pub fn predict(&self, stat: &SecurityStat) -> FpPrediction {
        if self.train_samples == 0 || self.weights.is_empty() {
            return FpPrediction {
                stat_id: stat.stat_id.clone(),
                rule_id: stat.rule_id.clone(),
                fp_probability: self.fp_prior,
                tp_probability: 1.0 - self.fp_prior,
                is_fp_candidate: false,
                trained: false,
            };
        }

        let features = Self::feature_vector(stat);
        let mut z = self.bias;
        for (j, v) in features.iter().enumerate() {
            let s = if self.std[j] < 1e-9 { 1.0 } else { self.std[j] };
            z += self.weights[j] * (v - self.mean[j]) / s;
        }
        let p = sigmoid(z);

        FpPrediction {
            stat_id: stat.stat_id.clone(),
            rule_id: stat.rule_id.clone(),
            fp_probability: p,
            tp_probability: 1.0 - p,
            is_fp_candidate: p >= 0.5,
            trained: true,
        }
    }

    /// `fp_labels` ES 인덱스 매핑.
    pub fn fp_labels_mapping() -> Value {
        serde_json::json!({
            "mappings": {
                "properties": {
                    "label_id": { "type": "keyword" },
                    "detection_id": { "type": "keyword" },
                    "rule_id": { "type": "keyword" },
                    "label": { "type": "integer" },
                    "matched_count": { "type": "integer" },
                    "unique_ips": { "type": "integer" },
                    "unique_paths": { "type": "integer" },
                    "unique_methods": { "type": "integer" },
                    "status_4xx": { "type": "integer" },
                    "status_5xx": { "type": "integer" },
                    "error_rate": { "type": "float" },
                    "timestamp": { "type": "date" }
                }
            }
        })
    }
}

fn sigmoid(z: f64) -> f64 {
    1.0 / (1.0 + (-z).exp())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_label(id: &str, error_rate: f64, label: u8) -> FpLabel {
        FpLabel {
            label_id: format!("lab-{}", id),
            detection_id: format!("det-{}", id),
            rule_id: "rule-011".to_string(),
            label,
            matched_count: 10.0,
            unique_ips: 4.0,
            unique_paths: 3.0,
            unique_methods: 2.0,
            status_4xx: 8.0,
            status_5xx: 1.0,
            error_rate,
            timestamp: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn make_stat(id: &str, error_rate: f64) -> SecurityStat {
        SecurityStat {
            stat_id: id.to_string(),
            rule_id: "rule-011".to_string(),
            rule_name: "Test".to_string(),
            severity: "medium".to_string(),
            mitre_tactics: vec![],
            mitre_techniques: vec![],
            window_sec: 60,
            matched_count: 10,
            unique_ips: 4,
            unique_paths: 3,
            unique_methods: 2,
            status_4xx: 8,
            status_5xx: 1,
            error_rate,
            top_ips: vec![],
            top_paths: vec![],
            samples: vec![],
            group_key: None,
            timestamp: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn trains_and_separates_fp() {
        // 오탐(고 error_rate)과 진탐(저 error_rate)을 학습시킨다.
        let mut labels = Vec::new();
        for i in 0..50 {
            labels.push(make_label(&format!("fp-{}", i), 0.9, 1));
            labels.push(make_label(&format!("tp-{}", i), 0.05, 0));
        }

        let mut model = FpFilterModel::default();
        model.train(&labels, 0.05, 200);

        // 학습 후 오탐 샘플은 높은 확률로, 진탐 샘플은 낮은 확률로 예측.
        let fp_pred = model.predict(&make_stat("new-fp", 0.95));
        let tp_pred = model.predict(&make_stat("new-tp", 0.02));

        assert!(fp_pred.fp_probability > 0.5, "fp prob={}", fp_pred.fp_probability);
        assert!(tp_pred.fp_probability < 0.5, "tp prob={}", tp_pred.fp_probability);
        assert!(fp_pred.is_fp_candidate);
        assert!(!tp_pred.is_fp_candidate);
        assert!(model.train_samples == 100);
    }

    #[test]
    fn untrained_model_returns_prior() {
        let model = FpFilterModel::default();
        let pred = model.predict(&make_stat("s", 0.5));
        assert!(!pred.trained);
        assert!(!pred.is_fp_candidate);
        assert!((pred.fp_probability - 0.5).abs() < 1e-9);
    }

    #[test]
    fn empty_training_keeps_defaults() {
        let mut model = FpFilterModel::default();
        model.train(&[], 0.05, 10);
        assert_eq!(model.train_samples, 0);
        assert!(model.weights.len() == FEATURES.len());
    }

    #[test]
    fn single_class_does_not_panic() {
        let labels: Vec<FpLabel> = (0..10).map(|i| make_label(&format!("a-{}", i), 0.5, 1)).collect();
        let mut model = FpFilterModel::default();
        model.train(&labels, 0.05, 10);
        let pred = model.predict(&make_stat("s", 0.5));
        assert!(pred.trained);
        assert_eq!(model.train_samples, 10);
    }

    #[test]
    fn feature_vector_roundtrip() {
        let stat = make_stat("s", 0.7);
        let v = FpFilterModel::feature_vector(&stat);
        assert_eq!(v.len(), FEATURES.len());
        assert_eq!(v[0], 10.0);
        assert_eq!(v[6], 0.7);
    }
}
