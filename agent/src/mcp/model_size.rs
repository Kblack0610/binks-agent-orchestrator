//! Model size detection and classification
//!
//! Parses model names to determine parameter count and classifies them
//! for automatic MCP tier filtering.

use regex::Regex;
use std::sync::LazyLock;

/// Model size classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelSize {
    Small,   // <= small_threshold (default 8B)
    Medium,  // <= medium_threshold (default 32B)
    Large,   // > medium_threshold
    Unknown, // Could not determine size
}

impl ModelSize {
    /// Get the default max tier for this size class
    pub fn default_max_tier(self) -> u8 {
        match self {
            ModelSize::Small => 1,   // Essential only
            ModelSize::Medium => 2,  // Essential + Standard
            ModelSize::Large => 3,   // All except agent-only
            ModelSize::Unknown => 1, // Conservative - treat as small
        }
    }
}

static SIZE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d+)[bB]").expect("Invalid regex"));

/// Parse model size from an Ollama model name
///
/// Uses default thresholds: small <= 8B, medium <= 32B
///
/// # Examples
///
/// ```
/// use agent::mcp::model_size::parse_model_size;
///
/// assert_eq!(parse_model_size("llama3.1:8b").default_max_tier(), 1);  // Small
/// assert_eq!(parse_model_size("qwen3-coder:30b").default_max_tier(), 2);  // Medium
/// assert_eq!(parse_model_size("llama3.1:70b").default_max_tier(), 3);  // Large
/// ```
pub fn parse_model_size(model: &str) -> ModelSize {
    parse_model_size_with_thresholds(model, 8, 32)
}

/// Parse model size with custom thresholds
///
/// # Arguments
/// * `model` - The model name (e.g., "qwen3-coder:30b")
/// * `small_threshold` - Upper bound for "small" (inclusive, in billions)
/// * `medium_threshold` - Upper bound for "medium" (inclusive, in billions)
pub fn parse_model_size_with_thresholds(
    model: &str,
    small_threshold: u32,
    medium_threshold: u32,
) -> ModelSize {
    match extract_size_billions(model) {
        Some(size) => classify_size(size, small_threshold, medium_threshold),
        None => ModelSize::Unknown,
    }
}

/// Extract the parameter count (in billions) from model name
fn extract_size_billions(model: &str) -> Option<u32> {
    // First try after the colon (most common: "model:30b")
    if let Some(tag) = model.split(':').nth(1) {
        if let Some(size) = extract_from_string(tag) {
            return Some(size);
        }
    }

    // Fallback: search the entire model name
    extract_from_string(model)
}

fn extract_from_string(s: &str) -> Option<u32> {
    SIZE_REGEX
        .captures(s)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok())
}

fn classify_size(size_billions: u32, small_threshold: u32, medium_threshold: u32) -> ModelSize {
    if size_billions <= small_threshold {
        ModelSize::Small
    } else if size_billions <= medium_threshold {
        ModelSize::Medium
    } else {
        ModelSize::Large
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_common_models() {
        // Small models
        assert_eq!(parse_model_size("llama3.1:8b"), ModelSize::Small);
        assert_eq!(parse_model_size("mistral:7b"), ModelSize::Small);
        assert_eq!(parse_model_size("phi3:3b"), ModelSize::Small);

        // Medium models
        assert_eq!(parse_model_size("qwen3-coder:30b"), ModelSize::Medium);
        assert_eq!(parse_model_size("phi4:14b"), ModelSize::Medium);
        assert_eq!(parse_model_size("codellama:13b"), ModelSize::Medium);

        // Large models
        assert_eq!(parse_model_size("llama3.1:70b"), ModelSize::Large);
        assert_eq!(parse_model_size("deepseek-r1:671b"), ModelSize::Large);
        assert_eq!(parse_model_size("qwen:72b"), ModelSize::Large);
    }

    #[test]
    fn test_edge_cases() {
        // Boundary: exactly at threshold
        assert_eq!(parse_model_size("model:8b"), ModelSize::Small);
        assert_eq!(parse_model_size("model:32b"), ModelSize::Medium);
        assert_eq!(parse_model_size("model:33b"), ModelSize::Large);

        // Unknown - no size info
        assert_eq!(parse_model_size("gpt-4"), ModelSize::Unknown);
        assert_eq!(parse_model_size("claude-3"), ModelSize::Unknown);
        assert_eq!(parse_model_size("custom-model"), ModelSize::Unknown);
    }

    #[test]
    fn test_format_variations() {
        // With suffix after size
        assert_eq!(parse_model_size("mistral:7b-instruct"), ModelSize::Small);
        assert_eq!(parse_model_size("llama3.1:8b-q4_0"), ModelSize::Small);

        // Uppercase B
        assert_eq!(parse_model_size("model:8B"), ModelSize::Small);
        assert_eq!(parse_model_size("model:30B"), ModelSize::Medium);
    }

    #[test]
    fn test_custom_thresholds() {
        // With thresholds: small <= 7, medium <= 14
        assert_eq!(
            parse_model_size_with_thresholds("model:7b", 7, 14),
            ModelSize::Small
        );
        assert_eq!(
            parse_model_size_with_thresholds("model:8b", 7, 14),
            ModelSize::Medium
        );
        assert_eq!(
            parse_model_size_with_thresholds("model:14b", 7, 14),
            ModelSize::Medium
        );
        assert_eq!(
            parse_model_size_with_thresholds("model:15b", 7, 14),
            ModelSize::Large
        );
    }

    #[test]
    fn test_default_tiers() {
        assert_eq!(ModelSize::Small.default_max_tier(), 1);
        assert_eq!(ModelSize::Medium.default_max_tier(), 2);
        assert_eq!(ModelSize::Large.default_max_tier(), 3);
        assert_eq!(ModelSize::Unknown.default_max_tier(), 1); // Conservative
    }
}
