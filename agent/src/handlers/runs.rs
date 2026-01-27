//! Runs command handler
//!
//! View, analyze, and manage workflow runs.

use anyhow::Result;

use super::CommandContext;
use crate::cli::runs_args::{ExportFormat, ImprovementCategory, RunsCommands};
use crate::db::runs::{ImprovementFilter, ImprovementStatus, RunFilter, RunStatus};
use crate::db::Database;

/// Handle the `runs` command
pub async fn run_runs_command(_ctx: &CommandContext, command: RunsCommands) -> Result<()> {
    let db = Database::open()?;

    match command {
        RunsCommands::List {
            limit,
            workflow,
            status,
        } => run_list(&db, limit, workflow, status).await,
        RunsCommands::View { id, events, full } => run_view(&db, &id, events, full).await,
        RunsCommands::Export { id, format, output } => run_export(&db, &id, format, output).await,
        RunsCommands::Compare { run_a, run_b } => run_compare(&db, &run_a, &run_b).await,
        RunsCommands::Summary { last, workflow } => run_summary(&db, last, workflow).await,
        RunsCommands::Improve {
            category,
            description,
            runs,
        } => run_improve(&db, category, &description, runs).await,
        RunsCommands::Improvements {
            status,
            category,
            limit,
        } => run_list_improvements(&db, status, category, limit).await,
    }
}

async fn run_list(
    db: &Database,
    limit: u32,
    workflow: Option<String>,
    status: Option<String>,
) -> Result<()> {
    let status = status.map(|s| s.parse::<RunStatus>()).transpose()?;

    let filter = RunFilter {
        workflow_name: workflow,
        status,
        limit: Some(limit),
        offset: None,
    };

    let runs = db.list_runs(&filter)?;

    if runs.is_empty() {
        println!("No runs found.");
        return Ok(());
    }

    println!("Recent workflow runs:\n");
    println!(
        "{:<12} {:<20} {:<12} {:<10} {:<12}",
        "ID", "WORKFLOW", "STATUS", "DURATION", "STARTED"
    );
    println!("{}", "─".repeat(70));

    for run in runs {
        let id_short = if run.id.len() > 8 {
            &run.id[..8]
        } else {
            &run.id
        };
        let duration = run
            .duration_ms
            .map(format_duration)
            .unwrap_or_else(|| "running".to_string());
        let started = run.started_at.format("%Y-%m-%d %H:%M").to_string();

        println!(
            "{:<12} {:<20} {:<12} {:<10} {:<12}",
            id_short, run.workflow_name, run.status, duration, started
        );
    }

    Ok(())
}

async fn run_view(db: &Database, id: &str, show_events: bool, full: bool) -> Result<()> {
    // Support partial ID matching
    let run = find_run_by_prefix(db, id)?;

    println!("\n{}", "═".repeat(60));
    println!("  RUN: {}", run.id);
    println!("{}\n", "═".repeat(60));

    println!("Workflow:   {}", run.workflow_name);
    println!("Task:       {}", run.task);
    println!("Status:     {}", run.status);
    println!("Model:      {}", run.model);
    println!(
        "Started:    {}",
        run.started_at.format("%Y-%m-%d %H:%M:%S UTC")
    );

    if let Some(completed_at) = run.completed_at {
        println!(
            "Completed:  {}",
            completed_at.format("%Y-%m-%d %H:%M:%S UTC")
        );
    }

    if let Some(duration) = run.duration_ms {
        println!("Duration:   {}", format_duration(duration));
    }

    if let Some(ref error) = run.error {
        println!("\nError: {}", error);
    }

    // Show metrics if available
    if let Some(metrics) = db.get_run_metrics(&run.id)? {
        println!("\nMetrics:");
        println!(
            "  Tool calls: {} ({} successful, {} failed)",
            metrics.total_tool_calls, metrics.successful_tool_calls, metrics.failed_tool_calls
        );
        println!("  Iterations: {}", metrics.total_iterations);
        println!(
            "  Files: {} read, {} modified",
            metrics.files_read, metrics.files_modified
        );

        if !metrics.tools_used.is_empty() {
            println!("\n  Tools used:");
            let mut tools: Vec<_> = metrics.tools_used.iter().collect();
            tools.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
            for (tool, count) in tools.iter().take(10) {
                println!("    {}: {}", tool, count);
            }
        }
    }

    // Show context output
    if let Some(ref context) = run.context {
        if let Some(obj) = context.as_object() {
            println!(
                "\nContext keys: {}",
                obj.keys().cloned().collect::<Vec<_>>().join(", ")
            );

            if full {
                for (key, value) in obj {
                    println!("\n--- {} ---", key);
                    if let Some(s) = value.as_str() {
                        println!("{}", s);
                    } else {
                        println!("{}", serde_json::to_string_pretty(value)?);
                    }
                }
            }
        }
    }

    // Show events if requested
    if show_events {
        let events = db.get_run_events(&run.id)?;
        println!("\nEvents ({} total):", events.len());
        println!("{}", "─".repeat(60));

        for event in events {
            let time = event.timestamp.format("%H:%M:%S%.3f").to_string();
            println!(
                "[{}] Step {} - {}",
                time, event.step_index, event.event_type
            );

            if full {
                if let Some(obj) = event.event_data.as_object() {
                    for (key, value) in obj {
                        let value_str = if let Some(s) = value.as_str() {
                            if s.len() > 100 && !full {
                                format!("{}...", &s[..100])
                            } else {
                                s.to_string()
                            }
                        } else {
                            value.to_string()
                        };
                        println!("    {}: {}", key, value_str);
                    }
                }
            }
        }
    }

    Ok(())
}

