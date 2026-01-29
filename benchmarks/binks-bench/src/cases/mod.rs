//! Benchmark test cases organized by tier
//!
//! - **Tier 1**: Simple single-tool tasks
//! - **Tier 2**: Multi-step sequential tasks
//! - **Tier 3**: Complex reasoning tasks
//! - **Platform**: Real-world platform repo tasks

pub mod tier1;
pub mod tier2;
pub mod tier3;

use crate::{BenchmarkCase, Tier};

/// Get all benchmark cases
pub fn all_cases() -> Vec<BenchmarkCase> {
    let mut cases = Vec::new();
    cases.extend(tier1::all_cases());
    cases.extend(tier2::all_cases());
    cases.extend(tier3::all_cases());
    cases
}

/// Get cases for a specific tier
pub fn cases_for_tier(tier: Tier) -> Vec<BenchmarkCase> {
    match tier {
        Tier::Tier1 => tier1::all_cases(),
        Tier::Tier2 => tier2::all_cases(),
        Tier::Tier3 => tier3::all_cases(),
        Tier::Platform => vec![], // TODO: platform::all_cases()
    }
}

/// Get a specific case by ID
pub fn get_case(id: &str) -> Option<BenchmarkCase> {
    all_cases().into_iter().find(|c| c.id == id)
}

/// List all case IDs
pub fn list_case_ids() -> Vec<String> {
    all_cases().into_iter().map(|c| c.id).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_cases_not_empty() {
        let cases = all_cases();
        assert!(!cases.is_empty(), "Should have at least Tier 1 cases");
    }

    #[test]
    fn test_cases_have_unique_ids() {
        let cases = all_cases();
        let mut ids: Vec<_> = cases.iter().map(|c| &c.id).collect();
        ids.sort();
        let original_len = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), original_len, "All case IDs should be unique");
    }

    #[test]
    fn test_get_case_by_id() {
        let cases = all_cases();
        if let Some(first) = cases.first() {
            let found = get_case(&first.id);
            assert!(found.is_some());
            assert_eq!(found.unwrap().id, first.id);
        }
    }
}
