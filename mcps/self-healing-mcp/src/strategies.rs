//! Fix generation strategies for different error types

use crate::types::ErrorPattern;
use anyhow::Result;

/// Generate a suggested fix based on error pattern
pub fn generate_fix_strategy(pattern: &ErrorPattern) -> Result<String> {
    let error_type = pattern.error_type.as_str();
    let tool_name = pattern.tool_name.as_deref().unwrap_or("unknown");

    let strategy = match error_type {
        "Timeout" => generate_timeout_fix(tool_name, pattern),
        "ConnectionRefused" => generate_connection_refused_fix(tool_name, pattern),
        "ServerCrashed" => generate_server_crashed_fix(tool_name, pattern),
        "ToolError" if pattern.correlation_score > 0.7 => {
            // High correlation suggests specific issue
            generate_tool_error_fix(tool_name, pattern)
        }
        _ => generate_generic_fix(tool_name, pattern),
    };

    Ok(strategy)
}

/// Generate fix for timeout errors
fn generate_timeout_fix(tool_name: &str, pattern: &ErrorPattern) -> String {
    format!(
        r#"## Timeout Fix Strategy for {tool_name}

**Issue:** Tool timing out after standard threshold ({occurrences} occurrences)

**Recommended Actions:**
1. Increase timeout threshold by 50% (e.g., 60s → 90s)
2. Add retry logic with exponential backoff (3 attempts: 1s, 2s, 4s)
3. Monitor avg tool duration to establish baseline
4. Consider parallel execution if multiple calls detected

**Configuration Change:**
```toml
[tools.{tool_name}]
timeout_ms = 90000  # Increased from 60000
retry_attempts = 3
retry_backoff_ms = 1000
```

**Expected Impact:** 40-60% reduction in timeout errors
**Risk:** Low - only extends wait time
**Verification:** Monitor success rate over 7 days
"#,
        tool_name = tool_name,
        occurrences = pattern.occurrences
    )
}

/// Generate fix for connection refused errors
fn generate_connection_refused_fix(tool_name: &str, pattern: &ErrorPattern) -> String {
    format!(
        r#"## Connection Refused Fix for {tool_name}

**Issue:** MCP server not responding ({occurrences} occurrences)

**Recommended Actions:**
1. Add health check before tool calls (ping endpoint)
2. Auto-restart MCP server on connection failure
3. Implement retry with exponential backoff
4. Configure daemon watchdog for auto-recovery

**Health Check Implementation:**
```rust
// Before calling tool:
if !mcp_server_healthy("{tool_name}") {{
    restart_mcp_server("{tool_name}").await?;
    wait_for_ready("{tool_name}", Duration::from_secs(5)).await?;
}}
```

**Expected Impact:** 70-80% reduction in connection errors
**Risk:** Medium - server restarts may disrupt other operations
**Verification:** Monitor restart frequency and success rate
"#,
        tool_name = tool_name,
        occurrences = pattern.occurrences
    )
}

/// Generate fix for server crashed errors
fn generate_server_crashed_fix(tool_name: &str, pattern: &ErrorPattern) -> String {
    format!(
        r#"## Server Crashed Recovery for {tool_name}

**Issue:** MCP server crashing during execution ({occurrences} occurrences)

**Recommended Actions:**
1. Implement circuit breaker pattern (stop calling after N failures)
2. Analyze crash logs for root cause (memory leak, segfault, panic)
3. Add resource limits (memory, CPU) to prevent system impact
4. Auto-restart with backoff (1min, 5min, 15min)
5. Report diagnostics to inbox for manual review

**Circuit Breaker Config:**
```toml
[tools.{tool_name}.circuit_breaker]
failure_threshold = 3
timeout_duration_sec = 300
half_open_requests = 1
```

**Expected Impact:** 90% reduction in cascade failures
**Risk:** High - may require code changes to MCP server
**Verification:** Manual review of crash logs + success rate monitoring
"#,
        tool_name = tool_name,
        occurrences = pattern.occurrences
    )
}