async fn run_export(
    db: &Database,
    id: &str,
    format: ExportFormat,
    output: Option<String>,
) -> Result<()> {
    let run = find_run_by_prefix(db, id)?;
    let events = db.get_run_events(&run.id)?;
    let metrics = db.get_run_metrics(&run.id)?;

    let content = match format {
        ExportFormat::Markdown => export_markdown(&run, &events, metrics.as_ref()),
        ExportFormat::Json => export_json(&run, &events, metrics.as_ref())?,
    };

    match output {
        Some(path) => {
            std::fs::write(&path, &content)?;
            println!("Exported to: {}", path);
        }
        None => {
            println!("{}", content);
        }
    }

    Ok(())
}

async fn run_compare(db: &Database, run_a_id: &str, run_b_id: &str) -> Result<()> {
    let run_a = find_run_by_prefix(db, run_a_id)?;
    let run_b = find_run_by_prefix(db, run_b_id)?;

    let metrics_a = db.get_run_metrics(&run_a.id)?;
    let metrics_b = db.get_run_metrics(&run_b.id)?;

    println!("\n{}", "═".repeat(60));
    println!("  COMPARISON: {} vs {}", &run_a.id[..8], &run_b.id[..8]);
    println!("{}\n", "═".repeat(60));

    println!(
        "{:<20} {:<20} {:<20}",
        "METRIC",
        format!("RUN A ({})", &run_a.id[..8]),
        format!("RUN B ({})", &run_b.id[..8])
    );
    println!("{}", "─".repeat(60));

    println!(
        "{:<20} {:<20} {:<20}",
        "Workflow", run_a.workflow_name, run_b.workflow_name
    );
    println!(
        "{:<20} {:<20} {:<20}",
        "Status",
        run_a.status.to_string(),
        run_b.status.to_string()
    );

    let dur_a = run_a
        .duration_ms
        .map(format_duration)
        .unwrap_or("-".to_string());
    let dur_b = run_b
        .duration_ms
        .map(format_duration)
        .unwrap_or("-".to_string());
    println!("{:<20} {:<20} {:<20}", "Duration", dur_a, dur_b);

    if let (Some(ma), Some(mb)) = (&metrics_a, &metrics_b) {
        println!(
            "{:<20} {:<20} {:<20}",
            "Tool calls", ma.total_tool_calls, mb.total_tool_calls
        );
        let rate_a = if ma.total_tool_calls > 0 {
            format!(
                "{:.0}%",
                (ma.successful_tool_calls as f64 / ma.total_tool_calls as f64) * 100.0
            )
        } else {
            "-".to_string()
        };
        let rate_b = if mb.total_tool_calls > 0 {
            format!(
                "{:.0}%",
                (mb.successful_tool_calls as f64 / mb.total_tool_calls as f64) * 100.0
            )
        } else {
            "-".to_string()
        };
        println!("{:<20} {:<20} {:<20}", "Success rate", rate_a, rate_b);
        println!(
            "{:<20} {:<20} {:<20}",
            "Iterations", ma.total_iterations, mb.total_iterations
        );
        println!(
            "{:<20} {:<20} {:<20}",
            "Files read", ma.files_read, mb.files_read
        );
        println!(
            "{:<20} {:<20} {:<20}",
            "Files modified", ma.files_modified, mb.files_modified
        );
    }

    Ok(())
}

