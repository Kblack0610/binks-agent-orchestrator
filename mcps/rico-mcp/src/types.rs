//! Core types for RICO dataset representation

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A 64-dimensional UI layout vector from RICO
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LayoutVector(pub Vec<f32>);

impl LayoutVector {
    /// Get the vector as a fixed-size array (panics if not exactly 64 elements)
    pub fn as_array(&self) -> [f32; 64] {
        let mut arr = [0.0f32; 64];
        for (i, &v) in self.0.iter().take(64).enumerate() {
            arr[i] = v;
        }
        arr
    }

    /// Get the vector as a slice
    pub fn as_slice(&self) -> &[f32] {
        &self.0
    }

    /// Create from a fixed array
    pub fn from_array(arr: [f32; 64]) -> Self {
        Self(arr.to_vec())
    }
}

/// UI component classification from semantic annotations
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ComponentClass {
    /// Numeric class ID (0-23 for component types)
    pub class_id: u32,
    /// Human-readable class name (e.g., "Button", "ImageView", "TextField")
    pub name: String,
    /// Confidence score from 0.0 to 1.0
    #[serde(default)]
    pub confidence: f32,
}

/// Text button concept (one of 197 concepts)
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct TextButtonConcept {
    /// Concept ID (0-196)
    pub concept_id: u32,
    /// Concept name (e.g., "Submit", "Cancel", "Next")
    pub name: String,
}

/// Icon classification (one of 97 classes)
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct IconClass {
    /// Icon class ID (0-96)
    pub class_id: u32,
    /// Icon class name (e.g., "back_arrow", "menu", "search")
    pub name: String,
}

/// RICO UI screen metadata
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ScreenMetadata {
    /// Unique screen identifier (0-66261)
    pub screen_id: u32,
    /// Android package name (e.g., "com.instagram.android")
    pub app_package: String,
    /// Human-readable app name (if available)
    #[serde(default)]
    pub app_name: Option<String>,
    /// Detected UI components
    #[serde(default)]
    pub components: Vec<ComponentClass>,
    /// Text button concepts found on this screen
    #[serde(default)]
    pub text_buttons: Vec<TextButtonConcept>,
    /// Icon classes found on this screen
    #[serde(default)]
    pub icon_classes: Vec<IconClass>,
    /// Path to screenshot file (if available)
    #[serde(default)]
    pub screenshot_path: Option<String>,
    /// Path to view hierarchy JSON (if available)
    #[serde(default)]
    pub hierarchy_path: Option<String>,
}

/// Result from similarity search
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct SimilarityResult {
    /// RICO screen ID
    pub screen_id: u32,
    /// Cosine similarity score (0.0-1.0, higher is more similar)
    pub similarity_score: f32,
    /// App name (if available)
    #[serde(default)]
    pub app_name: Option<String>,
    /// App package name
    pub app_package: String,
    /// Main component types found
    #[serde(default)]
    pub components: Vec<String>,
    /// Whether screenshot is available locally
    pub screenshot_available: bool,
}

/// UI pattern summary for design guidance
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct PatternGuidance {
    /// Pattern name (e.g., "Login Screen", "List View", "Navigation Drawer")
    pub pattern_name: String,
    /// Common components in this pattern
    pub common_components: Vec<String>,
    /// Description of typical layout structure
    pub typical_layout: String,
    /// Example apps using this pattern
    pub example_apps: Vec<String>,
    /// Design best practices and considerations
    pub design_tips: Vec<String>,
    /// Accessibility considerations
    pub accessibility_notes: Vec<String>,
}

/// Dataset status information
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct DatasetStatus {
    /// Number of screens with vectors loaded
    pub vectors_loaded: usize,
    /// Number of screens with metadata loaded
    pub metadata_loaded: usize,
    /// Whether semantic annotations are loaded
    pub annotations_loaded: bool,
    /// Number of screens with local screenshots
    pub screenshots_available: usize,
    /// Cache statistics
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_size: usize,
}

/// The 24 RICO component types
pub const COMPONENT_TYPES: &[&str] = &[
    "Text",
    "Image",
    "Icon",
    "Text Button",
    "List Item",
    "Input",
    "Background Image",
    "Card",
    "Web View",
    "Radio Button",
    "Drawer",
    "Checkbox",
    "Advertisement",
    "Modal",
    "Pager Indicator",
    "Slider",
    "On/Off Switch",
    "Button Bar",
    "Toolbar",
    "Number Stepper",
    "Multi-Tab",
    "Date Picker",
    "Map View",
    "Video",
];

impl ScreenMetadata {
    /// Get component type names as strings
    pub fn component_names(&self) -> Vec<String> {
        self.components.iter().map(|c| c.name.clone()).collect()
    }
}
