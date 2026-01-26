//! Monitor module for autonomous agent polling and dispatching
//!
//! This module provides the monitoring functionality that:
//! - Polls GitHub for issues/PRs needing attention
//! - Writes reports to the local inbox
//! - Dispatches tasks to the agent when work is needed
//! - Sends notifications via configured channels

use anyhow::Result;
use chrono::Local;

use crate::mcp::{McpClientPool, McpTool};

/// Configuration for the monitor
pub struct MonitorConfig {
    /// Ollama server URL
    pub ollama_url: String,
    /// Model to use
    pub model: String,
    /// Repositories to monitor (owner/repo format)
    pub repos: Vec<String>,
    /// Whether to run once or continuously
    pub once: bool,
    /// Polling interval in seconds (for continuous mode)
    pub interval: u64,
    /// Optional system prompt override
    pub system_prompt: Option<String>,
}

/// The Monitor struct that handles polling and dispatching
pub struct Monitor {
    config: MonitorConfig,
    pool: McpClientPool,
}

impl Monitor {
    /// Create a new Monitor with the given configuration
    pub fn new(config: MonitorConfig, pool: McpClientPool) -> Self {
        Self { config, pool }
    }

    /// Run the monitor (once or continuously based on config)
    pub async fn run(&mut self) -> Result<()> {
        if self.config.once {
            self.run_once().await
        } else {
            self.run_loop().await
        }
    }

    /// Run a single monitoring cycle
    pub async fn run_once(&mut self) -> Result<()> {
        let now = Local::now();
        println!(
            "[{}] Starting monitor cycle...",
            now.format("%Y-%m-%d %H:%M:%S")
        );

        // 1. Poll GitHub for each configured repo
        for repo in &self.config.repos.clone() {
            println!("  Checking {}...", repo);
            match self.check_repo(repo).await {
                Ok(summary) => {
                    if !summary.is_empty() {
                        println!("    {}", summary);
                        // Write to inbox
                        self.write_to_inbox(&format!("Repo check: {}\n{}", repo, summary))
                            .await?;
                    } else {
                        println!("    No actionable items");
                    }
                }
                Err(e) => {
                    eprintln!("    Error checking {}: {}", repo, e);
                }
            }
        }

        // 2. Write completion message to inbox
        self.write_to_inbox(&format!(
            "Monitor cycle completed. Checked {} repos.",
            self.config.repos.len()
        ))
        .await?;

        println!(
            "[{}] Monitor cycle complete",
            Local::now().format("%Y-%m-%d %H:%M:%S")
        );
        Ok(())
    }

    /// Run the monitor in a continuous loop
    pub async fn run_loop(&mut self) -> Result<()> {
        println!(
            "Starting continuous monitor (interval: {}s)",
            self.config.interval
        );
        println!("Press Ctrl+C to stop\n");

        loop {
            if let Err(e) = self.run_once().await {
                eprintln!("Monitor cycle error: {}", e);
            }

            println!("\nSleeping for {} seconds...\n", self.config.interval);
            tokio::time::sleep(std::time::Duration::from_secs(self.config.interval)).await;
        }
    }

    /// Check a single repository for actionable items
    async fn check_repo(&mut self, repo: &str) -> Result<String> {
        let mut summary_parts = Vec::new();

        // Check for open issues assigned to us (or with certain labels)
        let issues = self.get_repo_issues(repo).await?;
        if !issues.is_empty() {
            summary_parts.push(format!("{} open issues", issues.len()));
        }

        // Check for PRs needing review
        let prs = self.get_repo_prs(repo).await?;
        if !prs.is_empty() {
            summary_parts.push(format!("{} open PRs", prs.len()));
        }

        // Check workflow status
        let workflow_status = self.get_workflow_status(repo).await?;
        if !workflow_status.is_empty() {
            summary_parts.push(workflow_status);
        }

        Ok(summary_parts.join(", "))
    }

