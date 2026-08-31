use std::collections::HashMap;

use serde_json::Value;

use aads_core::models::Rule;

use crate::types::{DetectionResult, LogEntry};

/// 최적화된 보안 위협 통계 (AI 검토용).
///
/// Rule 검출 결과의 raw 로그 전체가 아니라, 집계·요약된 피처와 대표 샘플만
/// 담아 ES `security_stat` 인덱스에 적재한다. 이를 통해 AI가 raw 로그를
/// 검토하지 않고 최적화된 데이터만으로 보안 위협을 판단할 수 있게 한다.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SecurityStat {
    pub stat_id: String,
    pub rule_id: String,
    pub rule_name: String,
    pub severity: String,
    pub mitre_tactics: Vec<String>,
    pub mitre_techniques: Vec<String>,

    pub window_sec: i32,
    pub matched_count: u32,

    pub unique_ips: usize,
    pub unique_paths: usize,
    pub unique_methods: usize,

    pub status_4xx: usize,
    pub status_5xx: usize,
    pub error_rate: f64,

    pub top_ips: Vec<(String, u32)>,
    pub top_paths: Vec<(String, u32)>,

    /// 대표 샘플 로그 (전체가 아닌 소수): AI가 실제 근거를 확인할 수 있도록.
    pub samples: Vec<Value>,

    pub group_key: Option<String>,
    pub timestamp: String,
}

const MAX_SAMPLES: usize = 5;
const MAX_TOP: usize = 5;

/// 하나의 검출 결과를 AI 검토용 최적화 통계로 변환한다.
pub fn build_security_stat(result: &DetectionResult, rule: &Rule) -> Option<SecurityStat> {
    if !result.detected || result.matched_entries.is_empty() {
        return None;
    }

    let mut ips: HashMap<String, u32> = HashMap::new();
    let mut paths: HashMap<String, u32> = HashMap::new();
    let mut methods: HashMap<String, u32> = HashMap::new();
    let mut status4xx = 0usize;
    let mut status5xx = 0usize;

    for log in &result.matched_entries {
        *ips.entry(normalize(log.client_ip.clone())).or_insert(0) += 1;
        *paths.entry(normalize(log.path.clone())).or_insert(0) += 1;
        *methods.entry(normalize(log.method.clone())).or_insert(0) += 1;
        match log.status_code {
            400..=499 => status4xx += 1,
            500..=599 => status5xx += 1,
            _ => {}
        }
    }

    let total = result.matched_entries.len().max(1);
    let unique_ips = ips.len();
    let unique_paths = paths.len();
    let unique_methods = methods.len();
    let top_ips = top_n(ips, MAX_TOP);
    let top_paths = top_n(paths, MAX_TOP);

    let samples = result
        .matched_entries
        .iter()
        .take(MAX_SAMPLES)
        .map(log_to_value)
        .collect();

    Some(SecurityStat {
        stat_id: uuid::Uuid::new_v4().to_string(),
        rule_id: rule.id.clone(),
        rule_name: result.rule_name.clone(),
        severity: result.severity.clone(),
        mitre_tactics: parse_str_array(rule.mitre_tactics.as_deref()),
        mitre_techniques: parse_str_array(rule.mitre_techniques.as_deref()),
        window_sec: rule.window_sec,
        matched_count: result.matched_count,
        unique_ips,
        unique_paths,
        unique_methods,
        status_4xx: status4xx,
        status_5xx: status5xx,
        error_rate: ((status4xx + status5xx) as f64 / total as f64),
        top_ips,
        top_paths,
        samples,
        group_key: result.group_key.clone(),
        timestamp: result.timestamp.clone(),
    })
}

/// rule 목록을 순회하며 검출 결과와 매칭하여 통계를 일괄 생성한다.
pub fn build_security_stats(
    results: &[DetectionResult],
    rules: &[Rule],
) -> Vec<SecurityStat> {
    let by_id: HashMap<&str, &Rule> = rules.iter().map(|r| (r.id.as_str(), r)).collect();

    results
        .iter()
        .filter_map(|res| {
            let rule = by_id.get(res.rule_id.as_str()).copied()?;
            build_security_stat(res, rule)
        })
        .collect()
}

fn normalize(s: String) -> String {
    if s.is_empty() { "<empty>".to_string() } else { s }
}

fn top_n(map: HashMap<String, u32>, n: usize) -> Vec<(String, u32)> {
    let mut items: Vec<(String, u32)> = map.into_iter().collect();
    items.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    items.truncate(n);
    items
}

fn log_to_value(log: &LogEntry) -> Value {
    serde_json::json!({
        "timestamp": log.timestamp,
        "source": log.source,
        "client_ip": log.client_ip,
        "method": log.method,
        "path": log.path,
        "query": log.query,
        "status_code": log.status_code,
        "response_size": log.response_size,
        "user_agent": log.user_agent,
        "user_id": log.user_id,
        "response_time": log.response_time,
    })
}

