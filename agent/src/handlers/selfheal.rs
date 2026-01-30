//! Self-healing command handler
//!
//! Analyze workflow failures and propose/apply automated improvements.

use anyhow::{Context, Result};

use super::CommandContext;
use crate::cli::selfheal_args::SelfHealCommands;

/// Handle the `self-heal` command
pub async fn run_selfheal_command(ctx: &CommandContext, command: Option<SelfHealCommands>) -> Result<()> {
    // If no subcommand provided, default to Detect
    let command = command.unwrap_or(SelfHealCommands::Detect {
        since: "-7d".to_string(),
        min_occurrences: 3,
        confidence: 0.75,
    });

    match command {
        SelfHealCommands::Detect {
            since,
            min_occurrences,
            confidence,
        } => detect_patterns(ctx, &since, min_occurrences, confidence).await,
        SelfHealCommands::Show { pattern_id } => show_pattern(ctx, &pattern_id).await,
        SelfHealCommands::Test { improvement_id } => test_improvement(ctx, &improvement_id).await,
        SelfHealCommands::Apply { improvement_id, yes } => {
            apply_improvement(ctx, &improvement_id, yes).await
        }
        SelfHealCommands::Verify {
            improvement_id,
            window_days,
        } => verify_improvement(ctx, &improvement_id, window_days).await,
        SelfHealCommands::Dashboard { detailed } => show_dashboard(ctx, detailed).await,
        SelfHealCommands::Improvements { status, limit } => {
            list_improvements(ctx, status, limit).await
        }
    }
}

