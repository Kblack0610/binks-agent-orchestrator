//! Benchmark entry point for `cargo bench`
//!
//! This file provides the entry point for running Binks agent benchmarks
//! via Cargo's built-in benchmarking infrastructure.
//!
//! ## Usage
//!
//! ```bash
//! # Run all benchmarks
//! cargo bench -p binks-bench
//!
//! # Run with specific filter
//! cargo bench -p binks-bench -- tier1
//! ```
//!
//! Note: For more control over benchmark execution (model selection, baseline comparison),
//! use the `agent bench` CLI command instead.

use binks_bench::cases;
use std::time::Instant;

fn main() {
    println!("Binks Agent Benchmarks");
    println!("======================\n");

    // List available cases
    let all_cases = cases::all_cases();
    println!("Available benchmark cases: {}\n", all_cases.len());

    for case in &all_cases {
        println!(
            "  {} [{}] - {}",
            case.id,
            case.tier,
            case.name
        );
    }

    println!("\n---");
    println!("Note: Full benchmark execution requires the `agent bench` CLI command.");
    println!("This entry point provides case discovery and basic timing.\n");

    // Basic timing test (no actual agent execution)
    let start = Instant::now();
    let _ = cases::all_cases();
    let elapsed = start.elapsed();
    println!("Case loading time: {:?}", elapsed);
}
