//! Run tracking and analysis subcommands
//!
//! Commands for viewing workflow runs, exporting reports, and tracking improvements.

use clap::{Subcommand, ValueEnum};

#[derive(Subcommand)]
pub enum RunsCommands {
    /// List workflow runs
    List {
        /// Maximum number of runs to show
        #[arg(long, short, default_value = "20")]
        limit: u32,
        /// Filter by workflow name
        #[arg(long, short)]
        workflow: Option<String>,
        /// Filter by status (running, completed, failed, cancelled)
        #[arg(long, short)]
        status: Option<String>,
    },
    /// View details of a specific run
    View {
        /// Run ID (or partial ID prefix)
        id: String,
        /// Include all events
        #[arg(long, short)]
        events: bool,
        /// Show full output (don't truncate)
        #[arg(long, short)]
        full: bool,
    },
    /// Export a run report for analysis
    Export {
        /// Run ID (or partial ID prefix)
        id: String,
        /// Output format
        #[arg(long, short, value_enum, default_value = "markdown")]
        format: ExportFormat,
        /// Output to file instead of stdout
        #[arg(long, short)]
        output: Option<String>,
    },
    /// Compare two runs
    Compare {
        /// First run ID
        run_a: String,
        /// Second run ID
        run_b: String,
    },
    /// Show summary of recent runs
    Summary {
        /// Number of recent runs to summarize
        #[arg(long, short, default_value = "10")]
        last: u32,
        /// Filter by workflow name
        #[arg(long, short)]
        workflow: Option<String>,
    },
    /// Record an improvement based on run analysis
    Improve {
        /// Category of improvement
        #[arg(long, short, value_enum)]
        category: ImprovementCategory,
        /// Description of the improvement
        #[arg(long, short)]
        description: String,
        /// Related run IDs (comma-separated)
        #[arg(long, short)]
        runs: Option<String>,
    },
    /// List recorded improvements
    Improvements {
        /// Filter by status (proposed, applied, verified, rejected)
        #[arg(long, short)]
        status: Option<String>,
        /// Filter by category
        #[arg(long, short, value_enum)]
        category: Option<ImprovementCategory>,
        /// Maximum number to show
        #[arg(long, default_value = "20")]
        limit: u32,
    },
}

#[derive(Clone, Copy, ValueEnum)]
pub enum ExportFormat {
    /// Markdown format (optimized for Claude analysis)
    Markdown,
    /// JSON format (machine-readable)
    Json,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum ImprovementCategory {
    /// Prompt improvements
    Prompt,
    /// Workflow structure improvements
    Workflow,
    /// Agent configuration improvements
    Agent,
    /// Tool usage improvements
    Tool,
    /// Other improvements
    Other,
}

impl From<ImprovementCategory> for crate::db::runs::ImprovementCategory {
    fn from(cat: ImprovementCategory) -> Self {
        match cat {
            ImprovementCategory::Prompt => crate::db::runs::ImprovementCategory::Prompt,
            ImprovementCategory::Workflow => crate::db::runs::ImprovementCategory::Workflow,
            ImprovementCategory::Agent => crate::db::runs::ImprovementCategory::Agent,
            ImprovementCategory::Tool => crate::db::runs::ImprovementCategory::Tool,
            ImprovementCategory::Other => crate::db::runs::ImprovementCategory::Other,
        }
    }
}
