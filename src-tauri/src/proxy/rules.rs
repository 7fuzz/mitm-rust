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

    let rules_guard = state.rules.read().await;
    let enabled_rules: Vec<&InterceptRule> = rules_guard.iter().filter(|r| r.is_enabled).collect();

    if enabled_rules.is_empty() {
        // Default behavior: if intercept is ON and no active rules exist, intercept all
        return true;
    }

    let rule_matches = |rule: &InterceptRule| -> bool {
        let phase_match = match rule.target_phase.as_str() {
            "request" => phase == InterceptPhase::Request,
            "response" => phase == InterceptPhase::Response,
            "both" => true,
            _ => true,
        };

        if !phase_match {
            return false;
        }

        let target_value = match rule.match_field.as_str() {
            "url" => url,
            "host" => host,
            "method" => method,
            "header" => {
                &headers.iter().map(|(k, v)| format!("{}: {}", k, v)).collect::<Vec<_>>().join("\n")
            }
            _ => url,
        };

        match rule.operator.as_str() {
            "equals" => target_value.eq_ignore_ascii_case(&rule.match_value),
            "contains" => target_value.to_lowercase().contains(&rule.match_value.to_lowercase()),
            "regex" => {
                if let Ok(re) = Regex::new(&rule.match_value) {
                    re.is_match(target_value)
                } else {
                    false
                }
            }
            _ => target_value.to_lowercase().contains(&rule.match_value.to_lowercase()),
        }
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
