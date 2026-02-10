//! Centralized model family detection and capabilities
//!
//! This module provides model family detection based on model name patterns.
//! It does NOT define default models - model selection comes from `.agent.toml` only.
//!
//! # Design Principles
//!
//! - No hardcoded default model names
//! - Pattern-based family detection (e.g., "qwen", "llama3", not "qwen3:14b")
//! - LiteLLM-ready abstraction for future provider migration

use serde::{Deserialize, Serialize};

/// Function call format supported by different model families
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FunctionCallFormat {
    /// Native JSON function calling (OpenAI-compatible)
    #[default]
    Native,
    /// XML-based tool calling (Llama3 style)
    Xml,
    /// Hermes-style function calling
    Hermes,
}

/// Capabilities detected for a model family
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    /// Whether the model supports tool/function calling
    pub tool_calling: bool,
    /// Whether this is a reasoning/thinking model (like DeepSeek-R1)
    pub thinking: bool,
    /// Format for function calls
    pub function_format: FunctionCallFormat,
    /// Whether the model supports vision/image inputs
    pub vision: bool,
}

impl Default for ModelCapabilities {
    fn default() -> Self {
        Self {
            tool_calling: false,
            thinking: false,
            function_format: FunctionCallFormat::Native,
            vision: false,
        }
    }
}

/// A model family definition with pattern matching and default capabilities
#[derive(Debug, Clone)]
pub struct ModelFamily {
    /// Pattern prefix to match against model names (lowercase)
    pub pattern: &'static str,
    /// Default capabilities for this family
    pub capabilities: ModelCapabilities,
}

/// Built-in model family definitions
///
/// These patterns are matched against model names (case-insensitive).
/// Order matters - more specific patterns should come first.
const BUILTIN_FAMILIES: &[ModelFamily] = &[
    // Reasoning models - check first as they may contain other family names
    ModelFamily {
        pattern: "deepseek-r1",
        capabilities: ModelCapabilities {
            tool_calling: false,
            thinking: true,
            function_format: FunctionCallFormat::Native,
            vision: false,
        },
    },
    ModelFamily {
        pattern: "qwq",
        capabilities: ModelCapabilities {
            tool_calling: false,
            thinking: true,
            function_format: FunctionCallFormat::Native,
            vision: false,
        },
    },
    // Qwen family - good tool support
    ModelFamily {
        pattern: "qwen",
        capabilities: ModelCapabilities {
            tool_calling: true,
            thinking: false,
            function_format: FunctionCallFormat::Native,
            vision: false,
        },
    },
    // Llama3 family - XML-based tools
    ModelFamily {
        pattern: "llama3",
        capabilities: ModelCapabilities {
            tool_calling: true,
            thinking: false,
            function_format: FunctionCallFormat::Xml,
            vision: false,
        },
    },
    ModelFamily {
        pattern: "llama-3",
        capabilities: ModelCapabilities {
            tool_calling: true,
            thinking: false,
            function_format: FunctionCallFormat::Xml,
            vision: false,
        },
    },
    // Mistral/Mixtral family
    ModelFamily {
        pattern: "mistral",
        capabilities: ModelCapabilities {
            tool_calling: true,
            thinking: false,
            function_format: FunctionCallFormat::Native,
            vision: false,
        },
    },
    ModelFamily {
        pattern: "mixtral",
        capabilities: ModelCapabilities {
            tool_calling: true,
            thinking: false,
            function_format: FunctionCallFormat::Native,
            vision: false,
        },
    },
    // Phi family
    ModelFamily {
        pattern: "phi",
        capabilities: ModelCapabilities {
            tool_calling: true,
            thinking: false,
            function_format: FunctionCallFormat::Native,
            vision: false,
        },
    },
    // Gemma family
    ModelFamily {
        pattern: "gemma",
        capabilities: ModelCapabilities {
            tool_calling: true,
            thinking: false,
            function_format: FunctionCallFormat::Native,
            vision: false,
        },
    },
    // Command-R family
    ModelFamily {
        pattern: "command-r",
        capabilities: ModelCapabilities {
            tool_calling: true,
            thinking: false,
            function_format: FunctionCallFormat::Native,
            vision: false,
        },
    },
    // Hermes family - specialized format
    ModelFamily {
        pattern: "hermes",
        capabilities: ModelCapabilities {
            tool_calling: true,
            thinking: false,
            function_format: FunctionCallFormat::Hermes,
            vision: false,
        },
    },
    // Vision models
    ModelFamily {
        pattern: "llava",
        capabilities: ModelCapabilities {
            tool_calling: false,
            thinking: false,
            function_format: FunctionCallFormat::Native,
            vision: true,
        },
    },
    ModelFamily {
        pattern: "bakllava",
        capabilities: ModelCapabilities {
            tool_calling: false,
            thinking: false,
            function_format: FunctionCallFormat::Native,
            vision: true,
        },
    },
];

