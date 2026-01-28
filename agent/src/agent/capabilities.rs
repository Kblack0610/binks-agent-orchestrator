//! Model capability detection and classification
//!
//! Auto-detects what features a model supports (tool calling, thinking/reasoning,
//! function call format, etc.) so the agent can adapt its behavior accordingly.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use crate::llm::show_model;

// ============================================================================
// Capability Types
// ============================================================================

/// Function call format supported by the model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FunctionCallFormat {
    /// Native Ollama tool_calls array
    #[default]
    Native,
    /// XML-style `<function=name>` format (e.g., Llama models)
    Xml,
    /// JSON embedded in content
    Json,
    /// Unknown/undetected format
    Unknown,
}

/// Capabilities detected for a model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    /// Whether the model supports tool/function calling
    pub tool_calling: bool,

    /// Whether the model emits thinking/reasoning tokens (e.g., `<think>` tags)
    pub thinking: bool,

    /// The function call format the model uses
    pub function_call_format: FunctionCallFormat,

    /// Whether the model supports vision/image inputs
    pub vision: bool,

    /// Whether the model is specialized for code tasks
    pub code_specialized: bool,

    /// Whether the model reliably produces JSON output
    pub json_mode: bool,
}

impl Default for ModelCapabilities {
    fn default() -> Self {
        Self {
            tool_calling: true, // Assume tool calling by default
            thinking: false,
            function_call_format: FunctionCallFormat::Native,
            vision: false,
            code_specialized: false,
            json_mode: false,
        }
    }
}

impl ModelCapabilities {
    /// Create capabilities for a non-tool-calling model
    pub fn no_tools() -> Self {
        Self {
            tool_calling: false,
            ..Default::default()
        }
    }

    /// Create capabilities for a thinking/reasoning model
    pub fn thinking_model() -> Self {
        Self {
            thinking: true,
            tool_calling: false, // Most reasoning models don't support tools well
            ..Default::default()
        }
    }
}

// ============================================================================
// Ollama API Response Types
// ============================================================================

/// Response from Ollama `/api/show` endpoint
#[derive(Debug, Deserialize)]
pub struct OllamaShowResponse {
    /// Model template string
    #[serde(default)]
    pub template: String,

    /// Model details
    #[serde(default)]
    pub details: OllamaModelDetails,

    /// Model parameters
    #[serde(default)]
    pub parameters: String,

    /// Modelfile contents
    #[serde(default)]
    pub modelfile: String,
}

/// Details about the model from Ollama
#[derive(Debug, Default, Deserialize)]
pub struct OllamaModelDetails {
    /// Model family (e.g., "llama", "qwen", "deepseek")
    #[serde(default)]
    pub family: String,

    /// Model families list
    #[serde(default)]
    pub families: Vec<String>,

    /// Parameter size string (e.g., "7B")
    #[serde(default)]
    pub parameter_size: String,

    /// Quantization level
    #[serde(default)]
    pub quantization_level: String,
}

// ============================================================================
// Capability Detection
// ============================================================================

/// Cache for detected capabilities
static CAPABILITY_CACHE: LazyLock<Mutex<HashMap<String, ModelCapabilities>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Configuration override for a specific model
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelCapabilityOverride {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calling: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call_format: Option<FunctionCallFormat>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_specialized: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_mode: Option<bool>,
}

impl ModelCapabilityOverride {
    /// Apply overrides to base capabilities
    pub fn apply_to(&self, mut caps: ModelCapabilities) -> ModelCapabilities {
        if let Some(v) = self.tool_calling {
            caps.tool_calling = v;
        }
        if let Some(v) = self.thinking {
            caps.thinking = v;
        }
        if let Some(v) = self.function_call_format {
            caps.function_call_format = v;
        }
        if let Some(v) = self.vision {
            caps.vision = v;
        }
        if let Some(v) = self.code_specialized {
            caps.code_specialized = v;
        }
        if let Some(v) = self.json_mode {
            caps.json_mode = v;
        }
        caps
    }
}

/// Detect capabilities for a model
///
/// Checks cache first, then queries Ollama `/api/show`, then falls back to
/// name-based heuristics. Applies any config overrides at the end.
pub async fn detect_capabilities(
    ollama_url: &str,
    model: &str,
    overrides: Option<&HashMap<String, ModelCapabilityOverride>>,
) -> ModelCapabilities {
    // Check cache first
    {
        let cache = CAPABILITY_CACHE.lock().unwrap();
        if let Some(caps) = cache.get(model) {
            let caps = caps.clone();
            // Apply overrides even for cached values
            return apply_overrides(caps, model, overrides);
        }
    }

    // Try to detect from Ollama
    let caps = match show_model(ollama_url, model).await {
        Ok(info) => detect_from_show_response(&info, model),
        Err(e) => {
            tracing::debug!("Failed to fetch model info for {}: {}", model, e);
            infer_from_model_name(model)
        }
    };

    // Cache the detected capabilities (before overrides)
    {
        let mut cache = CAPABILITY_CACHE.lock().unwrap();
        cache.insert(model.to_string(), caps.clone());
    }

    // Apply overrides
    apply_overrides(caps, model, overrides)
}

