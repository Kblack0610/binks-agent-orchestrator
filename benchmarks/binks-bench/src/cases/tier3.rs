//! Tier 3: Complex reasoning benchmark cases
//!
//! These cases test the agent's ability to perform complex reasoning,
//! multi-step analysis, and synthesize information from multiple sources.
//! Each case requires significant interpretation and decision-making.

use crate::{BenchmarkCase, SuccessCriteria, Tier};
use std::time::Duration;

/// Get all Tier 3 benchmark cases
pub fn all_cases() -> Vec<BenchmarkCase> {
    vec![
        explain_code_case(),
        find_todos_case(),
        system_health_case(),
        git_status_case(),
        inbox_roundtrip_case(),
    ]
}

/// T3-01: Explain code structure and purpose
fn explain_code_case() -> BenchmarkCase {
    BenchmarkCase::builder(
        "t3_explain_code",
        "Read the file agent/src/agent/mod.rs and explain: 1) What is the main struct or type defined? 2) What are its key fields or methods? 3) What is its purpose in the codebase? Provide a concise summary.",
    )
    .name("Explain Code")
    .tier(Tier::Tier3)
    .expected_tools(vec!["mcp__filesystem__read_file"])
    .success_criteria(SuccessCriteria::all(vec![
        SuccessCriteria::no_errors(),
        SuccessCriteria::tools_called(vec!["mcp__filesystem__read_file"]),
    ]))
    .timeout(Duration::from_secs(90))
    .servers(vec!["filesystem"])
    .description("Tests code comprehension and explanation capability")
    .build()
}

/// T3-02: Find and categorize TODO comments across codebase
fn find_todos_case() -> BenchmarkCase {
    BenchmarkCase::builder(
        "t3_find_todos",
        "Search the agent/ directory for any TODO, FIXME, or HACK comments in the source code. List each one with its file location and categorize them by type (e.g., bug fixes, improvements, technical debt). How many total did you find?",
    )
    .name("Find TODOs")
    .tier(Tier::Tier3)
    .expected_tools(vec![
        "mcp__filesystem__search_files",
        "mcp__filesystem__read_file",
    ])
    .success_criteria(SuccessCriteria::all(vec![
        SuccessCriteria::no_errors(),
        SuccessCriteria::any(vec![
            SuccessCriteria::tools_called(vec!["mcp__filesystem__search_files"]),
            SuccessCriteria::tools_called(vec!["mcp__filesystem__read_file"]),
        ]),
    ]))
    .timeout(Duration::from_secs(120))
    .servers(vec!["filesystem"])
    .description("Tests codebase analysis and categorization capability")
    .build()
}

/// T3-03: Assess system health and report concerns
fn system_health_case() -> BenchmarkCase {
    BenchmarkCase::builder(
        "t3_system_health",
        "Perform a system health check: get memory usage, CPU info, and disk space. Analyze the results and report any potential concerns (e.g., high memory usage >80%, low disk space <10GB free). Provide a health score from 1-10.",
    )
    .name("System Health")
    .tier(Tier::Tier3)
    .expected_tools(vec![
        "mcp__sysinfo__get_memory_info",
        "mcp__sysinfo__get_cpu_info",
        "mcp__sysinfo__get_disk_info",
    ])
    .success_criteria(SuccessCriteria::all(vec![
        SuccessCriteria::no_errors(),
        SuccessCriteria::tools_called(vec![
            "mcp__sysinfo__get_memory_info",
            "mcp__sysinfo__get_cpu_info",
            "mcp__sysinfo__get_disk_info",
        ]),
    ]))
    .timeout(Duration::from_secs(60))
    .servers(vec!["sysinfo"])
    .description("Tests system analysis and health assessment capability")
    .build()
}

/// T3-04: Analyze git repository status
fn git_status_case() -> BenchmarkCase {
    BenchmarkCase::builder(
        "t3_git_status",
        "Check the git status of this repository. Report: 1) Current branch name, 2) Number of uncommitted changes (if any), 3) Summary of recent commits (last 3). Use the git command to get this information.",
    )
    .name("Git Status")
    .tier(Tier::Tier3)
    .expected_tools(vec!["mcp__exec__run_command"])
    .success_criteria(SuccessCriteria::all(vec![
        SuccessCriteria::no_errors(),
        SuccessCriteria::tools_called(vec!["mcp__exec__run_command"]),
    ]))
    .timeout(Duration::from_secs(60))
    .servers(vec!["exec"])
    .description("Tests git repository analysis capability")
    .build()
}

/// T3-05: Inbox write and read roundtrip
fn inbox_roundtrip_case() -> BenchmarkCase {
    BenchmarkCase::builder(
        "t3_inbox_roundtrip",
        "Write a test message to the inbox with source 'benchmark', priority 'normal', and tag 'test'. The message should say 'Benchmark test at [current time]'. Then read the inbox to verify the message was written. Report success or failure.",
    )
    .name("Inbox Roundtrip")
    .tier(Tier::Tier3)
    .expected_tools(vec![
        "mcp__inbox__write_inbox",
        "mcp__inbox__read_inbox",
    ])
    .success_criteria(SuccessCriteria::all(vec![
        SuccessCriteria::no_errors(),
        SuccessCriteria::tools_called(vec![
            "mcp__inbox__write_inbox",
            "mcp__inbox__read_inbox",
        ]),
    ]))
    .timeout(Duration::from_secs(60))
    .servers(vec!["inbox"])
    .description("Tests inbox write/read roundtrip capability")
    .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier3_cases_count() {
        let cases = all_cases();
        assert_eq!(cases.len(), 5, "Should have exactly 5 Tier 3 cases");
    }

    #[test]
    fn test_all_cases_are_tier3() {
        for case in all_cases() {
            assert_eq!(case.tier, Tier::Tier3, "Case {} should be Tier 3", case.id);
        }
    }

    #[test]
    fn test_cases_have_servers() {
        for case in all_cases() {
            assert!(
                case.servers.is_some(),
                "Tier 3 case {} should specify servers for efficiency",
                case.id
            );
        }
    }

    #[test]
    fn test_cases_have_expected_tools() {
        for case in all_cases() {
            assert!(
                !case.expected_tools.is_empty(),
                "Tier 3 case {} should have expected tools",
                case.id
            );
        }
    }

    #[test]
    fn test_cases_have_reasonable_timeout() {
        for case in all_cases() {
            assert!(
                case.timeout <= Duration::from_secs(180),
                "Tier 3 case {} should have timeout <= 180s",
                case.id
            );
        }
    }

    #[test]
    fn test_tier3_timeouts_are_longer() {
        // Tier 3 cases should generally have longer timeouts than Tier 1
        for case in all_cases() {
            assert!(
                case.timeout >= Duration::from_secs(60),
                "Tier 3 case {} should have timeout >= 60s for complex reasoning",
                case.id
            );
        }
    }
}