/// Detect capabilities for a model based on its name
///
/// Returns detected capabilities or defaults if no family matches.
///
/// # Arguments
///
/// * `model` - Model name (e.g., "qwen3-coder:30b", "llama3.1:70b")
///
/// # Examples
///
/// ```
/// use agent::models::detect_capabilities;
///
/// let caps = detect_capabilities("qwen3-coder:30b");
/// assert!(caps.tool_calling);
/// assert!(!caps.thinking);
/// ```
pub fn detect_capabilities(model: &str) -> ModelCapabilities {
    let model_lower = model.to_lowercase();

    // Check for reasoning model indicators anywhere in the name
    if is_reasoning_model(&model_lower) {
        return ModelCapabilities {
            tool_calling: false,
            thinking: true,
            function_format: FunctionCallFormat::Native,
            vision: false,
        };
    }

    // Match against known families
    for family in BUILTIN_FAMILIES {
        if model_lower.contains(family.pattern) {
            return family.capabilities.clone();
        }
    }

    // Default: unknown model, assume no special capabilities
    ModelCapabilities::default()
}

/// Check if a model is a reasoning/thinking model
///
/// These models use chain-of-thought reasoning and typically
/// don't support tool calling.
pub fn is_reasoning_model(model: &str) -> bool {
    let model_lower = model.to_lowercase();
    model_lower.contains("deepseek-r1")
        || model_lower.contains("qwq")
        || model_lower.contains("o1")
        || model_lower.contains("reasoning")
        || model_lower.contains("-r1")
}

/// Check if a model likely supports tool calling
///
/// This is a convenience wrapper around `detect_capabilities`.
pub fn supports_tool_calling(model: &str) -> bool {
    detect_capabilities(model).tool_calling
}

/// Get the function call format for a model
///
/// This is a convenience wrapper around `detect_capabilities`.
pub fn get_function_format(model: &str) -> FunctionCallFormat {
    detect_capabilities(model).function_format
}

/// List of model family patterns known to have good tool support
///
/// Use this for filtering or displaying available families.
pub fn tool_capable_families() -> Vec<&'static str> {
    BUILTIN_FAMILIES
        .iter()
        .filter(|f| f.capabilities.tool_calling)
        .map(|f| f.pattern)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qwen_detection() {
        let caps = detect_capabilities("qwen3-coder:30b");
        assert!(caps.tool_calling);
        assert!(!caps.thinking);
        assert_eq!(caps.function_format, FunctionCallFormat::Native);
    }

    #[test]
    fn test_llama3_detection() {
        let caps = detect_capabilities("llama3.1:70b");
        assert!(caps.tool_calling);
        assert!(!caps.thinking);
        assert_eq!(caps.function_format, FunctionCallFormat::Xml);
    }

    #[test]
    fn test_deepseek_r1_detection() {
        let caps = detect_capabilities("deepseek-r1:14b");
        assert!(!caps.tool_calling);
        assert!(caps.thinking);
    }

    #[test]
    fn test_reasoning_model_detection() {
        assert!(is_reasoning_model("deepseek-r1:14b"));
        assert!(is_reasoning_model("qwq:32b"));
        assert!(is_reasoning_model("some-model-r1"));
        assert!(!is_reasoning_model("qwen3-coder:30b"));
        assert!(!is_reasoning_model("llama3.1:70b"));
    }

    #[test]
    fn test_unknown_model_defaults() {
        let caps = detect_capabilities("unknown-model:7b");
        assert!(!caps.tool_calling);
        assert!(!caps.thinking);
        assert!(!caps.vision);
    }

    #[test]
    fn test_tool_capable_families() {
        let families = tool_capable_families();
        assert!(families.contains(&"qwen"));
        assert!(families.contains(&"llama3"));
        assert!(families.contains(&"mistral"));
        assert!(!families.contains(&"deepseek-r1"));
    }

    #[test]
    fn test_vision_model_detection() {
        let caps = detect_capabilities("llava:13b");
        assert!(caps.vision);
        assert!(!caps.tool_calling);
    }
}
