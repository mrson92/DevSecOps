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
    StatusCodeGte(u16),
    StatusCodeLte(u16),
    ResponseSizeGte(u64),
    HasField(String),
    PathNotEmpty,
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

    /// Get the threshold from condition (e.g., >= 11 from count_threshold(..., 11))
    pub fn get_threshold(&self, condition: &str) -> u32 {
        if let Some(n) = condition.find("count_threshold(") {
            let rest = &condition[n + "count_threshold(".len()..];
            if let Some(end) = rest.find(')') {
                let inner = &rest[..end];
                if let Some((_, count_str)) = inner.split_once(", ") {
                    if let Ok(n) = count_str.parse::<u32>() {
                        return n;
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
    if condition.starts_with("count(filter(logs, log -> ") {
        if let Some(end_filter) = condition.find("))") {
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

fn find_matching_paren(s: &str, start: usize) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in s[start..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                if depth == 0 {
                    return Some(start + i);
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    None
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
            CompiledPattern::StatusCodeGte(min) => {
                log.status_code >= *min
            }
            CompiledPattern::StatusCodeLte(max) => {
                log.status_code <= *max
            }
            CompiledPattern::ResponseSizeGte(min) => {
                log.response_size >= *min
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
