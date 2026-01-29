//! Tier 2: Multi-step sequential benchmark cases
//!
//! These cases test the agent's ability to chain multiple tool calls
//! in sequence to accomplish a task. Each case requires 2-3 tool calls.

use crate::{BenchmarkCase, SuccessCriteria, Tier};
use std::time::Duration;

/// Get all Tier 2 benchmark cases
pub fn all_cases() -> Vec<BenchmarkCase> {
    vec![
        search_and_read_case(),
        list_and_count_case(),
        read_and_edit_case(),
        system_summary_case(),
        find_and_info_case(),
    ]
}

/// T2-01: Search for files then read the first match
fn search_and_read_case() -> BenchmarkCase {
    BenchmarkCase::builder(
        "t2_search_and_read",
        "Search for all .rs files in the agent/src directory, then read the first file you find and tell me what it contains.",
    )
    .name("Search Then Read")
    .tier(Tier::Tier2)
    .expected_tools(vec![
        "mcp__filesystem__search_files",
        "mcp__filesystem__read_file",
    ])
    .success_criteria(SuccessCriteria::all(vec![
        SuccessCriteria::no_errors(),
        SuccessCriteria::tools_called(vec![
            "mcp__filesystem__search_files",
            "mcp__filesystem__read_file",
        ]),
    ]))
    .timeout(Duration::from_secs(60))
    .servers(vec!["filesystem"])
    .description("Tests sequential file search then read capability")
    .build()
}

/// T2-02: List directory then count specific file types
fn list_and_count_case() -> BenchmarkCase {
    BenchmarkCase::builder(
        "t2_list_and_count",
        "List the contents of the mcps/ directory, then get file info for each subdirectory to determine which one has the most files. Report the name and file count.",
    )
    .name("List Then Analyze")
    .tier(Tier::Tier2)
    .expected_tools(vec![
        "mcp__filesystem__list_dir",
        "mcp__filesystem__file_info",
    ])
    .success_criteria(SuccessCriteria::all(vec![
        SuccessCriteria::no_errors(),
        SuccessCriteria::tools_called(vec!["mcp__filesystem__list_dir"]),
    ]))
    .timeout(Duration::from_secs(60))
    .servers(vec!["filesystem"])
    .description("Tests directory listing followed by analysis")
    .build()
}

/// T2-03: Read a file then make an edit
fn read_and_edit_case() -> BenchmarkCase {
    BenchmarkCase::builder(
        "t2_read_and_edit",
        "Read the file benchmarks/binks-bench/test-data/version.txt, then update the version number from 1.0.0 to 1.0.1 using the edit tool.",
    )
    .name("Read Then Edit")
    .tier(Tier::Tier2)
    .expected_tools(vec![
        "mcp__filesystem__read_file",
        "mcp__filesystem__edit_file",
    ])
    .success_criteria(SuccessCriteria::all(vec![
        SuccessCriteria::no_errors(),
        SuccessCriteria::tools_called(vec![
            "mcp__filesystem__read_file",
            "mcp__filesystem__edit_file",
        ]),
    ]))
    .timeout(Duration::from_secs(60))
    .servers(vec!["filesystem"])
    .description("Tests read-then-edit workflow")
    .build()
}

/// T2-04: Get comprehensive system summary
fn system_summary_case() -> BenchmarkCase {
    BenchmarkCase::builder(
        "t2_system_summary",
        "Give me a comprehensive system summary including: the OS name and version, CPU model and core count, and total RAM. Present it in a clear format.",
    )
    .name("System Summary")
    .tier(Tier::Tier2)
    .expected_tools(vec![
        "mcp__sysinfo__get_os_info",
        "mcp__sysinfo__get_cpu_info",
        "mcp__sysinfo__get_memory_info",
    ])
    .success_criteria(SuccessCriteria::all(vec![
        SuccessCriteria::no_errors(),
        SuccessCriteria::tools_called(vec![
            "mcp__sysinfo__get_os_info",
            "mcp__sysinfo__get_cpu_info",
            "mcp__sysinfo__get_memory_info",
        ]),
    ]))
    .timeout(Duration::from_secs(60))
    .servers(vec!["sysinfo"])
    .description("Tests gathering multiple system info types")
    .build()
}

/// T2-05: Find a file then get its info
fn find_and_info_case() -> BenchmarkCase {
    BenchmarkCase::builder(
        "t2_find_and_info",
        "Find all Cargo.toml files in the repository, then get detailed file info (size, modification time) for the root Cargo.toml.",
    )
    .name("Find Then Info")
    .tier(Tier::Tier2)
    .expected_tools(vec![
        "mcp__filesystem__search_files",
        "mcp__filesystem__file_info",
    ])
    .success_criteria(SuccessCriteria::all(vec![
        SuccessCriteria::no_errors(),
        SuccessCriteria::tools_called(vec![
            "mcp__filesystem__search_files",
            "mcp__filesystem__file_info",
        ]),
    ]))
    .timeout(Duration::from_secs(60))
    .servers(vec!["filesystem"])
    .description("Tests file search followed by metadata retrieval")
    .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier2_cases_count() {
        let cases = all_cases();
        assert_eq!(cases.len(), 5, "Should have exactly 5 Tier 2 cases");
    }

    #[test]
    fn test_all_cases_are_tier2() {
        for case in all_cases() {
            assert_eq!(case.tier, Tier::Tier2, "Case {} should be Tier 2", case.id);
        }
    }

    #[test]
    fn test_cases_have_servers() {
        for case in all_cases() {
            assert!(
                case.servers.is_some(),
                "Tier 2 case {} should specify servers for efficiency",
                case.id
            );
        }
    }

    #[test]
    fn test_cases_have_multiple_expected_tools() {
        for case in all_cases() {
            assert!(
                case.expected_tools.len() >= 2,
                "Tier 2 case {} should require at least 2 tools",
                case.id
            );
        }
    }

    #[test]
    fn test_cases_have_reasonable_timeout() {
        for case in all_cases() {
            assert!(
                case.timeout <= Duration::from_secs(120),
                "Tier 2 case {} should have timeout <= 120s",
                case.id
            );
        }
    }
}
