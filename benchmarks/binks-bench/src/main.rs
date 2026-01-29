//! Binks Benchmark CLI
//!
//! Standalone benchmark runner for the Binks agent.
//!
//! Usage:
//!   cargo run -p binks-bench -- [OPTIONS]
//!
//! Examples:
//!   cargo run -p binks-bench -- --tier 1           # Run Tier 1 benchmarks
//!   cargo run -p binks-bench -- --case t1_read_file # Run specific case
//!   cargo run -p binks-bench -- --baseline         # Establish baseline
//!   cargo run -p binks-bench -- --compare model    # Compare against baseline

use anyhow::Result;
use binks_bench::{
    cases, Baseline, BenchmarkCase, BenchmarkRunner, OutputFormat, Reporter, RunnerConfig, Tier,
};
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "binks-bench")]
#[command(about = "Binks agent benchmark runner")]
struct Cli {
    /// Run specific tier only (1, 2, 3, or 4 for Platform)
    #[arg(short, long)]
    tier: Option<u8>,

    /// Run a specific benchmark case by ID
    #[arg(short, long)]
    case: Option<String>,

    /// Establish a new baseline from this run
    #[arg(long)]
    baseline: bool,

    /// Compare results against a baseline model
    #[arg(long)]
    compare: Option<String>,

    /// Output format: terminal, markdown, json, csv
    #[arg(short, long, default_value = "terminal")]
    output: String,

    /// Ollama server URL
    #[arg(long, env = "OLLAMA_URL", default_value = "http://localhost:11434")]
    ollama_url: String,

    /// Model to use for benchmarks
    #[arg(short, long, env = "OLLAMA_MODEL", default_value = "llama3.1:8b")]
    model: String,

    /// MCP config path (uses default if not specified)
    #[arg(long)]
    mcp_config: Option<String>,

    /// Print verbose output during benchmark execution
    #[arg(short, long)]
    verbose: bool,

    /// List available benchmark cases and exit
    #[arg(long)]
    list: bool,
}

fn init_tracing(verbose: bool) {
    let level = if verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level.to_string()));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    // Handle --list flag
    if cli.list {
        println!("Available benchmark cases:\n");
        for case in cases::all_cases() {
            println!("  {} ({}) - {}", case.id, case.tier, case.name);
        }
        return Ok(());
    }

    run_bench(cli).await
}

async fn run_bench(cli: Cli) -> Result<()> {
    let format: OutputFormat = cli.output.parse().unwrap_or(OutputFormat::Terminal);
    let reporter = Reporter::new(format);

    // Create runner config
    let runner_config = RunnerConfig {
        ollama_url: cli.ollama_url.clone(),
        model: cli.model.clone(),
        mcp_config: cli.mcp_config,
        verbose: cli.verbose,
    };
    let runner = BenchmarkRunner::new(runner_config);

    // Collect test cases based on filters
    let all_cases = cases::all_cases();
    let cases_to_run: Vec<BenchmarkCase> = if let Some(case_id) = &cli.case {
        // Run specific case
        all_cases
            .into_iter()
            .filter(|c| c.id == *case_id)
            .collect()
    } else if let Some(tier_num) = cli.tier {
        // Run specific tier
        let target_tier = match tier_num {
            1 => Tier::Tier1,
            2 => Tier::Tier2,
            3 => Tier::Tier3,
            4 => Tier::Platform,
            _ => {
                anyhow::bail!(
                    "Invalid tier: {}. Valid tiers are 1, 2, 3, or 4 (Platform)",
                    tier_num
                );
            }
        };
        all_cases
            .into_iter()
            .filter(|c| c.tier == target_tier)
            .collect()
    } else {
        // Run all cases
        all_cases
    };

    if cases_to_run.is_empty() {
        println!("No benchmark cases match the specified filters.");
        if let Some(case_id) = &cli.case {
            println!("Case '{}' not found. Available cases:", case_id);
            for c in cases::all_cases() {
                println!("  - {} ({})", c.id, c.tier);
            }
        }
        return Ok(());
    }

    // Print header if terminal format
    if matches!(format, OutputFormat::Terminal) {
        println!("\n=== Binks Agent Benchmarks ===");
        println!("Model: {}", cli.model);
        println!("Cases to run: {}", cases_to_run.len());
        println!();
    }

    // Run benchmarks
    let results = runner.run_all(&cases_to_run).await?;
    let summary = runner.summarize(&results, &cases_to_run);

    // Output results
    println!("{}", reporter.summary(&summary));

    if cli.verbose {
        println!("{}", reporter.results(&results));
    }

    // Handle baseline operations
    if cli.baseline {
        let baseline = Baseline::from_results(cli.model.clone(), &results);
        let baseline_path = Baseline::default_path(&cli.model);

        baseline.save(&baseline_path)?;

        if matches!(format, OutputFormat::Terminal) {
            println!("\nBaseline saved to: {}", baseline_path.display());
        }
    }

    // Handle comparison
    if let Some(baseline_model) = cli.compare {
        let baseline_path = Baseline::default_path(&baseline_model);

        if baseline_path.exists() {
            let baseline = Baseline::load(&baseline_path)?;
            let regression_report = baseline.compare(&results);

            println!("{}", reporter.regression(&regression_report));

            if regression_report.has_severe_regressions() {
                std::process::exit(1);
            }
        } else {
            println!(
                "Warning: No baseline found for model '{}' at {}",
                baseline_model,
                baseline_path.display()
            );
            println!("Run with --baseline first to establish a baseline.");
        }
    }

    // Exit with error if overall pass rate is below threshold
    if summary.overall_pass_rate < 50.0 {
        if matches!(format, OutputFormat::Terminal) {
            println!("\nBenchmark failed: pass rate below 50%");
        }
        std::process::exit(1);
    }

    Ok(())
}
