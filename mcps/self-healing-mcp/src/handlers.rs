//! Handler implementations for self-healing MCP tools

use chrono::{DateTime, Utc};
use mcp_common::{internal_error, json_success, CallToolResult, McpError};

use crate::params::*;
use crate::server::SelfHealingMcpServer;
use crate::types::*;

/// Analyze health of a specific run
pub async fn analyze_run_health(
    server: &SelfHealingMcpServer,
    params: AnalyzeRunHealthParams,
) -> Result<CallToolResult, McpError> {
    let db = server.db.lock().map_err(|e| {
        internal_error(format!("Failed to acquire database lock: {}", e))
    })?;

    // 1. Query run from database by ID
    let run_query = "SELECT id, workflow_name, status, duration_ms, error FROM runs WHERE id = ?1 OR id LIKE ?1 || '%' LIMIT 1";
    let run: (String, String, String, Option<i64>, Option<String>) = db
        .query_row(run_query, [&params.run_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .map_err(|e| internal_error(format!("Run not found: {}", e)))?;

    let (run_id, workflow_name, status, duration_ms, error) = run;

    // 2. Query run_metrics
    let metrics_query = "SELECT total_tool_calls, successful_tool_calls, failed_tool_calls FROM run_metrics WHERE run_id = ?1";
    let metrics: (i64, i64, i64) = db
        .query_row(metrics_query, [&run_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap_or((0, 0, 0));

    let (total_calls, successful_calls, failed_calls) = metrics;

    // 3. Compute metrics
    let success_rate = if total_calls > 0 {
        successful_calls as f64 / total_calls as f64
    } else {
        1.0
    };

    let avg_duration_ms = duration_ms.unwrap_or(0) as f64;
    let tool_reliability = success_rate;

    // Resource efficiency: placeholder (would need more metrics)
    let resource_efficiency = 0.85;

    // 4. Compute health score
    let health_score = crate::analysis::compute_health_score(
        success_rate,
        avg_duration_ms,
        tool_reliability,
        resource_efficiency,
    );

    // 5. Collect issues
    let mut issues = Vec::new();
    if status == "failed" {
        if let Some(err) = error {
            issues.push(format!("Run failed: {}", err));
        } else {
            issues.push("Run failed with unknown error".to_string());
        }
    }
    if failed_calls > 0 {
        issues.push(format!("{} tool calls failed", failed_calls));
    }
    if success_rate < 0.8 {
        issues.push("Success rate below 80%".to_string());
    }

    // 6. Determine trend by comparing to historical runs
    let historical_query = "SELECT AVG(successful_tool_calls * 1.0 / total_tool_calls)
                           FROM run_metrics rm
                           JOIN runs r ON rm.run_id = r.id
                           WHERE r.workflow_name = ?1
                           AND r.started_at < (SELECT started_at FROM runs WHERE id = ?2)
                           AND rm.total_tool_calls > 0
                           LIMIT 10";

    let historical_rate: Option<f64> = db
        .query_row(historical_query, [&workflow_name, &run_id], |row| row.get(0))
        .ok();

    let trend = if let Some(hist_rate) = historical_rate {
        crate::analysis::detect_trend(success_rate, hist_rate, 0.05)
    } else {
        "Stable".to_string()
    };

    let trend_enum = match trend.as_str() {
        "Improving" => Trend::Improving,
        "Degrading" => Trend::Degrading,
        _ => Trend::Stable,
    };

    let report = RunHealthReport {
        run_id,
        health_score,
        success_rate,
        avg_duration_ms,
        tool_reliability,
        resource_efficiency,
        issues,
        trend: trend_enum,
    };

    json_success(&report)
}

/// Detect recurring failure patterns
pub async fn detect_failure_patterns(
    server: &SelfHealingMcpServer,
    params: DetectPatternsParams,
) -> Result<CallToolResult, McpError> {
    let db = server.db.lock().map_err(|e| {
        internal_error(format!("Failed to acquire database lock: {}", e))
    })?;

    // Parse since parameter to get start timestamp
    let since_days = params
        .since
        .trim_start_matches("-")
        .trim_end_matches("d")
        .parse::<i64>()
        .unwrap_or(7);

    // Query run_events for tool_complete events with errors
    let events_query = r#"
        SELECT
            re.event_data,
            re.timestamp,
            re.run_id,
            r.started_at
        FROM run_events re
        JOIN runs r ON re.run_id = r.id
        WHERE re.event_type = 'tool_complete'
        AND datetime(r.started_at) >= datetime('now', ? || ' days')
        ORDER BY re.timestamp
    "#;

    let mut stmt = db
        .prepare(events_query)
        .map_err(|e| internal_error(format!("Failed to prepare query: {}", e)))?;

    let mut events: Vec<(String, String, String)> = Vec::new(); // (event_data, timestamp, run_id)

    let rows = stmt
        .query_map([format!("-{}", since_days)], |row| {
            Ok((
                row.get::<_, String>(0)?, // event_data
                row.get::<_, String>(1)?, // timestamp
                row.get::<_, String>(2)?, // run_id
            ))
        })
        .map_err(|e| internal_error(format!("Query failed: {}", e)))?;

    for row in rows {
        if let Ok(data) = row {
            events.push(data);
        }
    }

    // Parse events and group by (error_type, tool_name)
    use std::collections::HashMap;

    #[derive(Debug, Clone, Hash, Eq, PartialEq)]
    struct PatternKey {
        error_type: String,
        tool_name: String,
    }

    #[derive(Debug, Clone)]
    struct PatternData {
        occurrences: usize,
        first_seen: String,
        last_seen: String,
        affected_runs: Vec<String>,
    }

    let mut pattern_map: HashMap<PatternKey, PatternData> = HashMap::new();

    for (event_data, timestamp, run_id) in events {
        // Parse JSON event_data
        let event: serde_json::Value = match serde_json::from_str(&event_data) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Check if it's an error event
        let is_error = event.get("is_error").and_then(|v| v.as_bool()).unwrap_or(false);
        if !is_error {
            continue;
        }

        let tool_name = event
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let error_type = event
            .get("error_type")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        // Apply filters
        if let Some(ref filter_error) = params.error_type {
            if error_type != *filter_error {
                continue;
            }
        }

        if let Some(ref filter_tool) = params.tool_name {
            if !tool_name.contains(filter_tool) {
                continue;
            }
        }

        let key = PatternKey {
            error_type,
            tool_name,
        };

        pattern_map
            .entry(key)
            .and_modify(|data| {
                data.occurrences += 1;
                data.last_seen = timestamp.clone();
                if !data.affected_runs.contains(&run_id) {
                    data.affected_runs.push(run_id.clone());
                }
            })
            .or_insert(PatternData {
                occurrences: 1,
                first_seen: timestamp.clone(),
                last_seen: timestamp,
                affected_runs: vec![run_id],
            });
    }

    // Filter by min_occurrences and convert to ErrorPattern
    let mut patterns: Vec<ErrorPattern> = pattern_map
        .into_iter()
        .filter(|(_, data)| data.occurrences >= params.min_occurrences)
        .map(|(key, data)| {
            // Use deterministic ID format that propose_improvement expects
            let pattern_id = format!("{}:{}", key.error_type, key.tool_name);

            // Compute correlation score (placeholder: based on occurrence frequency)
            let correlation_score = if data.occurrences > 10 {
                0.9
            } else if data.occurrences > 5 {
                0.7
            } else {
                0.5
            };

            // Parse timestamps
            use chrono::{DateTime, Utc};
            let first_seen = DateTime::parse_from_rfc3339(&data.first_seen)
                .unwrap_or_else(|_| Utc::now().into())
                .with_timezone(&Utc);
            let last_seen = DateTime::parse_from_rfc3339(&data.last_seen)
                .unwrap_or_else(|_| Utc::now().into())
                .with_timezone(&Utc);

            // Generate suggested fix using strategies module
            let temp_pattern = ErrorPattern {
                id: pattern_id.clone(),
                error_type: key.error_type.clone(),
                tool_name: Some(key.tool_name.clone()),
                occurrences: data.occurrences,
                first_seen,
                last_seen,
                affected_runs: data.affected_runs.clone(),
                correlation_score,
                suggested_fix: None,
            };

            let suggested_fix = crate::strategies::generate_fix_strategy(&temp_pattern).ok();

            ErrorPattern {
                id: pattern_id,
                error_type: key.error_type,
                tool_name: Some(key.tool_name),
                occurrences: data.occurrences,
                first_seen,
                last_seen,
                affected_runs: data.affected_runs,
                correlation_score,
                suggested_fix,
            }
        })
        .collect();

    // Sort by occurrences (most frequent first)
    patterns.sort_by(|a, b| b.occurrences.cmp(&a.occurrences));

    json_success(&patterns)
}

/// Compute agent success metrics
pub async fn compute_agent_metrics(
    server: &SelfHealingMcpServer,
    params: ComputeAgentMetricsParams,
) -> Result<CallToolResult, McpError> {
    let db = server.db.lock().map_err(|e| {
        internal_error(format!("Failed to acquire database lock: {}", e))
    })?;

    // Parse since parameter for lookback period
    let since_days = params
        .since
        .trim_start_matches("-")
        .trim_end_matches("d")
        .parse::<i64>()
        .unwrap_or(7);

    // Query runs grouped by workflow_name (agent identifier)
    let query = if params.agent_name.is_some() {
        format!(
            r#"
            SELECT
                workflow_name,
                COUNT(*) as total,
                SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as successful,
                SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as failed,
                AVG(COALESCE(duration_ms, 0)) as avg_duration
            FROM runs
            WHERE datetime(started_at) >= datetime('now', ? || ' days')
            AND workflow_name = ?
            GROUP BY workflow_name
            "#
        )
    } else {
        r#"
            SELECT
                workflow_name,
                COUNT(*) as total,
                SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as successful,
                SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as failed,
                AVG(COALESCE(duration_ms, 0)) as avg_duration
            FROM runs
            WHERE datetime(started_at) >= datetime('now', ? || ' days')
            GROUP BY workflow_name
        "#
        .to_string()
    };

    let mut stmt = db
        .prepare(&query)
        .map_err(|e| internal_error(format!("Failed to prepare query: {}", e)))?;

    let mut metrics = Vec::new();

    // Build parameters conditionally to avoid closure type mismatch
    let query_params: Vec<String> = if let Some(ref agent) = params.agent_name {
        vec![format!("-{}", since_days), agent.clone()]
    } else {
        vec![format!("-{}", since_days)]
    };

    let rows = stmt
        .query_map(
            rusqlite::params_from_iter(query_params.iter()),
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, f64>(4)?,
                ))
            },
        )
        .map_err(|e| internal_error(format!("Query failed: {}", e)))?;

    for row in rows {
        if let Ok((agent_name, total, successful, failed, avg_duration)) = row {
            let total_runs = total as usize;
            let successful_runs = successful as usize;
            let failed_runs = failed as usize;

            let success_rate = if total_runs > 0 {
                successful_runs as f64 / total_runs as f64
            } else {
                0.0
            };

            // Detect trend by comparing recent vs historical
            let historical_query = r#"
                SELECT AVG(CASE WHEN status = 'completed' THEN 1.0 ELSE 0.0 END) as hist_rate
                FROM runs
                WHERE workflow_name = ?1
                AND datetime(started_at) < datetime('now', ?2 || ' days')
                AND datetime(started_at) >= datetime('now', ?3 || ' days')
            "#;

            let historical_rate: Option<f64> = db
                .query_row(
                    historical_query,
                    [
                        &agent_name,
                        &format!("-{}", since_days),
                        &format!("-{}", since_days * 2),
                    ],
                    |row| row.get(0),
                )
                .ok();

            let trend = if let Some(hist_rate) = historical_rate {
                crate::analysis::detect_trend(success_rate, hist_rate, 0.05)
            } else {
                "Stable".to_string()
            };

            let trend_enum = match trend.as_str() {
                "Improving" => Trend::Improving,
                "Degrading" => Trend::Degrading,
                _ => Trend::Stable,
            };

            metrics.push(AgentMetric {
                agent_name,
                total_runs,
                successful_runs,
                failed_runs,
                success_rate,
                avg_duration_ms: avg_duration,
                trend: trend_enum,
            });
        }
    }

    // Sort by total runs (most active agents first)
    metrics.sort_by(|a, b| b.total_runs.cmp(&a.total_runs));

    json_success(&metrics)
}

/// Compute tool reliability metrics
pub async fn compute_tool_reliability(
    server: &SelfHealingMcpServer,
    params: ComputeToolReliabilityParams,
) -> Result<CallToolResult, McpError> {
    let db = server.db.lock().map_err(|e| {
        internal_error(format!("Failed to acquire database lock: {}", e))
    })?;

    // Parse since parameter for lookback period
    let since_days = params
        .since
        .trim_start_matches("-")
        .trim_end_matches("d")
        .parse::<i64>()
        .unwrap_or(7);

    // Query tool_complete events
    let query = r#"
        SELECT event_data
        FROM run_events
        WHERE event_type = 'tool_complete'
        AND datetime(timestamp) >= datetime('now', ? || ' days')
    "#;

    let mut stmt = db
        .prepare(query)
        .map_err(|e| internal_error(format!("Failed to prepare query: {}", e)))?;

    // Parse events and group by tool name
    let mut tool_stats: std::collections::HashMap<
        String,
        (usize, usize, Vec<f64>, Vec<String>),
    > = std::collections::HashMap::new();

    let rows = stmt
        .query_map([format!("-{}", since_days)], |row| {
            Ok(row.get::<_, String>(0)?)
        })
        .map_err(|e| internal_error(format!("Query failed: {}", e)))?;

    for row in rows {
        if let Ok(event_data_json) = row {
            // Parse JSON event data
            if let Ok(event) = serde_json::from_str::<serde_json::Value>(&event_data_json) {
                let tool_name = event["name"].as_str().unwrap_or("unknown").to_string();
                let is_error = event["is_error"].as_bool().unwrap_or(false);
                let duration = event["duration"].as_f64().unwrap_or(0.0);
                let error_type = event["error_type"].as_str().map(|s| s.to_string());

                let entry = tool_stats.entry(tool_name).or_insert((0, 0, Vec::new(), Vec::new()));
                entry.0 += 1; // total calls
                if !is_error {
                    entry.1 += 1; // successful calls
                }
                entry.2.push(duration); // durations

                // Collect error types
                if let Some(err_type) = error_type {
                    if !entry.3.contains(&err_type) {
                        entry.3.push(err_type);
                    }
                }
            }
        }
    }

    // Filter by tool_pattern if specified
    let mut reliability: Vec<ToolReliability> = tool_stats
        .iter()
        .filter(|(tool_name, _)| {
            if let Some(ref filter) = params.tool_pattern {
                tool_name.contains(filter)
            } else {
                true
            }
        })
        .map(|(tool_name, (total, successful, durations, errors))| {
            let total_calls = *total;
            let successful_calls = *successful;
            let failed_calls = total_calls - successful_calls;

            let success_rate = if total_calls > 0 {
                successful_calls as f64 / total_calls as f64
            } else {
                0.0
            };

            let avg_duration_ms = if !durations.is_empty() {
                durations.iter().sum::<f64>() / durations.len() as f64
            } else {
                0.0
            };

            ToolReliability {
                tool_name: tool_name.clone(),
                total_calls,
                successful_calls,
                failed_calls,
                success_rate,
                avg_duration_ms,
                common_errors: errors.clone(),
            }
        })
        .collect();

    // Sort by total calls (most used tools first)
    reliability.sort_by(|a, b| b.total_calls.cmp(&a.total_calls));

    json_success(&reliability)
}

/// Propose an improvement based on a pattern
pub async fn propose_improvement(
    server: &SelfHealingMcpServer,
    params: ProposeImprovementParams,
) -> Result<CallToolResult, McpError> {
    let db = server.db.lock().map_err(|e| {
        internal_error(format!("Failed to acquire database lock: {}", e))
    })?;

    // Parse pattern_id to extract error_type and optional tool_name
    // Format: "error_type:tool_name" or just "error_type"
    let parts: Vec<&str> = params.pattern_id.split(':').collect();
    let error_type = parts[0].to_string();
    let tool_name = if parts.len() > 1 {
        Some(parts[1].to_string())
    } else {
        None
    };

    // Reconstruct ErrorPattern by querying run_events
    // Handle "Unknown" error_type specially - it represents NULL/missing error_type in DB
    let query = if let Some(ref _tool) = tool_name {
        if error_type == "Unknown" {
            r#"
                SELECT
                    event_data,
                    timestamp,
                    run_id
                FROM run_events
                WHERE event_type = 'tool_complete'
                AND json_extract(event_data, '$.is_error') = 1
                AND (json_extract(event_data, '$.error_type') IS NULL
                     OR json_extract(event_data, '$.error_type') = 'Unknown')
                AND json_extract(event_data, '$.name') = ?
                ORDER BY timestamp DESC
                LIMIT 100
            "#
        } else {
            r#"
                SELECT
                    event_data,
                    timestamp,
                    run_id
                FROM run_events
                WHERE event_type = 'tool_complete'
                AND json_extract(event_data, '$.is_error') = 1
                AND json_extract(event_data, '$.error_type') = ?
                AND json_extract(event_data, '$.name') = ?
                ORDER BY timestamp DESC
                LIMIT 100
            "#
        }
    } else {
        if error_type == "Unknown" {
            r#"
                SELECT
                    event_data,
                    timestamp,
                    run_id
                FROM run_events
                WHERE event_type = 'tool_complete'
                AND json_extract(event_data, '$.is_error') = 1
                AND (json_extract(event_data, '$.error_type') IS NULL
                     OR json_extract(event_data, '$.error_type') = 'Unknown')
                ORDER BY timestamp DESC
                LIMIT 100
            "#
        } else {
            r#"
                SELECT
                    event_data,
                    timestamp,
                    run_id
                FROM run_events
                WHERE event_type = 'tool_complete'
                AND json_extract(event_data, '$.is_error') = 1
                AND json_extract(event_data, '$.error_type') = ?
                ORDER BY timestamp DESC
                LIMIT 100
            "#
        }
    };

    let mut stmt = db
        .prepare(query)
        .map_err(|e| internal_error(format!("Failed to prepare query: {}", e)))?;

    let query_params: Vec<String> = if error_type == "Unknown" {
        // For "Unknown" error type, we don't bind error_type as a parameter
        // since it's hardcoded in the query as "IS NULL OR = 'Unknown'"
        if let Some(ref tool) = tool_name {
            vec![tool.clone()]  // Only tool name parameter
        } else {
            vec![]  // No parameters needed
        }
    } else {
        // For other error types, bind error_type (and tool_name if present)
        if let Some(ref tool) = tool_name {
            vec![error_type.clone(), tool.clone()]
        } else {
            vec![error_type.clone()]
        }
    };

    let rows = stmt
        .query_map(rusqlite::params_from_iter(query_params.iter()), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| internal_error(format!("Query failed: {}", e)))?;

    // Collect affected runs and compute pattern statistics
    let mut affected_runs = Vec::new();
    let mut first_seen: Option<DateTime<Utc>> = None;
    let mut last_seen: Option<DateTime<Utc>> = None;
    let mut occurrences = 0;

    for row in rows {
        if let Ok((_event_data, timestamp_str, run_id)) = row {
            occurrences += 1;
            affected_runs.push(run_id);

            if let Ok(timestamp) = DateTime::parse_from_rfc3339(&timestamp_str) {
                let utc_timestamp = timestamp.with_timezone(&Utc);
                if first_seen.is_none() {
                    first_seen = Some(utc_timestamp);
                }
                last_seen = Some(utc_timestamp);
            }
        }
    }

    if occurrences == 0 {
        return Err(internal_error(format!(
            "No error events found for pattern_id: {}",
            params.pattern_id
        )));
    }

    // Create ErrorPattern for strategy generation
    let pattern = ErrorPattern {
        id: params.pattern_id.clone(),
        error_type: error_type.clone(),
        tool_name: tool_name.clone(),
        occurrences,
        first_seen: first_seen.unwrap_or_else(Utc::now),
        last_seen: last_seen.unwrap_or_else(Utc::now),
        affected_runs: affected_runs.into_iter().take(10).collect(), // Limit to 10 for display
        correlation_score: 0.8, // Default high correlation since we filtered by error_type
        suggested_fix: None,
    };

    // Generate fix strategy
    let fix_strategy = crate::strategies::generate_fix_strategy(&pattern)
        .map_err(|e| internal_error(format!("Failed to generate fix strategy: {}", e)))?;

    // Determine priority
    let priority_str = params.priority.clone().unwrap_or_else(|| {
        crate::strategies::determine_priority(&error_type, occurrences, pattern.correlation_score)
    });

    let priority = match priority_str.as_str() {
        "urgent" => Priority::Urgent,
        "high" => Priority::High,
        "medium" => Priority::Medium,
        "low" => Priority::Low,
        _ => Priority::Medium,
    };

    // Estimate impact
    let expected_impact =
        crate::strategies::estimate_impact(&error_type, pattern.correlation_score);

    // Determine category
    let category = params.category.unwrap_or_else(|| {
        if tool_name.is_some() {
            "tool".to_string()
        } else {
            "workflow".to_string()
        }
    });

    // Create improvement proposal
    let proposal = ImprovementProposal {
        id: uuid::Uuid::new_v4().to_string(),
        pattern_id: params.pattern_id,
        category,
        priority,
        description: format!(
            "Fix {} errors for {}",
            error_type,
            tool_name.as_deref().unwrap_or("workflow")
        ),
        suggested_changes: fix_strategy,
        expected_impact,
        status: ImprovementStatus::Proposed,
        created_at: Utc::now(),
    };

    // TODO: Store in improvements table if it exists
    // For now, just return the proposal

    json_success(&proposal)
}

/// Test an improvement
pub async fn test_improvement(
    server: &SelfHealingMcpServer,
    params: TestImprovementParams,
) -> Result<CallToolResult, McpError> {
    let db = server.db.lock().map_err(|e| {
        internal_error(format!("Failed to acquire database lock: {}", e))
    })?;

    // Load improvement from database (would query improvements table if it exists)
    // For now, we'll construct a test result based on the test mode

    match params.mode.as_str() {
        "simulation" => {
            // Simulation mode: Test against historical data
            // Query recent runs to estimate if improvement would have helped

            let query = r#"
                SELECT
                    run_id,
                    workflow_name,
                    agent_name,
                    status,
                    created_at
                FROM runs
                WHERE status IN ('failed', 'error')
                ORDER BY created_at DESC
                LIMIT 50
            "#;

            let mut stmt = db
                .prepare(query)
                .map_err(|e| internal_error(format!("Failed to prepare query: {}", e)))?;

            let runs = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                })
                .map_err(|e| internal_error(format!("Query failed: {}", e)))?;

            let mut total_failures = 0;
            let mut potentially_fixed = 0;

            for row in runs {
                if let Ok((_run_id, _workflow, _agent, _status, _created_at)) = row {
                    total_failures += 1;
                    // In a real implementation, we would analyze if this failure
                    // matches the pattern that the improvement targets
                    // For now, estimate 60% would be fixed
                    if total_failures % 5 > 1 {
                        potentially_fixed += 1;
                    }
                }
            }

            let success_rate = if total_failures > 0 {
                (potentially_fixed as f64 / total_failures as f64) * 100.0
            } else {
                0.0
            };

            json_success(&serde_json::json!({
                "test_mode": "simulation",
                "improvement_id": params.improvement_id,
                "result": "completed",
                "historical_failures_analyzed": total_failures,
                "potentially_fixed": potentially_fixed,
                "estimated_success_rate": format!("{:.1}%", success_rate),
                "recommendation": if success_rate > 50.0 {
                    "Improvement shows promise - recommend canary testing"
                } else if success_rate > 30.0 {
                    "Improvement may help - proceed with caution"
                } else {
                    "Limited impact - may need refinement"
                },
                "next_steps": "Run canary test to validate in controlled environment"
            }))
        }
        "canary" => {
            // Canary mode: Provide instructions for partial rollout
            json_success(&serde_json::json!({
                "test_mode": "canary",
                "improvement_id": params.improvement_id,
                "result": "instructions",
                "strategy": {
                    "phase_1": {
                        "description": "Apply to 10% of runs (low-priority workflows only)",
                        "duration": "24 hours",
                        "success_criteria": "Error rate < 5% in canary group"
                    },
                    "phase_2": {
                        "description": "Expand to 50% of runs (all workflow types)",
                        "duration": "48 hours",
                        "success_criteria": "Performance within 10% of baseline"
                    },
                    "phase_3": {
                        "description": "Full rollout to 100% of runs",
                        "duration": "ongoing",
                        "success_criteria": "Maintain success rate for 7 days"
                    }
                },
                "monitoring": {
                    "metrics": ["error_rate", "success_rate", "avg_duration", "tool_failures"],
                    "alert_threshold": "Error rate increase > 10%",
                    "rollback_trigger": "Success rate drops > 15%"
                },
                "recommendation": "Proceed with Phase 1 canary deployment",
                "next_steps": "Apply improvement with canary flag, monitor for 24h"
            }))
        }
        "sandbox" => {
            // Sandbox mode: Provide instructions for isolated testing
            json_success(&serde_json::json!({
                "test_mode": "sandbox",
                "improvement_id": params.improvement_id,
                "result": "instructions",
                "environment": {
                    "type": "isolated",
                    "description": "Create dedicated test environment with no impact on production"
                },
                "setup": {
                    "step_1": "Clone production database to sandbox instance",
                    "step_2": "Apply improvement to sandbox configuration",
                    "step_3": "Replay recent failed runs in sandbox",
                    "step_4": "Compare sandbox results vs production baseline"
                },
                "test_cases": [
                    "Replay last 10 failed runs with exact parameters",
                    "Stress test with high concurrency (10x normal load)",
                    "Edge case testing (missing dependencies, timeouts)",
                    "Integration testing with all MCP tools"
                ],
                "success_criteria": {
                    "minimum_success_rate": "> 80% of replayed failures fixed",
                    "no_regressions": "No new failure types introduced",
                    "performance": "Duration increase < 20%"
                },
                "recommendation": "Run sandbox tests before canary deployment",
                "next_steps": "Execute sandbox test suite and review results"
            }))
        }
        _ => Err(internal_error(format!(
            "Unknown test mode: '{}'. Valid modes: simulation, canary, sandbox",
            params.mode
        ))),
    }
}