async fn run_summary(db: &Database, last: u32, workflow: Option<String>) -> Result<()> {
    let filter = RunFilter {
        workflow_name: workflow.clone(),
        status: None,
        limit: Some(last),
        offset: None,
    };

    let runs = db.list_runs(&filter)?;

    if runs.is_empty() {
        println!("No runs found.");
        return Ok(());
    }

    let title = match workflow {
        Some(ref w) => format!("Summary for workflow '{}'", w),
        None => "Summary of recent runs".to_string(),
    };

    println!("\n{}", "═".repeat(60));
    println!("  {}", title);
    println!("{}\n", "═".repeat(60));

    let total = runs.len();
    let completed = runs
        .iter()
        .filter(|r| r.status == RunStatus::Completed)
        .count();
    let failed = runs
        .iter()
        .filter(|r| r.status == RunStatus::Failed)
        .count();
    let running = runs
        .iter()
        .filter(|r| r.status == RunStatus::Running)
        .count();
    let cancelled = runs
        .iter()
        .filter(|r| r.status == RunStatus::Cancelled)
        .count();

    println!("Total runs:   {}", total);
    println!(
        "  Completed:  {} ({:.0}%)",
        completed,
        (completed as f64 / total as f64) * 100.0
    );
    println!(
        "  Failed:     {} ({:.0}%)",
        failed,
        (failed as f64 / total as f64) * 100.0
    );
    println!("  Running:    {}", running);
    println!("  Cancelled:  {}", cancelled);

    // Calculate average duration for completed runs
    let completed_durations: Vec<i64> = runs
        .iter()
        .filter(|r| r.status == RunStatus::Completed && r.duration_ms.is_some())
        .filter_map(|r| r.duration_ms)
        .collect();

    if !completed_durations.is_empty() {
        let avg = completed_durations.iter().sum::<i64>() / completed_durations.len() as i64;
        let min = *completed_durations.iter().min().unwrap();
        let max = *completed_durations.iter().max().unwrap();
        println!("\nDuration (completed runs):");
        println!("  Average:    {}", format_duration(avg));
        println!("  Min:        {}", format_duration(min));
        println!("  Max:        {}", format_duration(max));
    }

    // Group by workflow if not filtered
    if workflow.is_none() {
        let mut by_workflow: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for run in &runs {
            *by_workflow.entry(run.workflow_name.clone()).or_insert(0) += 1;
        }
        println!("\nBy workflow:");
        let mut workflows: Vec<_> = by_workflow.into_iter().collect();
        workflows.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        for (name, count) in workflows {
            println!("  {}: {}", name, count);
        }
    }

    Ok(())
}

async fn run_improve(
    db: &Database,
    category: ImprovementCategory,
    description: &str,
    runs: Option<String>,
) -> Result<()> {
    let related_runs: Vec<String> = runs
        .map(|r| r.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    let category = crate::db::runs::ImprovementCategory::from(category);
    let improvement = db.create_improvement(category, description, &related_runs)?;

    println!("Created improvement: {}", improvement.id);
    println!("  Category:    {}", improvement.category);
    println!("  Description: {}", improvement.description);
    if !improvement.related_runs.is_empty() {
        println!("  Related runs: {}", improvement.related_runs.join(", "));
    }
    println!("  Status:      {}", improvement.status);

    Ok(())
}

async fn run_list_improvements(
    db: &Database,
    status: Option<String>,
    category: Option<ImprovementCategory>,
    limit: u32,
) -> Result<()> {
    let status = status.map(|s| s.parse::<ImprovementStatus>()).transpose()?;
    let category = category.map(crate::db::runs::ImprovementCategory::from);

    let filter = ImprovementFilter {
        category,
        status,
        limit: Some(limit),
        offset: None,
    };

    let improvements = db.list_improvements(&filter)?;

    if improvements.is_empty() {
        println!("No improvements found.");
        return Ok(());
    }

    println!("Recorded improvements:\n");
    println!(
        "{:<12} {:<12} {:<12} {:<40}",
        "ID", "CATEGORY", "STATUS", "DESCRIPTION"
    );
    println!("{}", "─".repeat(80));

    for imp in improvements {
        let id_short = if imp.id.len() > 8 {
            &imp.id[..8]
        } else {
            &imp.id
        };
        let desc_short = if imp.description.len() > 37 {
            format!("{}...", &imp.description[..37])
        } else {
            imp.description.clone()
        };

        println!(
            "{:<12} {:<12} {:<12} {:<40}",
            id_short, imp.category, imp.status, desc_short
        );
    }

    Ok(())
}

// ============================================================================
// Helper Functions
// ============================================================================

fn find_run_by_prefix(db: &Database, prefix: &str) -> Result<crate::db::runs::Run> {
    // First try exact match
    if let Some(run) = db.get_run(prefix)? {
        return Ok(run);
    }

    // Try prefix match by listing and filtering
    let runs = db.list_runs(&RunFilter {
        limit: Some(100),
        ..Default::default()
    })?;

    let matches: Vec<_> = runs.iter().filter(|r| r.id.starts_with(prefix)).collect();

    match matches.len() {
        0 => Err(anyhow::anyhow!("No run found with ID prefix: {}", prefix)),
        1 => db
            .get_run(&matches[0].id)?
            .ok_or_else(|| anyhow::anyhow!("Run not found")),
        _ => Err(anyhow::anyhow!(
            "Ambiguous ID prefix '{}', matches {} runs",
            prefix,
            matches.len()
        )),
    }
}

fn format_duration(ms: i64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        let minutes = ms / 60000;
        let seconds = (ms % 60000) / 1000;
        format!("{}m {}s", minutes, seconds)
    }
}