fn parse_str_array(raw: Option<&str>) -> Vec<String> {
    match raw {
        None => Vec::new(),
        Some(s) => serde_json::from_str::<Vec<String>>(s).unwrap_or_else(|_| {
            s.trim_matches(|c| c == '[' || c == ']' || c == '"' || c == ' ')
                .split(',')
                .map(|x| x.trim().trim_matches('"').to_string())
                .filter(|x| !x.is_empty())
                .collect()
        }),
    }
}

/// ES `security_stat` 인덱스용 인덱스 매핑.
pub fn security_stat_mapping() -> Value {
    serde_json::json!({
        "mappings": {
            "properties": {
                "stat_id": { "type": "keyword" },
                "rule_id": { "type": "keyword" },
                "rule_name": { "type": "text", "fields": { "keyword": { "type": "keyword" } } },
                "severity": { "type": "keyword" },
                "mitre_tactics": { "type": "keyword" },
                "mitre_techniques": { "type": "keyword" },
                "window_sec": { "type": "integer" },
                "matched_count": { "type": "integer" },
                "unique_ips": { "type": "integer" },
                "unique_paths": { "type": "integer" },
                "unique_methods": { "type": "integer" },
                "status_4xx": { "type": "integer" },
                "status_5xx": { "type": "integer" },
                "error_rate": { "type": "float" },
                "group_key": { "type": "keyword" },
                "timestamp": { "type": "date" }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_log(ip: &str, path: &str, method: &str, status: u16) -> LogEntry {
        LogEntry {
            timestamp: "2024-01-01T10:00:00Z".to_string(),
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

    fn sample_rule() -> Rule {
        Rule {
            id: "rule-011-command-injection".to_string(),
            name: "Command Injection Attempt".to_string(),
            description: Some(String::new()),
            severity: "critical".to_string(),
            enabled: true,
            rule_type: "pattern".to_string(),
            condition: String::new(),
            window_sec: 60,
            slide_sec: 10,
            group_by: "[\"network.client.ip\"]".to_string(),
            actions: String::new(),
            mitre_tactics: Some("[\"TA0002\"]".to_string()),
            mitre_techniques: Some("[\"T1059\"]".to_string()),
            references: None,
            tags: None,
            version: 1,
            parent_rule_id: None,
            created_at: String::new(),
            updated_at: String::new(),
            created_by: None,
            updated_by: None,
        }
    }

    #[test]
    fn aggregates_and_dedups() {
        let result = DetectionResult {
            rule_id: "rule-011-command-injection".to_string(),
            rule_name: "Command Injection Attempt".to_string(),
            severity: "critical".to_string(),
            detected: true,
            matched_count: 3,
            group_key: Some("1.2.3.4".to_string()),
            matched_entries: vec![
                sample_log("1.2.3.4", "/admin", "POST", 403),
                sample_log("1.2.3.4", "/admin", "POST", 403),
                sample_log("5.6.7.8", "/etc/passwd", "GET", 500),
            ],
            timestamp: "2024-01-01T10:00:00Z".to_string(),
        };

        let stat = build_security_stat(&result, &sample_rule()).unwrap();
        assert_eq!(stat.unique_ips, 2);
        assert_eq!(stat.unique_paths, 2);
        assert_eq!(stat.unique_methods, 2);
        assert_eq!(stat.status_4xx, 2);
        assert_eq!(stat.status_5xx, 1);
        assert_eq!(stat.mitre_tactics, vec!["TA0002"]);
        assert_eq!(stat.mitre_techniques, vec!["T1059"]);
        assert_eq!(stat.samples.len(), 3);
        assert!(stat.error_rate > 0.0 && stat.error_rate <= 1.0);
    }

    #[test]
    fn skips_undetected_or_empty() {
        let result = DetectionResult {
            rule_id: "rule-011-command-injection".to_string(),
            rule_name: "x".to_string(),
            severity: "critical".to_string(),
            detected: false,
            matched_count: 0,
            group_key: None,
            matched_entries: vec![],
            timestamp: "t".to_string(),
        };
        assert!(build_security_stat(&result, &sample_rule()).is_none());
    }

    #[test]
    fn tops_and_sample_capping() {
        let rule = sample_rule();
        let mut entries = Vec::new();
        for i in 0..10 {
            entries.push(sample_log(&format!("10.0.0.{}", i), "/a", "GET", 200));
        }
        let result = DetectionResult {
            rule_id: rule.id.clone(),
            rule_name: "x".to_string(),
            severity: "medium".to_string(),
            detected: true,
            matched_count: 10,
            group_key: None,
            matched_entries: entries,
            timestamp: "t".to_string(),
        };
        let stat = build_security_stat(&result, &rule).unwrap();
        assert_eq!(stat.unique_ips, 10);
        assert_eq!(stat.top_ips.len(), 5);
        assert_eq!(stat.samples.len(), 5);
    }

    #[test]
    fn parses_mitre_arrays() {
        assert_eq!(parse_str_array(Some("[\"TA0002\",\"TA0003\"]")), vec!["TA0002", "TA0003"]);
        assert_eq!(parse_str_array(Some("TA0002,TA0003")), vec!["TA0002", "TA0003"]);
        assert_eq!(parse_str_array(None), Vec::<String>::new());
    }
}