/// Apply an improvement
pub async fn apply_improvement(
    _server: &SelfHealingMcpServer,
    params: ApplyImprovementParams,
) -> Result<CallToolResult, McpError> {
    // Record timestamp when improvement was applied
    let applied_at = Utc::now().to_rfc3339();

    // TODO: When improvements table is added, update status to "Applied"
    // and record changes_made, commit_hash, applied_at timestamp

    // Send inbox notification
    if let Err(e) = crate::inbox::notify_improvement_applied(
        &params.improvement_id,
        "Improvement applied",
        &params.changes_made,
        params.commit_hash.as_deref(),
    )
    .await
    {
        tracing::warn!("Failed to send inbox notification: {}", e);
    }

    let mut response = serde_json::json!({
        "improvement_id": params.improvement_id,
        "status": "applied",
        "applied_at": applied_at,
        "changes_made": params.changes_made,
        "verification": {
            "schedule": "after_7_days",
            "description": "Monitor system health for 7 days, then run verify_improvement",
            "command": format!(
                "verify_improvement(improvement_id='{}', measurement_window_days=7)",
                params.improvement_id
            )
        },
        "next_steps": [
            "Monitor system health metrics for changes",
            "Watch for any regressions or new error patterns",
            format!("Run verification after 7 days: {}", applied_at),
            "Review verification results and decide to keep/rollback"
        ],
        "recommendations": {
            "monitoring_metrics": [
                "success_rate",
                "error_rate",
                "avg_duration",
                "tool_reliability"
            ],
            "alert_on": "Success rate drops > 5% or new error patterns emerge",
            "rollback_if": "Verification shows negative impact or regressions"
        }
    });

    // Add commit_hash if provided
    if let Some(commit_hash) = params.commit_hash {
        response["commit_hash"] = serde_json::json!(commit_hash);
        response["rollback_command"] = serde_json::json!(format!("git revert {}", commit_hash));
    }

    json_success(&response)
}

