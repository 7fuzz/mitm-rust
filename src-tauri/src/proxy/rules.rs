use std::sync::Arc;
use regex::Regex;
use crate::db::InterceptRule;
use crate::state::{AppState, InterceptPhase};

pub async fn should_intercept(
    state: &Arc<AppState>,
    phase: InterceptPhase,
    method: &str,
    url: &str,
    host: &str,
    headers: &[(String, String)],
    request_headers: &[(String, String)],
    listener_label: &str,
) -> bool {
    let cfg = state.proxy_config.read().await;

    if !cfg.proxy_enabled || !cfg.intercept_enabled {
        return false;
    }

    match (cfg.intercept_mode.as_str(), phase) {
        ("request", InterceptPhase::Response) => return false,
        ("response", InterceptPhase::Request) => return false,
        _ => {}
    }

    if !state.source_matches(&cfg.intercept_source_scope, listener_label).await {
        return false;
    }

    let rules_guard = state.rules.read().await;
    let enabled_rules: Vec<&InterceptRule> = rules_guard.iter().filter(|r| r.is_enabled).collect();

    if enabled_rules.is_empty() {
        // Default behavior: if intercept is ON and no active rules exist, intercept all
        return true;
    }

    let path = crate::proxy::parse_path_from_url(url);
    let rule_matches = |rule: &InterceptRule| -> bool {
        rule_matches(rule, phase, method, url, host, &path, headers, request_headers)
    };

    // 1. Blacklist check ("pass" rules): If traffic matches ANY enabled "pass" rule, DO NOT INTERCEPT
    let pass_rules: Vec<&&InterceptRule> = enabled_rules.iter().filter(|r| r.action.eq_ignore_ascii_case("pass") || r.action.eq_ignore_ascii_case("allow") || r.action.eq_ignore_ascii_case("block")).collect();
    for rule in &pass_rules {
        if rule_matches(rule) {
            return false;
        }
    }

    // 2. Whitelist check ("intercept" rules):
    let intercept_rules: Vec<&&InterceptRule> = enabled_rules.iter().filter(|r| r.action.eq_ignore_ascii_case("intercept")).collect();

    if !intercept_rules.is_empty() {
        // If "intercept" rules exist, traffic MUST match AT LEAST ONE "intercept" rule
        for rule in &intercept_rules {
            if rule_matches(rule) {
                return true;
            }
        }
        // None of the whitelist rules matched -> pass through
        return false;
    }

    // If there are only "pass" rules and none matched above -> intercept remaining traffic
    true
}

/// Matches one rule against a flow. Responses are matched against the request that
/// produced them (method, URL, host, path), and header rules see both the request
/// headers and the response headers.
fn rule_matches(
    rule: &InterceptRule,
    phase: InterceptPhase,
    method: &str,
    url: &str,
    host: &str,
    path: &str,
    headers: &[(String, String)],
    request_headers: &[(String, String)],
) -> bool {
    let phase_match = match rule.target_phase.as_str() {
        "request" => phase == InterceptPhase::Request,
        "response" => phase == InterceptPhase::Response,
        _ => true,
    };
    if !phase_match {
        return false;
    }

    let header_text;
    let target_value = match rule.match_field.as_str() {
        "url" => url,
        "host" => host,
        "path" => path,
        "method" => method,
        "header" => {
            let mut lines: Vec<String> = headers.iter().map(|(k, v)| format!("{}: {}", k, v)).collect();
            if phase == InterceptPhase::Response {
                lines.extend(request_headers.iter().map(|(k, v)| format!("{}: {}", k, v)));
            }
            header_text = lines.join("\n");
            &header_text
        }
        _ => url,
    };

    match rule.operator.as_str() {
        "equals" => target_value.eq_ignore_ascii_case(&rule.match_value),
        "regex" => Regex::new(&rule.match_value).map(|re| re.is_match(target_value)).unwrap_or(false),
        _ => target_value.to_lowercase().contains(&rule.match_value.to_lowercase()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(phase: &str, field: &str, op: &str, value: &str) -> InterceptRule {
        InterceptRule {
            id: "r".into(),
            is_enabled: true,
            target_phase: phase.into(),
            match_field: field.into(),
            operator: op.into(),
            match_value: value.into(),
            order_index: 0,
            created_at_ms: 0,
            action: "intercept".into(),
        }
    }

    fn matches_response(r: &InterceptRule) -> bool {
        let url = "https://example.com/api/stuff?x=1";
        let res_headers = vec![("Content-Type".to_string(), "application/json".to_string())];
        let req_headers = vec![("Host".to_string(), "example.com".to_string())];
        let path = crate::proxy::parse_path_from_url(url);
        rule_matches(r, InterceptPhase::Response, "GET", url, "example.com", &path, &res_headers, &req_headers)
    }

    #[test]
    fn response_matches_original_request_url_host_and_path() {
        assert!(matches_response(&rule("both", "url", "contains", "https://example.com/api/stuff")));
        assert!(matches_response(&rule("response", "host", "equals", "example.com")));
        assert!(matches_response(&rule("both", "path", "regex", "^/api/stuff")));
        assert!(matches_response(&rule("both", "method", "equals", "get")));
    }

    #[test]
    fn response_header_rules_see_request_and_response_headers() {
        assert!(matches_response(&rule("both", "header", "contains", "host: example.com")));
        assert!(matches_response(&rule("both", "header", "contains", "application/json")));
    }

    #[test]
    fn request_only_rule_does_not_match_response() {
        assert!(!matches_response(&rule("request", "url", "contains", "example.com")));
    }

    #[test]
    fn path_is_not_the_full_url() {
        assert!(matches_response(&rule("both", "path", "equals", "/api/stuff?x=1")));
        assert!(!matches_response(&rule("both", "path", "contains", "example.com")));
    }
}