/// Generate fix for tool-specific errors
fn generate_tool_error_fix(tool_name: &str, pattern: &ErrorPattern) -> String {
    // Attempt to classify common tool errors
    let affected_runs_sample = pattern
        .affected_runs
        .iter()
        .take(3)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        r#"## Tool Error Fix for {tool_name}

**Issue:** Recurring tool-level errors ({occurrences} occurrences, correlation: {correlation:.2})

**Detected Pattern:**
- First seen: {first_seen}
- Last seen: {last_seen}
- Affected runs: {affected_runs_sample}...

**Recommended Actions:**
1. Analyze error context from affected runs (common parameters, state)
2. Add input validation before tool call
3. Implement fallback strategy (e.g., glob search if direct path fails)
4. Update agent prompt to avoid error condition

**Common Error Patterns:**
- **File not found:** Add file_exists check, use glob pattern for fuzzy matching
- **Permission denied:** Update agent_permissions table, route to permitted agents
- **Invalid parameter:** Add schema validation, provide examples in prompt

**Expected Impact:** 50-70% reduction in tool errors
**Risk:** Low-Medium depending on changes
**Verification:** Test against historical data + monitor new runs
"#,
        tool_name = tool_name,
        occurrences = pattern.occurrences,
        correlation = pattern.correlation_score,
        first_seen = pattern.first_seen.format("%Y-%m-%d %H:%M"),
        last_seen = pattern.last_seen.format("%Y-%m-%d %H:%M"),
        affected_runs_sample = affected_runs_sample
    )
}

/// Generate generic fix for unknown error types
fn generate_generic_fix(tool_name: &str, pattern: &ErrorPattern) -> String {
    format!(
        r#"## Generic Fix Strategy for {tool_name}

**Issue:** Error type '{error_type}' ({occurrences} occurrences)

**Recommended Actions:**
1. Collect more diagnostic data from affected runs
2. Enable verbose logging for {tool_name}
3. Add error context tracking (parameters, state, timing)
4. Review error messages for commonalities
5. Consider manual intervention if pattern persists

**Next Steps:**
- Run: `analyze_run_health` on affected runs: {affected_runs_sample}...
- Check logs: `tail -f ~/.binks/logs/{tool_name}.log`
- Review code: Examine {tool_name} implementation for edge cases

**Expected Impact:** Unknown - requires further analysis
**Risk:** Low - diagnostic only
**Verification:** Gather data for 7 days before proposing specific fix
"#,
        tool_name = tool_name,
        error_type = pattern.error_type,
        occurrences = pattern.occurrences,
        affected_runs_sample = pattern
            .affected_runs
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// Estimate expected impact as a percentage
pub fn estimate_impact(error_type: &str, correlation_score: f64) -> String {
    match error_type {
        "Timeout" if correlation_score > 0.8 => "40-60% reduction in timeout errors".to_string(),
        "Timeout" => "20-40% reduction in timeout errors".to_string(),
        "ConnectionRefused" if correlation_score > 0.8 => {
            "70-80% reduction in connection errors".to_string()
        }
        "ConnectionRefused" => "50-70% reduction in connection errors".to_string(),
        "ServerCrashed" => "90% reduction in cascade failures (may not prevent all crashes)"
            .to_string(),
        "ToolError" if correlation_score > 0.7 => "50-70% reduction in tool errors".to_string(),
        "ToolError" => "30-50% reduction in tool errors".to_string(),
        _ => "Unknown impact - requires further analysis".to_string(),
    }
}

/// Determine priority based on error type and frequency
pub fn determine_priority(error_type: &str, occurrences: usize, correlation_score: f64) -> String {
    if error_type == "ServerCrashed" || occurrences > 20 {
        return "urgent".to_string();
    }

    if occurrences > 10 && correlation_score > 0.8 {
        return "high".to_string();
    }

    if occurrences > 5 && correlation_score > 0.6 {
        return "medium".to_string();
    }

    "low".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_generate_timeout_fix() {
        let pattern = ErrorPattern {
            id: "pattern-1".to_string(),
            error_type: "Timeout".to_string(),
            tool_name: Some("mcp__kubernetes__pods_list".to_string()),
            occurrences: 5,
            first_seen: Utc::now(),
            last_seen: Utc::now(),
            affected_runs: vec!["run-1".to_string(), "run-2".to_string()],
            correlation_score: 0.9,
            suggested_fix: None,
        };

        let fix = generate_fix_strategy(&pattern).unwrap();
        assert!(fix.contains("Timeout Fix Strategy"));
        assert!(fix.contains("mcp__kubernetes__pods_list"));
        assert!(fix.contains("timeout_ms = 90000"));
    }

    #[test]
    fn test_priority_determination() {
        assert_eq!(determine_priority("ServerCrashed", 5, 0.5), "urgent");
        assert_eq!(determine_priority("Timeout", 25, 0.5), "urgent");
        assert_eq!(determine_priority("ToolError", 15, 0.9), "high");
        assert_eq!(determine_priority("Timeout", 7, 0.7), "medium");
        assert_eq!(determine_priority("ToolError", 3, 0.4), "low");
    }

    #[test]
    fn test_impact_estimation() {
        let impact = estimate_impact("Timeout", 0.9);
        assert!(impact.contains("40-60%"));

        let impact = estimate_impact("ConnectionRefused", 0.85);
        assert!(impact.contains("70-80%"));

        let impact = estimate_impact("ServerCrashed", 0.5);
        assert!(impact.contains("90%"));
    }
}
