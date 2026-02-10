//! Tool parameter types for RICO MCP

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for vector similarity search
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub struct SearchByVectorParams {
    /// 64-dimensional UI layout vector
    pub vector: Vec<f32>,
    /// Maximum number of results to return (default: 10)
    #[serde(default)]
    pub top_k: Option<usize>,
    /// Minimum similarity threshold (0.0-1.0, default: 0.5)
    #[serde(default)]
    pub min_similarity: Option<f32>,
    /// Filter by component types (e.g., ["Button", "TextField"])
    #[serde(default)]
    pub component_filter: Option<Vec<String>>,
}

/// Parameters for getting screen details
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub struct GetScreenDetailsParams {
    /// RICO screen ID (0-66261)
    pub screen_id: u32,
    /// Whether to include screenshot path if available
    #[serde(default)]
    pub include_screenshot: bool,
}

/// Parameters for listing component types
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub struct ListComponentTypesParams {
    /// Filter by category (optional)
    #[serde(default)]
    pub category: Option<String>,
}

/// Parameters for getting pattern guidance
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub struct GetPatternGuidanceParams {
    /// Pattern name (e.g., "Login Screen", "List View", "Navigation Drawer")
    pub pattern_name: String,
    /// Include accessibility notes
    #[serde(default = "default_true")]
    pub include_accessibility: bool,
}

fn default_true() -> bool {
    true
}

/// Parameters for batch screen analysis
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub struct BatchAnalyzeParams {
    /// List of screen IDs to analyze
    pub screen_ids: Vec<u32>,
    /// Include flow analysis between screens
    #[serde(default)]
    pub analyze_flow: bool,
}

/// Parameters for encoding a screenshot to a layout vector
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub struct EncodeScreenshotParams {
    /// Path to the screenshot image file (PNG, JPG, etc.)
    pub image_path: String,
    /// Whether to also search for similar screens after encoding
    #[serde(default)]
    pub search_similar: bool,
    /// Number of similar screens to return if search_similar is true (default: 5)
    #[serde(default)]
    pub top_k: Option<usize>,
}
