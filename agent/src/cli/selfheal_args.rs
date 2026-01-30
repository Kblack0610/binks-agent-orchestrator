//! Self-healing workflow subcommands
//!
//! Commands for automated health analysis and improvement proposals.

use clap::Subcommand;

#[derive(Subcommand)]
pub enum SelfHealCommands {
    /// Detect failure patterns and propose improvements (default action)
    Detect {
        /// Lookback period (e.g., "-7d", "-24h", "-1w")
        #[arg(long, default_value = "-7d")]
        since: String,
        /// Minimum number of occurrences to report a pattern
        #[arg(long, default_value = "3")]
        min_occurrences: usize,
        /// Minimum confidence threshold (0.0-1.0)
        #[arg(long, default_value = "0.75")]
        confidence: f64,
    },
    /// Show details of a detected pattern
    Show {
        /// Pattern ID to show details for
        pattern_id: String,
    },
    /// Test an improvement in simulation mode
    Test {
        /// Improvement ID to test
        improvement_id: String,
    },
    /// Apply an approved improvement
    Apply {
        /// Improvement ID to apply
        improvement_id: String,
        /// Skip confirmation prompt
        #[arg(long, short)]
        yes: bool,
    },
    /// Verify an improvement's actual impact
    Verify {
        /// Improvement ID to verify
        improvement_id: String,
        /// Measurement window in days
        #[arg(long, default_value = "7")]
        window_days: u32,
    },
    /// Show health dashboard with system metrics
    Dashboard {
        /// Include detailed per-agent metrics
        #[arg(long)]
        detailed: bool,
    },
    /// List all improvements (proposed, applied, verified)
    Improvements {
        /// Filter by status (proposed, applied, verified, rejected)
        #[arg(long, short)]
        status: Option<String>,
        /// Maximum number to show
        #[arg(long, default_value = "20")]
        limit: u32,
    },
}