/// Step 1-2: Detect failure patterns and propose improvements
async fn detect_patterns(
    ctx: &CommandContext,
    since: &str,
    min_occurrences: usize,
    confidence: f64,
) -> Result<()> {
    println!("🔍 Detecting failure patterns (last {}, min {} occurrences, confidence >= {:.0}%)...",
        since, min_occurrences, confidence * 100.0);

    // Get MCP pool to call self-healing-mcp tools
    let mut pool = crate::mcp::McpClientPool::load()?
        .ok_or_else(|| anyhow::anyhow!("No .mcp.json found - self-healing-mcp tools required"))?;

    // Call detect_failure_patterns from self-healing-mcp
    let patterns_result = pool
        .call_tool(
            "detect_failure_patterns",
            Some(serde_json::json!({
                "since": since,
                "min_occurrences": min_occurrences,
                "confidence_threshold": confidence,
            })),
        )
        .await
        .context("Failed to detect patterns")?;

    // Parse the result - extract text from Content
    let text = patterns_result
        .content
        .iter()
        .filter_map(|c| match &c.raw {
            rmcp::model::RawContent::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .next()
        .ok_or_else(|| anyhow::anyhow!("No text content returned from detect_failure_patterns"))?;

    let patterns: serde_json::Value = serde_json::from_str(text)
        .context("Failed to parse pattern detection result")?;

    // Check if any patterns found - MCP tool returns array directly
    let pattern_list = patterns
        .as_array()
        .context("Expected patterns array")?;

    if pattern_list.is_empty() {
        println!("✅ System healthy - no recurring failure patterns detected");
        return Ok(());
    }

    println!("\n📊 Found {} pattern(s):\n", pattern_list.len());

    // Display patterns
    for pattern in pattern_list {
        let pattern_id = pattern["id"].as_str().unwrap_or("unknown");
        let error_type = pattern["error_type"].as_str().unwrap_or("unknown");
        let tool_name = pattern["tool_name"].as_str().unwrap_or("unknown");
        let occurrences = pattern["occurrences"].as_u64().unwrap_or(0);
        let correlation_score = pattern["correlation_score"].as_f64().unwrap_or(0.0);

        println!("  Pattern {}", pattern_id);
        println!("    Error: {} on {}", error_type, tool_name);
        println!("    Occurrences: {}", occurrences);
        println!("    Correlation: {:.0}%", correlation_score * 100.0);
        println!();
    }

    // Propose improvements for each pattern
    println!("💡 Generating improvement proposals...\n");

    for pattern in pattern_list {
        let pattern_id = pattern["id"].as_str().unwrap_or("unknown");

        let proposal_result = pool
            .call_tool(
                "propose_improvement",
                Some(serde_json::json!({
                    "pattern_id": pattern_id,
                })),
            )
            .await
            .context(format!("Failed to propose improvement for pattern {}", pattern_id))?;

        // Extract text from Content
        let text = proposal_result
            .content
            .iter()
            .filter_map(|c| match &c.raw {
                rmcp::model::RawContent::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .next()
            .ok_or_else(|| anyhow::anyhow!("No text content returned from propose_improvement"))?;

        let proposal: serde_json::Value = serde_json::from_str(text)
            .context("Failed to parse proposal")?;

        let improvement_id = proposal["improvement_id"].as_str().unwrap_or("unknown");
        let description = proposal["description"].as_str().unwrap_or("No description");
        let expected_impact = proposal["expected_impact"].as_str().unwrap_or("Unknown");

        println!("  ✓ Proposal {} for pattern {}", improvement_id, pattern_id);
        println!("    Description: {}", description);
        println!("    Expected impact: {}", expected_impact);
        println!();
    }

    // Write summary to inbox
    println!("📬 Writing summary to inbox...");
    write_to_inbox(ctx, pattern_list.len()).await?;

    println!("\n📋 Next steps:");
    println!("  - Test: agent self-heal test <improvement-id>");
    println!("  - Apply: agent self-heal apply <improvement-id>");
    println!("  - View: agent self-heal show <pattern-id>");

    Ok(())
}

/// Show details of a detected pattern
async fn show_pattern(_ctx: &CommandContext, pattern_id: &str) -> Result<()> {
    println!("🔍 Showing pattern details for: {}", pattern_id);
    println!("⚠️  Not yet implemented - will show:");
    println!("  - Error type and affected tool");
    println!("  - List of affected runs with timestamps");
    println!("  - Context similarity analysis");
    println!("  - Suggested fix strategy");
    Ok(())
}

/// Test improvement in simulation mode
async fn test_improvement(_ctx: &CommandContext, improvement_id: &str) -> Result<()> {
    println!("🧪 Testing improvement: {}", improvement_id);
    println!("⚠️  Not yet implemented - will:");
    println!("  - Run simulation against historical data");
    println!("  - Show expected vs simulated impact");
    println!("  - Provide recommendation (safe/risky/reject)");
    Ok(())
}

/// Apply an approved improvement
async fn apply_improvement(_ctx: &CommandContext, improvement_id: &str, yes: bool) -> Result<()> {
    println!("🔧 Applying improvement: {}", improvement_id);

    if !yes {
        println!("\n⚠️  This will modify system configuration.");
        println!("Use --yes to skip this confirmation.");
        return Ok(());
    }

    println!("⚠️  Not yet implemented - will:");
    println!("  - Update config/code as specified");
    println!("  - Record application in database");
    println!("  - Write notification to inbox");
    println!("  - Schedule verification after 7 days");
    Ok(())
}

/// Verify improvement's actual impact
async fn verify_improvement(
    _ctx: &CommandContext,
    improvement_id: &str,
    window_days: u32,
) -> Result<()> {
    println!("📊 Verifying improvement: {} ({} day window)", improvement_id, window_days);
    println!("⚠️  Not yet implemented - will:");
    println!("  - Compare metrics before vs after");
    println!("  - Calculate actual impact percentage");
    println!("  - Recommend keep/rollback/extend monitoring");
    println!("  - Write verification report to inbox");
    Ok(())
}

/// Show health dashboard
async fn show_dashboard(_ctx: &CommandContext, detailed: bool) -> Result<()> {
    println!("📊 System Health Dashboard\n");

    if detailed {
        println!("⚠️  Detailed mode not yet implemented");
    }

    println!("⚠️  Not yet implemented - will show:");
    println!("  - Overall health score (0-100)");
    println!("  - Success rate trends (improving/degrading/stable)");
    println!("  - Per-agent metrics (if --detailed)");
    println!("  - Per-tool reliability scores");
    println!("  - Recent improvements and their impact");
    Ok(())
}

/// List improvements
async fn list_improvements(
    _ctx: &CommandContext,
    status: Option<String>,
    limit: u32,
) -> Result<()> {
    let status_filter = status.as_deref().unwrap_or("all");
    println!("📋 Listing improvements (status: {}, limit: {})\n", status_filter, limit);

    println!("⚠️  Not yet implemented - will show:");
    println!("  - Improvement ID");
    println!("  - Status (proposed/applied/verified/rejected)");
    println!("  - Description");
    println!("  - Expected vs actual impact");
    println!("  - Applied date (if applicable)");
    Ok(())
}

/// Write notification to inbox-mcp
async fn write_to_inbox(_ctx: &CommandContext, pattern_count: usize) -> Result<()> {
    let mut pool = crate::mcp::McpClientPool::load()?
        .ok_or_else(|| anyhow::anyhow!("No .mcp.json found - inbox-mcp tools required"))?;

    let message = format!(
        "Detected {} failure pattern(s). View details: agent self-heal improvements",
        pattern_count
    );

    pool.call_tool(
        "write_inbox",
        Some(serde_json::json!({
            "message": message,
            "priority": "high",
            "tags": ["self-heal", "patterns"],
            "source": "self-heal",
        })),
    )
    .await
    .context("Failed to write to inbox")?;

    Ok(())
}