/// Clear the capability cache (useful for testing)
pub fn clear_capability_cache() {
    let mut cache = CAPABILITY_CACHE.lock().unwrap();
    cache.clear();
}

fn apply_overrides(
    caps: ModelCapabilities,
    model: &str,
    overrides: Option<&HashMap<String, ModelCapabilityOverride>>,
) -> ModelCapabilities {
    if let Some(overrides_map) = overrides {
        // Try exact match first
        if let Some(override_config) = overrides_map.get(model) {
            return override_config.apply_to(caps);
        }

        // Try base model name (without tag)
        let base_model = model.split(':').next().unwrap_or(model);
        if let Some(override_config) = overrides_map.get(base_model) {
            return override_config.apply_to(caps);
        }
    }
    caps
}

/// Detect capabilities from Ollama show response
pub fn detect_from_show_response(info: &OllamaShowResponse, model: &str) -> ModelCapabilities {
    let mut caps = ModelCapabilities::default();

    let template_lower = info.template.to_lowercase();
    let family_lower = info.details.family.to_lowercase();
    let families: Vec<String> = info
        .details
        .families
        .iter()
        .map(|f| f.to_lowercase())
        .collect();

    // Tool calling detection
    // Look for tool/function-related tokens in template
    let has_tool_tokens = template_lower.contains("tool")
        || template_lower.contains("function")
        || template_lower.contains("<|plugin|>")
        || template_lower.contains("available_tools");

    caps.tool_calling = has_tool_tokens;

    // Function call format detection
    if template_lower.contains("<function=") || template_lower.contains("<|python_tag|>") {
        caps.function_call_format = FunctionCallFormat::Xml;
    } else if has_tool_tokens {
        caps.function_call_format = FunctionCallFormat::Native;
    }

    // Thinking/reasoning detection
    if template_lower.contains("<think>")
        || template_lower.contains("<|think|>")
        || is_reasoning_model(model)
    {
        caps.thinking = true;
        // Reasoning models typically don't handle tools well
        if !has_tool_tokens {
            caps.tool_calling = false;
        }
    }

    // Vision detection
    if families.iter().any(|f| f.contains("clip"))
        || family_lower.contains("llava")
        || model.to_lowercase().contains("vision")
        || model.to_lowercase().contains("llava")
    {
        caps.vision = true;
    }

    // Code specialization detection
    if family_lower.contains("code")
        || model.to_lowercase().contains("code")
        || model.to_lowercase().contains("coder")
    {
        caps.code_specialized = true;
    }

    // JSON mode - conservative, only if explicitly supported
    if template_lower.contains("json") && template_lower.contains("format") {
        caps.json_mode = true;
    }

    caps
}

/// Infer capabilities from model name alone (fallback)
pub fn infer_from_model_name(model: &str) -> ModelCapabilities {
    let model_lower = model.to_lowercase();
    let mut caps = ModelCapabilities::default();

    // Reasoning models
    if is_reasoning_model(model) {
        caps.thinking = true;
        caps.tool_calling = false;
        return caps;
    }

    // Vision models
    if model_lower.contains("llava")
        || model_lower.contains("vision")
        || model_lower.contains("bakllava")
    {
        caps.vision = true;
    }

    // Code models
    if model_lower.contains("code") || model_lower.contains("coder") {
        caps.code_specialized = true;
    }

    // Models known to support tools well
    let good_tool_models = [
        "qwen",
        "llama3",
        "mistral",
        "mixtral",
        "phi",
        "gemma",
        "command-r",
    ];

    caps.tool_calling = good_tool_models.iter().any(|m| model_lower.contains(m));

    // XML format models (Llama-style)
    if model_lower.contains("llama") {
        caps.function_call_format = FunctionCallFormat::Xml;
    }

    caps
}

/// Check if a model is a reasoning/thinking model
fn is_reasoning_model(model: &str) -> bool {
    let model_lower = model.to_lowercase();
    model_lower.contains("deepseek-r1")
        || model_lower.contains("qwq")
        || model_lower.contains("o1")
        || model_lower.contains("reasoning")
        || model_lower.contains("-r1")
}

// ============================================================================
// Think Tag Processing
// ============================================================================

static THINK_TAG_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    // Match <think>...</think> blocks, including multiline
    Regex::new(r"(?s)<think>.*?</think>").expect("Invalid think tag regex")
});