fn export_markdown(
    run: &crate::db::runs::Run,
    events: &[crate::db::runs::RunEvent],
    metrics: Option<&crate::db::runs::RunMetrics>,
) -> String {
    let mut md = String::new();

    md.push_str(&format!("# Run Analysis: {}\n\n", &run.id[..8]));

    md.push_str("## Overview\n\n");
    md.push_str(&format!("- **Workflow:** {}\n", run.workflow_name));
    md.push_str(&format!("- **Task:** {}\n", run.task));
    md.push_str(&format!("- **Status:** {}\n", run.status));
    md.push_str(&format!("- **Model:** {}\n", run.model));

    if let Some(duration) = run.duration_ms {
        md.push_str(&format!("- **Duration:** {}\n", format_duration(duration)));
    }

    md.push_str(&format!(
        "- **Started:** {}\n",
        run.started_at.format("%Y-%m-%d %H:%M:%S UTC")
    ));

    if let Some(completed) = run.completed_at {
        md.push_str(&format!(
            "- **Completed:** {}\n",
            completed.format("%Y-%m-%d %H:%M:%S UTC")
        ));
    }

    if let Some(error) = &run.error {
        md.push_str(&format!("\n**Error:** {}\n", error));
    }

    if let Some(metrics) = metrics {
        md.push_str("\n## Metrics\n\n");
        md.push_str("| Metric | Value |\n");
        md.push_str("|--------|-------|\n");
        md.push_str(&format!(
            "| Total tool calls | {} |\n",
            metrics.total_tool_calls
        ));
        md.push_str(&format!(
            "| Successful | {} |\n",
            metrics.successful_tool_calls
        ));
        md.push_str(&format!("| Failed | {} |\n", metrics.failed_tool_calls));
        md.push_str(&format!("| Iterations | {} |\n", metrics.total_iterations));
        md.push_str(&format!("| Files read | {} |\n", metrics.files_read));
        md.push_str(&format!(
            "| Files modified | {} |\n",
            metrics.files_modified
        ));

        if metrics.total_tool_calls > 0 {
            let rate =
                (metrics.successful_tool_calls as f64 / metrics.total_tool_calls as f64) * 100.0;
            md.push_str(&format!("| Success rate | {:.1}% |\n", rate));
        }

        if !metrics.tools_used.is_empty() {
            md.push_str("\n### Tool Usage\n\n");
            md.push_str("| Tool | Calls |\n");
            md.push_str("|------|-------|\n");

            let mut tools: Vec<_> = metrics.tools_used.iter().collect();
            tools.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
            for (tool, count) in tools {
                md.push_str(&format!("| {} | {} |\n", tool, count));
            }
        }
    }

    if !events.is_empty() {
        md.push_str("\n## Events\n\n");

        // Group events by step
        let mut current_step = 0;
        for event in events {
            if event.step_index != current_step {
                current_step = event.step_index;
                md.push_str(&format!("\n### Step {}\n\n", current_step + 1));
            }

            let time = event.timestamp.format("%H:%M:%S").to_string();
            md.push_str(&format!("- `{}` **{}**", time, event.event_type));

            // Add relevant details based on event type
            if let Some(obj) = event.event_data.as_object() {
                if let Some(name) = obj.get("name").and_then(|v| v.as_str()) {
                    md.push_str(&format!(" - `{}`", name));
                }
                if let Some(is_error) = obj.get("is_error").and_then(|v| v.as_bool()) {
                    if is_error {
                        md.push_str(" ❌");
                    }
                }
            }
            md.push('\n');
        }
    }

    // Analysis questions section
    md.push_str("\n## Analysis Questions\n\n");
    md.push_str("1. Were there any repeated failures that suggest a systematic issue?\n");
    md.push_str("2. Did the agent efficiently find the files it needed?\n");
    md.push_str("3. Were there unnecessary iterations or tool calls?\n");
    md.push_str("4. What improvements could reduce time or increase reliability?\n");

    md
}

fn export_json(
    run: &crate::db::runs::Run,
    events: &[crate::db::runs::RunEvent],
    metrics: Option<&crate::db::runs::RunMetrics>,
) -> Result<String> {
    let export = serde_json::json!({
        "run": run,
        "events": events,
        "metrics": metrics,
    });

    Ok(serde_json::to_string_pretty(&export)?)
}
