use std::collections::HashMap;

use serde_json::Value;

use crate::stat::SecurityStat;

/// 1차 분석: 경량 무감독 이상탐지 스코어링.
///
/// `security_stat`(Rule 검출로 적재된 집계 통계) 중 "평소 패턴과 정밀하게
/// 다른 예외적 행위"를 Robust z-score(중앙값/MAD 기반)로 점수화한다.
/// 라벨링된 정답 없이도 이상치를 선별해, 2차 LLM 분석에 보낼 고위험군
/// 우선순위를 정한다.
///
/// 각 통계를 독립 정규분포로 가정하는 대신 MAD(median absolute deviation)
/// 기반의 Robust z-score를 사용해 이상치(outlier)의 영향으로부터 강건하게
/// 만든다. 멀티 피처 점수는 개별 z-score의 L2 놈(norm)으로 결합한다.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ThreatScore {
    pub stat_id: String,
    pub rule_id: String,
    pub rule_name: String,
    pub severity: String,
    /// 전체 피처에 대한 결합 이상 점수 (0 이상; 높을수록 이상).
    pub anomaly_score: f64,
    /// 각 피처별 편차 점수 (진단용).
    pub feature_scores: HashMap<String, f64>,
    /// 0.0 ~ 1.0 정규화된 이상 가능성 (100 = 최고 이상).
    pub anomaly_percentile: f64,
    /// 단순 휴리스틱 위협 레벨 (low/medium/high/critical).
    pub threat_level: String,
    pub timestamp: String,
}

const MAD_THRESHOLD: f64 = 9.0;
const CRITICAL_Z: f64 = 8.0;
const HIGH_Z: f64 = 4.0;
const MEDIUM_Z: f64 = 2.0;

/// 기준이 될 피처 목록. 값이 커질수록 이상에 가깝다고 간주한다.
const FEATURES: &[&str] = &[
    "matched_count",
    "unique_ips",
    "unique_paths",
    "unique_methods",
    "status_4xx",
    "status_5xx",
    "error_rate",
];

/// 일괄 이상탐지: 입력 통계 집합을 기준 분포로 삼아 각각을 점수화한다.
///
/// 통계가 1건 이하이면 분포를 만들 수 없어 스코어링 불가 → 빈 결과 반환
/// (2차 분석은 이 경우 raw 통계만으로 진행).
pub fn score_security_stats(stats: &[SecurityStat]) -> Vec<ThreatScore> {
    if stats.len() < 2 {
        return Vec::new();
    }

    let features = extract_features(stats);

    stats
        .iter()
        .filter_map(|s| {
            let anomaly_score = robust_z_norm(&features, s)
                .map(|(score, per_feat)| (score, per_feat))?;

            if anomaly_score.0 <= f64::EPSILON {
                return None;
            }

            let percentile = anomaly_to_percentile(anomaly_score.0);
            let threat_level = level_for(anomaly_score.0);

            Some(ThreatScore {
                stat_id: s.stat_id.clone(),
                rule_id: s.rule_id.clone(),
                rule_name: s.rule_name.clone(),
                severity: s.severity.clone(),
                anomaly_score: anomaly_score.0,
                feature_scores: anomaly_score.1,
                anomaly_percentile: percentile,
                threat_level,
                timestamp: s.timestamp.clone(),
            })
        })
        .collect()
}

/// `SecurityStat` 하나를 LLM/상위 계층에 전달할 점수화 결과로 변환.
///
/// 단건에 대한 `ThreatScore`가 필요할 때 `score_security_stats` 없이 사용할 수
/// 있도록 헬퍼로 노출한다. (분포 대비가 아니라 절대 임계 기반 단순 변환)
pub fn to_threat_score(stat: &SecurityStat) -> Option<ThreatScore> {
    let mut feature_scores = HashMap::new();
    feature_scores.insert("matched_count".to_string(), stat.matched_count as f64);
    feature_scores.insert("unique_ips".to_string(), stat.unique_ips as f64);
    feature_scores.insert("unique_paths".to_string(), stat.unique_paths as f64);
    feature_scores.insert("unique_methods".to_string(), stat.unique_methods as f64);
    feature_scores.insert("status_4xx".to_string(), stat.status_4xx as f64);
    feature_scores.insert("status_5xx".to_string(), stat.status_5xx as f64);
    feature_scores.insert("error_rate".to_string(), stat.error_rate);

    let magnitude = feature_scores.values().fold(0.0, |acc, v| acc + v * v).sqrt();
    if magnitude <= f64::EPSILON {
        return None;
    }

    let percentile = anomaly_to_percentile(magnitude);
    Some(ThreatScore {
        stat_id: stat.stat_id.clone(),
        rule_id: stat.rule_id.clone(),
        rule_name: stat.rule_name.clone(),
        severity: stat.severity.clone(),
        anomaly_score: magnitude,
        feature_scores,
        anomaly_percentile: percentile,
        threat_level: level_for(magnitude),
        timestamp: stat.timestamp.clone(),
    })
}

/// 피처별 값 벡터 추출 (피처명 -> (중앙값, MAD 스케일)).
fn extract_features(stats: &[SecurityStat]) -> HashMap<String, FeatureStats> {
    let mut out = HashMap::new();
    for feat in FEATURES {
        let vals: Vec<f64> = stats.iter().map(|s| feature_value(s, feat)).collect();
        let med = median(&vals);
        let mad = median_abs_dev(&vals, med);
        // MAD가 0이면 스케일이 없으므로 1로 대체 (어느 쪽도 이상으로 안 잡히게).
        let scale = mad.max(1e-9);
        out.insert(feat.to_string(), FeatureStats { median: med, mad: scale });
    }
    out
}