/// Strip `<think>...</think>` blocks from content
///
/// Reasoning models emit their internal reasoning in these tags, which
/// should be removed from the final user-facing output.
pub fn strip_think_tags(content: &str) -> String {
    let result = THINK_TAG_REGEX.replace_all(content, "");
    // Clean up any resulting double newlines or leading/trailing whitespace
    result
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_capabilities() {
        let caps = ModelCapabilities::default();
        assert!(caps.tool_calling);
        assert!(!caps.thinking);
        assert_eq!(caps.function_call_format, FunctionCallFormat::Native);
        assert!(!caps.vision);
        assert!(!caps.code_specialized);
        assert!(!caps.json_mode);
    }

    #[test]
    fn test_no_tools_capabilities() {
        let caps = ModelCapabilities::no_tools();
        assert!(!caps.tool_calling);
    }

    #[test]
    fn test_thinking_model_capabilities() {
        let caps = ModelCapabilities::thinking_model();
        assert!(caps.thinking);
        assert!(!caps.tool_calling);
    }

    #[test]
    fn test_infer_reasoning_models() {
        let deepseek = infer_from_model_name("deepseek-r1:32b");
        assert!(deepseek.thinking);
        assert!(!deepseek.tool_calling);

        let qwq = infer_from_model_name("qwq:32b");
        assert!(qwq.thinking);
        assert!(!qwq.tool_calling);
    }

    #[test]
    fn test_infer_tool_calling_models() {
        let qwen = infer_from_model_name("qwen3:14b");
        assert!(qwen.tool_calling);

        let llama = infer_from_model_name("llama3.1:8b");
        assert!(llama.tool_calling);
        assert_eq!(llama.function_call_format, FunctionCallFormat::Xml);

        let mistral = infer_from_model_name("mistral:7b");
        assert!(mistral.tool_calling);
    }

    #[test]
    fn test_infer_vision_models() {
        let llava = infer_from_model_name("llava:13b");
        assert!(llava.vision);

        let bakllava = infer_from_model_name("bakllava:7b");
        assert!(bakllava.vision);
    }

    #[test]
    fn test_infer_code_models() {
        let codellama = infer_from_model_name("codellama:13b");
        assert!(codellama.code_specialized);

        let qwen_coder = infer_from_model_name("qwen2.5-coder:32b");
        assert!(qwen_coder.code_specialized);
    }

    #[test]
    fn test_override_apply() {
        let caps = ModelCapabilities::default();
        let override_config = ModelCapabilityOverride {
            tool_calling: Some(false),
            thinking: Some(true),
            ..Default::default()
        };

        let result = override_config.apply_to(caps);
        assert!(!result.tool_calling);
        assert!(result.thinking);
    }

    #[test]
    fn test_strip_think_tags_simple() {
        let content = "Hello <think>internal reasoning here</think> World";
        let result = strip_think_tags(content);
        assert_eq!(result, "Hello  World");
    }

    #[test]
    fn test_strip_think_tags_multiline() {
        let content = r#"Let me help you.
<think>
This is my internal reasoning.
It spans multiple lines.
</think>
Here's the answer."#;

        let result = strip_think_tags(content);
        assert!(!result.contains("<think>"));
        assert!(!result.contains("</think>"));
        assert!(!result.contains("internal reasoning"));
        assert!(result.contains("Let me help you"));
        assert!(result.contains("Here's the answer"));
    }

    #[test]
    fn test_strip_think_tags_multiple() {
        let content = "<think>first</think>middle<think>second</think>end";
        let result = strip_think_tags(content);
        assert!(!result.contains("<think>"));
        assert!(result.contains("middle"));
        assert!(result.contains("end"));
    }

    #[test]
    fn test_strip_think_tags_no_tags() {
        let content = "No thinking tags here";
        let result = strip_think_tags(content);
        assert_eq!(result, "No thinking tags here");
    }

    #[test]
    fn test_detect_from_show_with_tool_template() {
        let info = OllamaShowResponse {
            template: "{{.System}}\n{{if .Tools}}Available tools: {{.Tools}}{{end}}".to_string(),
            details: OllamaModelDetails::default(),
            parameters: String::new(),
            modelfile: String::new(),
        };

        let caps = detect_from_show_response(&info, "test-model");
        assert!(caps.tool_calling);
    }

    #[test]
    fn test_detect_from_show_with_think_template() {
        let info = OllamaShowResponse {
            template: "{{.System}}\n<think>{{.Thinking}}</think>".to_string(),
            details: OllamaModelDetails::default(),
            parameters: String::new(),
            modelfile: String::new(),
        };

        let caps = detect_from_show_response(&info, "test-model");
        assert!(caps.thinking);
    }

    #[test]
    fn test_detect_from_show_with_xml_function() {
        let info = OllamaShowResponse {
            template: "{{.System}}\n<function={{.FunctionName}}>".to_string(),
            details: OllamaModelDetails::default(),
            parameters: String::new(),
            modelfile: String::new(),
        };

        let caps = detect_from_show_response(&info, "test-model");
        assert_eq!(caps.function_call_format, FunctionCallFormat::Xml);
    }

    #[test]
    fn test_detect_from_show_vision_family() {
        let info = OllamaShowResponse {
            template: String::new(),
            details: OllamaModelDetails {
                family: "llava".to_string(),
                families: vec!["clip".to_string()],
                parameter_size: "13B".to_string(),
                quantization_level: "Q4_0".to_string(),
            },
            parameters: String::new(),
            modelfile: String::new(),
        };

        let caps = detect_from_show_response(&info, "llava:13b");
        assert!(caps.vision);
    }
}
