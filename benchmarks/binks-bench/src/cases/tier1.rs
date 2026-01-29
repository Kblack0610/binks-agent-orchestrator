//! Tier 1: Simple single-tool benchmark cases
//!
//! These cases test basic tool usage with minimal reasoning required.
//! Each case should require exactly one tool call to complete successfully.

use crate::{BenchmarkCase, SuccessCriteria, Tier};
use std::time::Duration;

/// Get all Tier 1 benchmark cases
pub fn all_cases() -> Vec<BenchmarkCase> {
    vec![
        read_file_case(),
        list_directory_case(),
        system_info_case(),
        search_files_case(),
        file_info_case(),
        memory_info_case(),
        cpu_info_case(),
        disk_info_case(),
    ]
}

/// T1-01: Read a specific file
fn read_file_case() -> BenchmarkCase {
    BenchmarkCase::builder(
        "t1_read_file",
        "Read the file at agent/src/lib.rs and tell me what crate it declares (the first line after the doc comment).",
    )
    .name("Read File")
    .tier(Tier::Tier1)
    .expected_tools(vec!["mcp__filesystem__read_file"])
    .success_criteria(SuccessCriteria::all(vec![
        SuccessCriteria::no_errors(),
        SuccessCriteria::tools_called(vec!["mcp__filesystem__read_file"]),
    ]))
    .timeout(Duration::from_secs(30))
    .servers(vec!["filesystem"])
    .description("Tests basic file reading capability")
    .build()
}

/// T1-02: List directory contents
fn list_directory_case() -> BenchmarkCase {
    BenchmarkCase::builder(
        "t1_list_dir",
        "List the contents of the mcps/ directory and tell me how many subdirectories it contains.",
    )
    .name("List Directory")
    .tier(Tier::Tier1)
    .expected_tools(vec!["mcp__filesystem__list_dir"])
    .success_criteria(SuccessCriteria::all(vec![
        SuccessCriteria::no_errors(),
        SuccessCriteria::tools_called(vec!["mcp__filesystem__list_dir"]),
    ]))
    .timeout(Duration::from_secs(30))
    .servers(vec!["filesystem"])
    .description("Tests directory listing capability")
    .build()
}

/// T1-03: Get system information
fn system_info_case() -> BenchmarkCase {
    BenchmarkCase::builder(
        "t1_sysinfo",
        "What operating system is this machine running? Include the OS name and version.",
    )
    .name("System Info")
    .tier(Tier::Tier1)
    .expected_tools(vec!["mcp__sysinfo__get_os_info"])
    .success_criteria(SuccessCriteria::all(vec![
        SuccessCriteria::no_errors(),
        SuccessCriteria::tools_called(vec!["mcp__sysinfo__get_os_info"]),
        SuccessCriteria::contains_text("Linux"),
    ]))
    .timeout(Duration::from_secs(30))
    .servers(vec!["sysinfo"])
    .description("Tests OS information retrieval")
    .build()
}

/// T1-04: Search for files by pattern
fn search_files_case() -> BenchmarkCase {
    BenchmarkCase::builder(
        "t1_search_files",
        "Find all Cargo.toml files in the current directory tree. How many are there?",
    )
    .name("Search Files")
    .tier(Tier::Tier1)
    .expected_tools(vec!["mcp__filesystem__search_files"])
    .success_criteria(SuccessCriteria::all(vec![
        SuccessCriteria::no_errors(),
        SuccessCriteria::tools_called(vec!["mcp__filesystem__search_files"]),
    ]))
    .timeout(Duration::from_secs(30))
    .servers(vec!["filesystem"])
    .description("Tests file search by glob pattern")
    .build()
}

/// T1-05: Get file information
fn file_info_case() -> BenchmarkCase {
    BenchmarkCase::builder(
        "t1_file_info",
        "What is the size of the file Cargo.toml in the root directory?",
    )
    .name("File Info")
    .tier(Tier::Tier1)
    .expected_tools(vec!["mcp__filesystem__file_info"])
    .success_criteria(SuccessCriteria::all(vec![
        SuccessCriteria::no_errors(),
        SuccessCriteria::tools_called(vec!["mcp__filesystem__file_info"]),
    ]))
    .timeout(Duration::from_secs(30))
    .servers(vec!["filesystem"])
    .description("Tests file metadata retrieval")
    .build()
}

/// T1-06: Get memory information
fn memory_info_case() -> BenchmarkCase {
    BenchmarkCase::builder(
        "t1_memory_info",
        "How much total RAM does this system have? Report it in gigabytes.",
    )
    .name("Memory Info")
    .tier(Tier::Tier1)
    .expected_tools(vec!["mcp__sysinfo__get_memory_info"])
    .success_criteria(SuccessCriteria::all(vec![
        SuccessCriteria::no_errors(),
        SuccessCriteria::tools_called(vec!["mcp__sysinfo__get_memory_info"]),
    ]))
    .timeout(Duration::from_secs(30))
    .servers(vec!["sysinfo"])
    .description("Tests memory information retrieval")
    .build()
}

/// T1-07: Get CPU information
fn cpu_info_case() -> BenchmarkCase {
    BenchmarkCase::builder(
        "t1_cpu_info",
        "What CPU model is this machine using? Include the number of cores.",
    )
    .name("CPU Info")
    .tier(Tier::Tier1)
    .expected_tools(vec!["mcp__sysinfo__get_cpu_info"])
    .success_criteria(SuccessCriteria::all(vec![
        SuccessCriteria::no_errors(),
        SuccessCriteria::tools_called(vec!["mcp__sysinfo__get_cpu_info"]),
    ]))
    .timeout(Duration::from_secs(30))
    .servers(vec!["sysinfo"])
    .description("Tests CPU information retrieval")
    .build()
}

/// T1-08: Get disk information
fn disk_info_case() -> BenchmarkCase {
    BenchmarkCase::builder(
        "t1_disk_info",
        "How much disk space is available on the root partition?",
    )
    .name("Disk Info")
    .tier(Tier::Tier1)
    .expected_tools(vec!["mcp__sysinfo__get_disk_info"])
    .success_criteria(SuccessCriteria::all(vec![
        SuccessCriteria::no_errors(),
        SuccessCriteria::tools_called(vec!["mcp__sysinfo__get_disk_info"]),
    ]))
    .timeout(Duration::from_secs(30))
    .servers(vec!["sysinfo"])
    .description("Tests disk information retrieval")
    .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier1_cases_count() {
        let cases = all_cases();
        assert!(cases.len() >= 5, "Should have at least 5 Tier 1 cases");
    }

    #[test]
    fn test_all_cases_are_tier1() {
        for case in all_cases() {
            assert_eq!(case.tier, Tier::Tier1, "Case {} should be Tier 1", case.id);
        }
    }

    #[test]
    fn test_cases_have_servers() {
        for case in all_cases() {
            assert!(
                case.servers.is_some(),
                "Tier 1 case {} should specify servers for efficiency",
                case.id
            );
        }
    }

    #[test]
    fn test_cases_have_expected_tools() {
        for case in all_cases() {
            assert!(
                !case.expected_tools.is_empty(),
                "Tier 1 case {} should have expected tools",
                case.id
            );
        }
    }

    #[test]
    fn test_cases_have_reasonable_timeout() {
        for case in all_cases() {
            assert!(
                case.timeout <= Duration::from_secs(60),
                "Tier 1 case {} should have timeout <= 60s",
                case.id
            );
        }
    }
}