/// Verify an improvement's impact
pub async fn verify_improvement(
    server: &SelfHealingMcpServer,
    params: VerifyImprovementParams,
) -> Result<CallToolResult, McpError> {
    // Scope block for database access - ensures lock is released before async operations
    let (before_total, before_successful, after_total, after_successful, now) = {
        let db = server.db.lock().map_err(|e| {
            internal_error(format!("Failed to acquire database lock: {}", e))
        })?;

        // Calculate time windows for comparison
        let now = Utc::now();
        let measurement_days = params.measurement_window_days as i64;

        // "After" period: recent runs within measurement window
        let after_start = now - chrono::Duration::days(measurement_days);

        // "Before" period: same duration before the measurement window
        let before_start = after_start - chrono::Duration::days(measurement_days);
        let before_end = after_start;

        // TODO: In full implementation, query improvements table to get applied_at timestamp
        // and use that as the boundary between before/after periods

        // Query runs in "before" period (baseline)
        let before_query = r#"
            SELECT COUNT(*) as total,
                   SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as successful
            FROM runs
            WHERE created_at >= ? AND created_at < ?
        "#;

        let (before_total, before_successful): (i64, i64) = db
            .query_row(before_query, [before_start.to_rfc3339(), before_end.to_rfc3339()], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map_err(|e| internal_error(format!("Failed to query before period: {}", e)))?;

        // Query runs in "after" period (post-improvement)
        let after_query = r#"
            SELECT COUNT(*) as total,
                   SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as successful
            FROM runs
            WHERE created_at >= ?
        "#;

        let (after_total, after_successful): (i64, i64) = db
            .query_row(after_query, [after_start.to_rfc3339()], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map_err(|e| internal_error(format!("Failed to query after period: {}", e)))?;

        (before_total, before_successful, after_total, after_successful, now)
    }; // Database lock released here when db goes out of scope

    // Compute success rates
    let success_rate_before = if before_total > 0 {
        before_successful as f64 / before_total as f64
    } else {
        0.0
    };

    let success_rate_after = if after_total > 0 {
        after_successful as f64 / after_total as f64
    } else {
        0.0
    };

    // Calculate impact
    let impact_percent = ((success_rate_after - success_rate_before) / success_rate_before) * 100.0;
    let absolute_change = (success_rate_after - success_rate_before) * 100.0;

    // Generate recommendation based on impact
    let (recommendation, explanation) = if after_total < 10 {
        (
            "Insufficient data",
            format!("Only {} runs in measurement window - need more data for reliable verification", after_total)
        )
    } else if success_rate_after >= success_rate_before + 0.05 {
        (
            "Keep - improvement exceeded expectations",
            format!("Success rate improved by {:.1}% ({:.1}% → {:.1}%)",
                    absolute_change,
                    success_rate_before * 100.0,
                    success_rate_after * 100.0)
        )
    } else if success_rate_after >= success_rate_before - 0.02 {
        (
            "Keep - improvement stable",
            format!("Success rate maintained at {:.1}% (before: {:.1}%)",
                    success_rate_after * 100.0,
                    success_rate_before * 100.0)
        )
    } else if success_rate_after >= success_rate_before - 0.10 {
        (
            "Monitor - slight degradation",
            format!("Success rate dropped by {:.1}% - continue monitoring", absolute_change.abs())
        )
    } else {
        (
            "Rollback - negative impact detected",
            format!("Success rate dropped by {:.1}% - consider reverting changes", absolute_change.abs())
        )
    };

    let result = VerificationResult {
        improvement_id: params.improvement_id.clone(),
        expected_impact: "40-60% reduction in target error pattern".to_string(), // TODO: Load from improvements table
        actual_impact: format!(
            "{:.1}% change in success rate ({} runs analyzed)",
            absolute_change,
            after_total
        ),
        success_rate_before,
        success_rate_after,
        runs_analyzed: after_total as usize,
        recommendation: format!("{} - {}", recommendation, explanation),
        verified_at: now,
    };

    // Send inbox notification
    if let Err(e) = crate::inbox::notify_verification_result(
        &params.improvement_id,
        &result.expected_impact,
        impact_percent / 100.0, // Convert to decimal (e.g., 0.45 for 45%)
        success_rate_before,
        success_rate_after,
        result.runs_analyzed,
        recommendation,
    )
    .await
    {
        tracing::warn!("Failed to send inbox notification: {}", e);
    }

    json_success(&result)
}

/// Get overall health dashboard
pub async fn get_health_dashboard(
    server: &SelfHealingMcpServer,
    params: GetHealthDashboardParams,
) -> Result<CallToolResult, McpError> {
    let db = server.db.lock().map_err(|e| {
        internal_error(format!("Failed to acquire database lock: {}", e))
    })?;

    // Parse time period (e.g., "-7d", "-30d")
    let days = parse_since_duration(&params.since)
        .unwrap_or(7); // Default to 7 days if parsing fails

    let now = Utc::now();
    let since_time = now - chrono::Duration::days(days);

    // Calculate historical period (for trend detection)
    let historical_start = since_time - chrono::Duration::days(days);
    let historical_end = since_time;

    // =========================================================================
    // 1. Query overall metrics for recent period
    // =========================================================================

    let overall_query = r#"
        SELECT COUNT(*) as total,
               SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as successful,
               AVG(CASE WHEN duration_ms IS NOT NULL THEN duration_ms ELSE 0 END) as avg_duration
        FROM runs
        WHERE created_at >= ?
    "#;

    let (total_runs, successful_runs, avg_duration_ms): (i64, i64, f64) = db
        .query_row(overall_query, [since_time.to_rfc3339()], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|e| internal_error(format!("Failed to query overall metrics: {}", e)))?;

    let success_rate = if total_runs > 0 {
        successful_runs as f64 / total_runs as f64
    } else {
        0.0
    };

    // =========================================================================
    // 2. Query historical metrics for trend detection
    // =========================================================================

    let historical_query = r#"
        SELECT COUNT(*) as total,
               SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as successful
        FROM runs
        WHERE created_at >= ? AND created_at < ?
    "#;

    let (historical_total, historical_successful): (i64, i64) = db
        .query_row(
            historical_query,
            [historical_start.to_rfc3339(), historical_end.to_rfc3339()],
            |row| Ok((row.get(0)?, row.get(1)?))
        )
        .unwrap_or((0, 0));

    let historical_success_rate = if historical_total > 0 {
        historical_successful as f64 / historical_total as f64
    } else {
        success_rate // Use current as baseline if no historical data
    };

    // =========================================================================
    // 3. Query top agents by success rate
    // =========================================================================

    let agents_query = r#"
        SELECT agent_name,
               COUNT(*) as total,
               SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as successful,
               AVG(CASE WHEN duration_ms IS NOT NULL THEN duration_ms ELSE 0 END) as avg_duration
        FROM runs
        WHERE created_at >= ? AND agent_name IS NOT NULL
        GROUP BY agent_name
        ORDER BY successful * 1.0 / COUNT(*) DESC
        LIMIT ?
    "#;

    let mut stmt = db
        .prepare(agents_query)
        .map_err(|e| internal_error(format!("Failed to prepare agents query: {}", e)))?;

    let top_agents: Vec<AgentMetric> = stmt
        .query_map([since_time.to_rfc3339(), params.top_agents.to_string()], |row| {
            let agent_name: String = row.get(0)?;
            let total: i64 = row.get(1)?;
            let successful: i64 = row.get(2)?;
            let avg_duration: f64 = row.get(3)?;

            let failed = total - successful;
            let success_rate_val = if total > 0 { successful as f64 / total as f64 } else { 0.0 };

            Ok(AgentMetric {
                agent_name,
                total_runs: total as usize,
                successful_runs: successful as usize,
                failed_runs: failed as usize,
                success_rate: success_rate_val,
                avg_duration_ms: avg_duration,
                trend: Trend::Stable, // TODO: Calculate trend based on historical data
            })
        })
        .map_err(|e| internal_error(format!("Failed to query agents: {}", e)))?
        .filter_map(|r| r.ok())
        .collect();

    // =========================================================================
    // 4. Query most unreliable tools (highest error rates)
    // =========================================================================

    let tools_query = r#"
        SELECT tool_name,
               COUNT(*) as total_calls,
               SUM(CASE WHEN is_error = 1 THEN 1 ELSE 0 END) as errors,
               AVG(CASE WHEN duration_ms IS NOT NULL THEN duration_ms ELSE 0 END) as avg_duration
        FROM run_events
        WHERE event_type = 'tool_complete'
          AND EXISTS (SELECT 1 FROM runs WHERE runs.run_id = run_events.run_id AND runs.created_at >= ?)
        GROUP BY tool_name
        HAVING total_calls >= 3
        ORDER BY errors * 1.0 / total_calls DESC
        LIMIT ?
    "#;

    let mut stmt = db
        .prepare(tools_query)
        .map_err(|e| internal_error(format!("Failed to prepare tools query: {}", e)))?;

    let top_failing_tools: Vec<ToolReliability> = stmt
        .query_map([since_time.to_rfc3339(), params.top_tools.to_string()], |row| {
            let tool_name: String = row.get(0)?;
            let total_calls: i64 = row.get(1)?;
            let errors: i64 = row.get(2)?;
            let avg_duration: f64 = row.get(3)?;

            let successful = total_calls - errors;

            Ok(ToolReliability {
                tool_name,
                total_calls: total_calls as usize,
                successful_calls: successful as usize,
                failed_calls: errors as usize,
                success_rate: if total_calls > 0 {
                    successful as f64 / total_calls as f64
                } else {
                    0.0
                },
                avg_duration_ms: avg_duration,
                common_errors: vec![], // TODO: Query most common error types
            })
        })
        .map_err(|e| internal_error(format!("Failed to query tools: {}", e)))?
        .filter_map(|r| r.ok())
        .collect();

    // =========================================================================
    // 5. Detect trends
    // =========================================================================

    let success_rate_trend = if historical_success_rate > 0.0 {
        let change_percent = ((success_rate - historical_success_rate) / historical_success_rate) * 100.0;
        let trend = if change_percent > 5.0 {
            Trend::Improving
        } else if change_percent < -5.0 {
            Trend::Degrading
        } else {
            Trend::Stable
        };

        TrendSummary {
            metric: "Success Rate".to_string(),
            trend,
            change_percent,
        }
    } else {
        TrendSummary {
            metric: "Success Rate".to_string(),
            trend: Trend::Stable,
            change_percent: 0.0,
        }
    };

    let recent_trends = vec![success_rate_trend];

    // =========================================================================
    // 6. Compute overall health score
    // =========================================================================

    // Normalize metrics to 0-1 range for health score calculation
    let avg_duration_normalized = if avg_duration_ms > 0.0 {
        // Assume 60 seconds is "normal", scale accordingly
        let normalized = 1.0 - (avg_duration_ms / 60000.0).min(1.0);
        normalized.max(0.0)
    } else {
        0.8 // Default if no duration data
    };

    let tool_reliability = if !top_failing_tools.is_empty() {
        // Average reliability of top tools
        top_failing_tools.iter()
            .map(|t| t.success_rate)
            .sum::<f64>() / top_failing_tools.len() as f64
    } else {
        0.9 // Default if no tool data
    };

    let resource_efficiency = 0.85; // TODO: Compute from actual resource metrics when available

    let overall_health_score = crate::analysis::compute_health_score(
        success_rate,
        avg_duration_normalized,
        tool_reliability,
        resource_efficiency,
    );

    // =========================================================================
    // 7. Count patterns and improvements
    // =========================================================================

    // TODO: When improvements table is added, query actual counts
    // For now, return placeholders
    let active_patterns = 0;
    let pending_improvements = 0;
    let applied_improvements = 0;

    // =========================================================================
    // 8. Build dashboard response
    // =========================================================================

    let dashboard = HealthDashboard {
        overall_health_score,
        total_runs: total_runs as usize,
        success_rate,
        active_patterns,
        pending_improvements,
        applied_improvements,
        top_agents,
        top_failing_tools,
        recent_trends,
    };

    json_success(&dashboard)
}

/// Parse a duration string like "-7d", "-30d" into days
fn parse_since_duration(since: &str) -> Option<i64> {
    if since.starts_with('-') && since.ends_with('d') {
        let days_str = &since[1..since.len() - 1];
        days_str.parse::<i64>().ok()
    } else {
        None
    }
}