struct FeatureStats {
    median: f64,
    mad: f64,
}

fn feature_value(s: &SecurityStat, feat: &str) -> f64 {
    match feat {
        "matched_count" => s.matched_count as f64,
        "unique_ips" => s.unique_ips as f64,
        "unique_paths" => s.unique_paths as f64,
        "unique_methods" => s.unique_methods as f64,
        "status_4xx" => s.status_4xx as f64,
        "status_5xx" => s.status_5xx as f64,
        "error_rate" => s.error_rate,
        _ => 0.0,
    }
}

/// Robust z-score: (x - median) / (1.4826 * MAD).
fn robust_z(dist: &HashMap<String, FeatureStats>, stat: &SecurityStat) -> HashMap<String, f64> {
    let mut out = HashMap::new();
    for feat in FEATURES {
        let x = feature_value(stat, feat);
        if let Some(st) = dist.get(*feat) {
            let z = (x - st.median) / (1.4826 * st.mad);
            out.insert(feat.to_string(), z.max(0.0));
        }
    }
    out
}

/// 각 피처 Robust z-score의 L2 놈과 개별 점수를 함께 반환.
fn robust_z_norm(
    dist: &HashMap<String, FeatureStats>,
    stat: &SecurityStat,
) -> Option<(f64, HashMap<String, f64>)> {
    let per_feat = robust_z(dist, stat);
    if per_feat.is_empty() {
        return None;
    }
    let sum_sq: f64 = per_feat.values().map(|v| v * v).sum();
    Some((sum_sq.sqrt(), per_feat))
}

fn median(vals: &[f64]) -> f64 {
    if vals.is_empty() {
        return 0.0;
    }
    let mut sorted = vals.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    if n % 2 == 0 {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    } else {
        sorted[n / 2]
    }
}

fn median_abs_dev(vals: &[f64], center: f64) -> f64 {
    let devs: Vec<f64> = vals.iter().map(|v| (v - center).abs()).collect();
    median(&devs)
}

/// 이상 점수(놈)를 0~1 이상 가능성으로 변환 (단조, 0 = 정상).
fn anomaly_to_percentile(score: f64) -> f64 {
    // score >= MAD_THRESHOLD 일 때 1.0 포화. 그 아래는 왜곡 없이 비례.
    (score / MAD_THRESHOLD).clamp(0.0, 1.0)
}

fn level_for(score: f64) -> String {
    if score >= CRITICAL_Z {
        "critical".to_string()
    } else if score >= HIGH_Z {
        "high".to_string()
    } else if score >= MEDIUM_Z {
        "medium".to_string()
    } else {
        "low".to_string()
    }
}

/// 디버깅용: 이상 점수 결과를 JSON 배열로 직렬화.
pub fn scores_to_json(scores: &[ThreatScore]) -> Value {
    serde_json::to_value(scores).unwrap_or_else(|_| Value::Array(Vec::new()))
}

// ----------------------------------------------------------------------------
// 테스트
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn stat(id: &str, count: u32, ips: usize, paths: usize, err_rate: f64) -> SecurityStat {
        SecurityStat {
            stat_id: id.to_string(),
            rule_id: "rule-011".to_string(),
            rule_name: "Test Rule".to_string(),
            severity: "medium".to_string(),
            mitre_tactics: vec![],
            mitre_techniques: vec![],
            window_sec: 60,
            matched_count: count,
            unique_ips: ips,
            unique_paths: paths,
            unique_methods: 1,
            status_4xx: 0,
            status_5xx: 0,
            error_rate: err_rate,
            top_ips: vec![],
            top_paths: vec![],
            samples: vec![],
            group_key: None,
            timestamp: "2024-01-01T10:00:00Z".to_string(),
        }
    }

    #[test]
    fn zero_magnitude_returns_none() {
        // 모든 피처가 0이면 점수 산출 불가 → None.
        let mut s = stat("s0", 0, 0, 0, 0.0);
        s.unique_methods = 0;
        assert!(to_threat_score(&s).is_none());
    }

    #[test]
    fn flags_outlier_among_baseline() {
        // 대부분 정상 분포, 한 건만 극단적으로 다름.
        let baseline: Vec<SecurityStat> = (0..20)
            .map(|i| stat(&format!("s{}", i), 5, 3, 3, 0.02))
            .collect();
        let mut stats = baseline;
        stats.push(stat("outlier", 500, 300, 250, 0.95));

        let scores = score_security_stats(&stats);
        let outlier = scores.iter().find(|s| s.stat_id == "outlier");

        assert!(outlier.is_some(), "outlier should be flagged");
        let o = outlier.unwrap();
        assert!(o.anomaly_score > 10.0);
        assert!(o.threat_level == "critical" || o.threat_level == "high");
        assert!(o.anomaly_percentile > 0.5);
    }

    #[test]
    fn normal_docs_not_scored_as_anomaly() {
        let stats: Vec<SecurityStat> = (0..10)
            .map(|i| stat(&format!("s{}", i), 5, 3, 3, 0.02))
            .collect();
        let scores = score_security_stats(&stats);
        // 전부 균등 → 이상 점수 0, 필터링됨.
        assert!(scores.is_empty());
    }

    #[test]
    fn requires_two_or_more_docs() {
        let single = vec![stat("s0", 10, 5, 4, 0.1)];
        assert!(score_security_stats(&single).is_empty());
    }

    #[test]
    fn single_stat_absolute_scale() {
        let s = stat("s0", 100, 50, 40, 0.8);
        let score = to_threat_score(&s).unwrap();
        assert!(score.anomaly_score > 0.0);
        assert!(score.anomaly_percentile <= 1.0);
    }
}
