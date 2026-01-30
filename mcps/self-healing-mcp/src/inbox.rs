//! Inbox notification integration
//!
//! Writes notifications to ~/.notes/inbox/ in the same format as inbox-mcp.
//! This allows self-healing-mcp to send notifications without depending on inbox-mcp being running.

use anyhow::Result;
use chrono::Local;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

/// Priority level for inbox messages
#[derive(Debug, Clone, Copy)]
pub enum Priority {
    #[allow(dead_code)]
    Low,
    Normal,
    High,
    Urgent,
}

impl Priority {
    fn to_markdown(self) -> &'static str {
        match self {
            Priority::Low => "[LOW]",
            Priority::Normal => "",
            Priority::High => "*[HIGH]*",
            Priority::Urgent => "**[URGENT]**",
        }
    }
}

/// Send a notification to the inbox
pub async fn send_notification(
    message: &str,
    priority: Priority,
    tags: &[&str],
    url: Option<&str>,
) -> Result<()> {
    // Get inbox path: ~/.notes/inbox/
    let inbox_path = std::env::var("INBOX_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".notes")
                .join("inbox")
        });

    // Ensure inbox directory exists
    fs::create_dir_all(&inbox_path).await?;

    // Get today's file path: YYYY-MM-DD.md
    let now = Local::now();
    let file_path = inbox_path.join(format!("{}.md", now.format("%Y-%m-%d")));

    // Format message in inbox-mcp format:
    // ## YYYY-MM-DD HH:MM:SS [source] #tag1 #tag2 *[priority]*
    // Message content here
    // Optional URL
    let tags_str = tags
        .iter()
        .map(|t| format!("#{}", t))
        .collect::<Vec<_>>()
        .join(" ");

    let priority_str = priority.to_markdown();

    let header = format!(
        "## {} [self-heal] {} {}",
        now.format("%Y-%m-%d %H:%M:%S"),
        tags_str,
        priority_str
    )
    .trim()
    .to_string();

    let content = if let Some(url_str) = url {
        format!("{}\n\n{}\n\n{}", header, message, url_str)
    } else {
        format!("{}\n\n{}", header, message)
    };

    // Open file for append (create if doesn't exist)
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file_path)
        .await?;

    // Check if file is empty to add header
    let metadata = file.metadata().await?;

    let full_content = if metadata.len() > 0 {
        format!("\n---\n\n{}\n", content)
    } else {
        format!("# Inbox - {}\n\n{}\n", now.format("%Y-%m-%d"), content)
    };

    file.write_all(full_content.as_bytes()).await?;

    Ok(())
}

/// Send pattern detection notification
#[allow(dead_code)]
pub async fn notify_pattern_detected(
    pattern_id: &str,
    error_type: &str,
    tool_name: Option<&str>,
    occurrences: usize,
    affected_runs: &[String],
    proposed_fix: &str,
    expected_impact: &str,
) -> Result<()> {
    let tool_str = tool_name.unwrap_or("workflow");
    let runs_str = if affected_runs.len() <= 3 {
        affected_runs.join(", ")
    } else {
        format!(
            "{}, {} (and {} more)",
            affected_runs[0],
            affected_runs[1],
            affected_runs.len() - 2
        )
    };

    let message = format!(
        "Detected failure pattern:\n\
        - Error: {} on {}\n\
        - Occurrences: {} in recent runs\n\
        - Affected runs: {}\n\n\
        Proposed fix: {}\n\
        Expected impact: {}\n\n\
        View details: /self-heal show {}\n\
        Apply fix: /self-heal apply {}",
        error_type,
        tool_str,
        occurrences,
        runs_str,
        proposed_fix,
        expected_impact,
        pattern_id,
        pattern_id
    );

    send_notification(&message, Priority::High, &["pattern", "self-heal"], None).await
}

/// Send improvement applied notification
pub async fn notify_improvement_applied(
    improvement_id: &str,
    description: &str,
    changes_made: &str,
    commit_hash: Option<&str>,
) -> Result<()> {
    let commit_str = if let Some(hash) = commit_hash {
        format!("\nCommit: {}", hash)
    } else {
        String::new()
    };

    let message = format!(
        "Applied improvement {}:\n\
        Description: {}\n\
        Changes: {}{}\n\n\
        Monitoring for 7 days to verify impact.",
        improvement_id, description, changes_made, commit_str
    );

    send_notification(
        &message,
        Priority::Normal,
        &["improvement", "applied"],
        None,
    )
    .await
}

/// Send verification result notification
pub async fn notify_verification_result(
    improvement_id: &str,
    expected_impact: &str,
    actual_impact: f64,
    success_rate_before: f64,
    success_rate_after: f64,
    runs_analyzed: usize,
    recommendation: &str,
) -> Result<()> {
    let impact_comparison = if actual_impact > 0.0 {
        format!(
            "{}% improvement (better than expected!)",
            actual_impact * 100.0
        )
    } else if actual_impact < 0.0 {
        format!("{}% degradation", actual_impact.abs() * 100.0)
    } else {
        "no significant change".to_string()
    };

    let message = format!(
        "Verified improvement {}:\n\
        Expected: {}\n\
        Actual: {}\n\
        Recommendation: {}\n\n\
        Metrics:\n\
        - Success rate before: {:.1}%\n\
        - Success rate after: {:.1}%\n\
        - Runs analyzed: {}",
        improvement_id,
        expected_impact,
        impact_comparison,
        recommendation,
        success_rate_before * 100.0,
        success_rate_after * 100.0,
        runs_analyzed
    );

    let priority = if recommendation.contains("Rollback") {
        Priority::Urgent
    } else if recommendation.contains("Keep") {
        Priority::Normal
    } else {
        Priority::High
    };

    send_notification(&message, priority, &["improvement", "verified"], None).await
}