    /// Get open issues from a repository
    async fn get_repo_issues(&mut self, repo: &str) -> Result<Vec<String>> {
        let args = serde_json::json!({
            "repo": repo,
            "state": "open",
            "limit": 10
        });

        match self.pool.call_tool("gh_issue_list", Some(args)).await {
            Ok(result) => {
                // Parse the result and extract issue titles
                let mut issues = Vec::new();
                for content in &result.content {
                    if let rmcp::model::RawContent::Text(text) = &content.raw {
                        // Try to parse as JSON array
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text.text) {
                            if let Some(arr) = parsed.as_array() {
                                for item in arr {
                                    if let Some(title) = item.get("title").and_then(|t| t.as_str())
                                    {
                                        issues.push(title.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(issues)
            }
            Err(e) => {
                tracing::warn!("Failed to get issues for {}: {}", repo, e);
                Ok(Vec::new())
            }
        }
    }

    /// Get open PRs from a repository
    async fn get_repo_prs(&mut self, repo: &str) -> Result<Vec<String>> {
        let args = serde_json::json!({
            "repo": repo,
            "state": "open",
            "limit": 10
        });

        match self.pool.call_tool("gh_pr_list", Some(args)).await {
            Ok(result) => {
                let mut prs = Vec::new();
                for content in &result.content {
                    if let rmcp::model::RawContent::Text(text) = &content.raw {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text.text) {
                            if let Some(arr) = parsed.as_array() {
                                for item in arr {
                                    if let Some(title) = item.get("title").and_then(|t| t.as_str())
                                    {
                                        prs.push(title.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(prs)
            }
            Err(e) => {
                tracing::warn!("Failed to get PRs for {}: {}", repo, e);
                Ok(Vec::new())
            }
        }
    }

    /// Get workflow status for a repository
    async fn get_workflow_status(&mut self, repo: &str) -> Result<String> {
        let args = serde_json::json!({
            "repo": repo,
            "limit": 5
        });

        match self.pool.call_tool("gh_run_list", Some(args)).await {
            Ok(result) => {
                let mut failed_count = 0;
                let mut in_progress_count = 0;

                for content in &result.content {
                    if let rmcp::model::RawContent::Text(text) = &content.raw {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text.text) {
                            if let Some(arr) = parsed.as_array() {
                                for item in arr {
                                    if let Some(conclusion) =
                                        item.get("conclusion").and_then(|c| c.as_str())
                                    {
                                        if conclusion == "failure" {
                                            failed_count += 1;
                                        }
                                    }
                                    if let Some(status) =
                                        item.get("status").and_then(|s| s.as_str())
                                    {
                                        if status == "in_progress" || status == "queued" {
                                            in_progress_count += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                let mut parts = Vec::new();
                if failed_count > 0 {
                    parts.push(format!("{} failed workflows", failed_count));
                }
                if in_progress_count > 0 {
                    parts.push(format!("{} in-progress workflows", in_progress_count));
                }
                Ok(parts.join(", "))
            }
            Err(e) => {
                tracing::warn!("Failed to get workflow status for {}: {}", repo, e);
                Ok(String::new())
            }
        }
    }

    /// Write a message to the local inbox
    async fn write_to_inbox(&mut self, message: &str) -> Result<()> {
        let args = serde_json::json!({
            "message": message,
            "source": "monitor",
            "tags": ["monitor", "status"]
        });

        match self.pool.call_tool("write_inbox", Some(args)).await {
            Ok(_) => {
                tracing::debug!("Wrote to inbox: {}", &message[..message.len().min(50)]);
                Ok(())
            }
            Err(e) => {
                // Don't fail if inbox isn't available, just log it
                tracing::warn!("Failed to write to inbox (is inbox-mcp configured?): {}", e);
                Ok(())
            }
        }
    }

    /// Send a notification via configured channels
    #[allow(dead_code)]
    async fn send_notification(&mut self, message: &str) -> Result<()> {
        let args = serde_json::json!({
            "message": message,
        });

        // Try Slack first
        if let Err(e) = self.pool.call_tool("send_slack", Some(args.clone())).await {
            tracing::debug!("Slack notification failed (may not be configured): {}", e);
        }

        // Try Discord
        let discord_args = serde_json::json!({
            "content": message,
        });
        if let Err(e) = self
            .pool
            .call_tool("send_discord", Some(discord_args))
            .await
        {
            tracing::debug!("Discord notification failed (may not be configured): {}", e);
        }

        Ok(())
    }
}

/// Run the monitor with the given configuration
pub async fn run_monitor(config: MonitorConfig) -> Result<()> {
    // Load MCP pool
    let mut pool = McpClientPool::load()?
        .ok_or_else(|| anyhow::anyhow!("No .mcp.json found - monitor needs MCP tools"))?;

    // Check required tools
    let tools: Vec<McpTool> = pool.list_all_tools().await?;

    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();

    // Check for github-gh tools
    if !tool_names.iter().any(|n: &&str| n.starts_with("gh_")) {
        println!("Warning: github-gh MCP server not found. Some features may not work.");
    }

    // Check for inbox tool
    if !tool_names.contains(&"write_inbox") {
        println!("Warning: inbox-mcp server not found. Inbox logging will be disabled.");
    }

    let mut monitor = Monitor::new(config, pool);
    monitor.run().await
}
