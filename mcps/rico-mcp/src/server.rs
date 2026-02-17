//! MCP Server implementation for RICO dataset access
//!
//! Provides tools for similarity search and design pattern guidance
//! based on the RICO Android UI dataset.

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError,
};
use std::sync::Arc;

use crate::config::RicoConfig;
use crate::dataset::{DatasetLoader, ScreenCache};
use crate::params::*;
use crate::search::VectorSearch;
use crate::types::{ComponentFrequency, DatasetStatus, PatternGuidance, COMPONENT_TYPES};

/// The main RICO MCP Server
pub struct RicoMcpServer {
    config: RicoConfig,
    loader: Arc<DatasetLoader>,
    cache: ScreenCache,
    tool_router: ToolRouter<Self>,
}

// ============================================================================
// Tool Router
// ============================================================================

#[tool_router]
impl RicoMcpServer {
    pub fn new() -> Result<Self, anyhow::Error> {
        let config = RicoConfig::from_env();
        config.validate()?;

        let loader = Arc::new(DatasetLoader::load(&config)?);
        let cache = ScreenCache::new(config.cache_size);

        Ok(Self {
            config,
            loader,
            cache,
            tool_router: Self::tool_router(),
        })
    }

    // ========================================================================
    // Search Tools
    // ========================================================================

    #[tool(
        description = "Search for similar UI screens using a 64-dimensional layout vector. Returns top-k most similar screens from the RICO dataset with metadata."
    )]
    async fn search_by_vector(
        &self,
        Parameters(params): Parameters<SearchByVectorParams>,
    ) -> Result<CallToolResult, McpError> {
        // Validate vector length
        if params.vector.len() != 64 {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "Vector must be 64-dimensional, got {}",
                params.vector.len()
            ))]));
        }

        let mut query_arr = [0.0f32; 64];
        for (i, &v) in params.vector.iter().enumerate() {
            query_arr[i] = v;
        }

        let top_k = params.top_k.unwrap_or(self.config.default_top_k);
        let min_sim = params.min_similarity.unwrap_or(self.config.min_similarity);
        let filter = params.component_filter.as_deref();

        let search = VectorSearch::new(&self.loader);
        let results = search.search(&query_arr, top_k, min_sim, filter);

        let json = serde_json::to_string_pretty(&results)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(
        description = "Encode a screenshot image into a 64-dimensional layout vector. Can optionally search for similar screens in the RICO dataset."
    )]
    async fn encode_screenshot(
        &self,
        Parameters(params): Parameters<EncodeScreenshotParams>,
    ) -> Result<CallToolResult, McpError> {
        use std::process::Command;

        // Find the encoder script
        let script_path = self.find_encoder_script();

        // Run the Python encoder
        let output = Command::new("python3")
            .arg(&script_path)
            .arg(&params.image_path)
            .arg("--json")
            .output()
            .map_err(|e| McpError::internal_error(format!("Failed to run encoder: {}", e), None))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "Encoding failed: {}",
                stderr
            ))]));
        }

        // Parse the vector from JSON output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let vector: Vec<f32> = serde_json::from_str(&stdout).map_err(|e| {
            McpError::internal_error(format!("Failed to parse vector: {}", e), None)
        })?;

        let mut result = serde_json::json!({
            "vector": vector,
            "dimensions": vector.len(),
            "source_image": params.image_path
        });

        // Optionally search for similar screens
        if params.search_similar {
            let mut query_arr = [0.0f32; 64];
            for (i, &v) in vector.iter().take(64).enumerate() {
                query_arr[i] = v;
            }

            let top_k = params.top_k.unwrap_or(5);
            let search = VectorSearch::new(&self.loader);
            let similar = search.search(&query_arr, top_k, self.config.min_similarity, None);

            result["similar_screens"] = serde_json::to_value(&similar)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        }

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(
        description = "Get detailed metadata for a specific RICO screen by ID. Includes components, text buttons, icons, and screenshot availability."
    )]
    async fn get_screen_details(
        &self,
        Parameters(params): Parameters<GetScreenDetailsParams>,
    ) -> Result<CallToolResult, McpError> {
        // Check cache first
        if let Some(cached) = self.cache.get(params.screen_id) {
            let json = serde_json::to_string_pretty(&cached)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            return Ok(CallToolResult::success(vec![Content::text(json)]));
        }

        // Get from loader
        match self.loader.get_metadata(params.screen_id) {
            Some(meta) => {
                let mut meta = meta.clone();

                // Add screenshot path if requested and available
                if params.include_screenshot {
                    meta.screenshot_path = self.loader.screenshot_path(params.screen_id);
                }

                // Cache it
                self.cache.insert(params.screen_id, meta.clone());

                let json = serde_json::to_string_pretty(&meta)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            None => Ok(CallToolResult::error(vec![Content::text(format!(
                "Screen ID {} not found in dataset",
                params.screen_id
            ))])),
        }
    }

    // ========================================================================
    // Reference Tools
    // ========================================================================

    #[tool(
        description = "List the 24 RICO UI component types (Text, Image, Icon, Button, etc.) with optional category filtering."
    )]
    async fn list_component_types(
        &self,
        Parameters(_params): Parameters<ListComponentTypesParams>,
    ) -> Result<CallToolResult, McpError> {
        let types: Vec<serde_json::Value> = COMPONENT_TYPES
            .iter()
            .enumerate()
            .map(|(i, name)| {
                serde_json::json!({
                    "id": i,
                    "name": name
                })
            })
            .collect();

        let json = serde_json::to_string_pretty(&types)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(
        description = "Get design guidance and best practices for a specific UI pattern (e.g., 'Login Screen', 'List View', 'Navigation Drawer')."
    )]
    async fn get_pattern_guidance(
        &self,
        Parameters(params): Parameters<GetPatternGuidanceParams>,
    ) -> Result<CallToolResult, McpError> {
        use std::collections::HashMap;

        // Map pattern names to search keywords
        let pattern_lower = params.pattern_name.to_lowercase();
        let (pattern_name, search_keywords): (&str, Vec<&str>) = if pattern_lower.contains("login")
            || pattern_lower.contains("signin")
            || pattern_lower.contains("sign in")
        {
            ("Login Screen", vec!["login", "signin", "auth"])
        } else if pattern_lower.contains("list") {
            ("List View", vec!["list", "feed", "timeline"])
        } else if pattern_lower.contains("nav")
            || pattern_lower.contains("drawer")
            || pattern_lower.contains("menu")
        {
            ("Navigation Drawer", vec!["drawer", "menu", "nav"])
        } else if pattern_lower.contains("settings") || pattern_lower.contains("preference") {
            ("Settings Screen", vec!["settings", "preference", "config"])
        } else if pattern_lower.contains("profile") || pattern_lower.contains("account") {
            ("Profile Screen", vec!["profile", "account", "user"])
        } else if pattern_lower.contains("search") {
            ("Search Screen", vec!["search", "find", "query"])
        } else if pattern_lower.contains("home") || pattern_lower.contains("dashboard") {
            ("Home Screen", vec!["home", "dashboard", "main"])
        } else if pattern_lower.contains("detail") || pattern_lower.contains("view") {
            ("Detail View", vec!["detail", "view", "info"])
        } else if pattern_lower.contains("form") || pattern_lower.contains("input") {
            ("Form/Input Screen", vec!["form", "input", "edit"])
        } else if pattern_lower.contains("chat") || pattern_lower.contains("message") {
            ("Chat/Messaging", vec!["chat", "message", "conversation"])
        } else {
            (params.pattern_name.as_str(), vec![pattern_lower.as_str()])
        };

        // Search for matching screens from the dataset
        let mut matching_screen_ids: Vec<u32> = Vec::new();
        for keyword in &search_keywords {
            let ids = self.loader.search_by_app_pattern(keyword, 100);
            matching_screen_ids.extend(ids);
        }
        matching_screen_ids.sort();
        matching_screen_ids.dedup();

        // Aggregate component frequencies from matching screens
        let mut component_counts: HashMap<String, usize> = HashMap::new();
        let mut app_names: Vec<String> = Vec::new();
        let screens_analyzed = matching_screen_ids.len();

        for screen_id in &matching_screen_ids {
            if let Some(meta) = self.loader.get_metadata(*screen_id) {
                // Count components
                for component in &meta.components {
                    *component_counts.entry(component.name.clone()).or_insert(0) += 1;
                }
                // Collect app names
                if let Some(ref name) = meta.app_name {
                    if !app_names.contains(name) {
                        app_names.push(name.clone());
                    }
                } else if !app_names.contains(&meta.app_package) {
                    app_names.push(meta.app_package.clone());
                }
            }
        }

        // Convert to frequencies sorted by occurrence
        let mut component_frequencies: Vec<ComponentFrequency> = component_counts
            .into_iter()
            .map(|(name, count)| ComponentFrequency {
                name,
                frequency: if screens_analyzed > 0 {
                    count as f32 / screens_analyzed as f32
                } else {
                    0.0
                },
            })
            .collect();
        component_frequencies.sort_by(|a, b| b.frequency.partial_cmp(&a.frequency).unwrap());
        component_frequencies.truncate(10); // Top 10 components

        // Limit app examples
        app_names.truncate(5);

        // Curated design tips based on pattern
        let design_tips = get_curated_design_tips(pattern_name);
        let accessibility_notes = if params.include_accessibility {
            get_curated_accessibility_notes(pattern_name)
        } else {
            vec![]
        };

        // Derive typical layout from component distribution
        let typical_layout = derive_layout_description(pattern_name, &component_frequencies);

        let guidance = PatternGuidance {
            pattern_name: pattern_name.to_string(),
            screens_analyzed,
            component_frequencies,
            typical_layout,
            example_apps: app_names,
            design_tips,
            accessibility_notes,
        };

        let json = serde_json::to_string_pretty(&guidance)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ========================================================================
    // Status Tools
    // ========================================================================

    #[tool(
        description = "Get RICO dataset status including loaded vectors, metadata, annotations, and cache statistics."
    )]
    async fn get_dataset_status(&self) -> Result<CallToolResult, McpError> {
        let cache_stats = self.cache.stats();

        let status = DatasetStatus {
            vectors_loaded: self.loader.screen_count(),
            metadata_loaded: self.loader.screen_count(),
            annotations_loaded: self.loader.has_annotations(),
            screenshots_available: self
                .loader
                .screen_ids()
                .filter(|&id| self.loader.screenshot_exists(id))
                .count(),
            cache_hits: cache_stats.hits,
            cache_misses: cache_stats.misses,
            cache_size: cache_stats.size,
        };

        let json = serde_json::to_string_pretty(&status)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ========================================================================
    // Batch/Flow Tools
    // ========================================================================

    #[tool(
        description = "Analyze multiple screens as a UI flow. Computes flow cohesion and cross-screen consistency."
    )]
    async fn analyze_flow(
        &self,
        Parameters(params): Parameters<BatchAnalyzeParams>,
    ) -> Result<CallToolResult, McpError> {
        let search = VectorSearch::new(&self.loader);

        // Get metadata for all screens
        let screens: Vec<_> = params
            .screen_ids
            .iter()
            .filter_map(|&id| self.loader.get_metadata(id).cloned())
            .collect();

        if screens.is_empty() {
            return Ok(CallToolResult::error(vec![Content::text(
                "No valid screen IDs found",
            )]));
        }

        let mut result = serde_json::json!({
            "screen_count": screens.len(),
            "screens": screens.iter().map(|s| serde_json::json!({
                "screen_id": s.screen_id,
                "app_package": s.app_package,
                "components": s.component_names()
            })).collect::<Vec<_>>()
        });

        if params.analyze_flow {
            // Compute flow cohesion (average pairwise similarity)
            if let Some(cohesion) = search.flow_cohesion(&params.screen_ids) {
                result["flow_cohesion"] = serde_json::json!(cohesion);
                result["cohesion_rating"] = serde_json::json!(if cohesion > 0.8 {
                    "High - consistent UI flow"
                } else if cohesion > 0.6 {
                    "Medium - mostly consistent"
                } else {
                    "Low - consider improving consistency"
                });
            }

            // Analyze component consistency
            let mut component_counts: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for screen in &screens {
                for comp in screen.component_names() {
                    *component_counts.entry(comp).or_insert(0) += 1;
                }
            }

            let screen_count = screens.len();
            let consistent_components: Vec<_> = component_counts
                .into_iter()
                .filter(|(_, count)| *count == screen_count)
                .map(|(name, _)| name)
                .collect();

            result["consistent_components"] = serde_json::json!(consistent_components);
        }

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    /// Find the Python encoder script for screenshot-to-vector conversion.
    /// Looks in several locations: beside the binary, in scripts/, or via env var.
    fn find_encoder_script(&self) -> String {
        use std::env;
        use std::path::PathBuf;

        // Check environment variable first
        if let Ok(path) = env::var("RICO_ENCODER_SCRIPT") {
            return path;
        }

        // Try to find relative to the executable
        if let Ok(exe_path) = env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                // Check beside the binary
                let beside_exe = exe_dir.join("rico-encode.py");
                if beside_exe.exists() {
                    return beside_exe.to_string_lossy().to_string();
                }

                // Check in scripts/ relative to binary
                let in_scripts = exe_dir.join("scripts").join("rico-encode.py");
                if in_scripts.exists() {
                    return in_scripts.to_string_lossy().to_string();
                }

                // Check parent directories (for dev builds)
                let mut search_dir = exe_dir.to_path_buf();
                for _ in 0..5 {
                    let scripts_path = search_dir.join("scripts").join("rico-encode.py");
                    if scripts_path.exists() {
                        return scripts_path.to_string_lossy().to_string();
                    }
                    if !search_dir.pop() {
                        break;
                    }
                }
            }
        }

        // Check common locations
        let common_paths = [
            PathBuf::from("/usr/local/share/rico-mcp/rico-encode.py"),
            PathBuf::from(env::var("HOME").unwrap_or_default())
                .join(".rico-mcp")
                .join("rico-encode.py"),
        ];

        for path in &common_paths {
            if path.exists() {
                return path.to_string_lossy().to_string();
            }
        }

        // Fallback: assume it's in PATH or current directory
        "rico-encode.py".to_string()
    }
}

// ============================================================================
// Server Handler Implementation
// ============================================================================

#[tool_handler]
impl rmcp::ServerHandler for RicoMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "RICO UI dataset MCP server for mobile design similarity search. \
                 Provides access to 66,000+ Android UI screens with layout vectors, \
                 component classifications, and design pattern guidance."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

impl Default for RicoMcpServer {
    fn default() -> Self {
        Self::new().expect("Failed to create RicoMcpServer")
    }
}

// ============================================================================
// Helper Functions for Pattern Guidance
// ============================================================================

/// Get curated design tips for a pattern
fn get_curated_design_tips(pattern_name: &str) -> Vec<String> {
    match pattern_name {
        "Login Screen" => vec![
            "Keep form fields minimal - username/email and password only".to_string(),
            "Make the primary action button prominent".to_string(),
            "Provide clear error states for invalid input".to_string(),
            "Consider biometric login options".to_string(),
        ],
        "List View" => vec![
            "Use consistent item heights for smooth scrolling".to_string(),
            "Include visual hierarchy in list items".to_string(),
            "Support swipe actions for common operations".to_string(),
            "Consider pull-to-refresh for updatable content".to_string(),
        ],
        "Navigation Drawer" => vec![
            "Group related navigation items".to_string(),
            "Highlight the current section".to_string(),
            "Keep navigation items to essential destinations".to_string(),
            "Consider using bottom navigation for <5 destinations".to_string(),
        ],
        "Settings Screen" => vec![
            "Group related settings into categories".to_string(),
            "Use clear, descriptive labels".to_string(),
            "Show current values for selections".to_string(),
            "Provide search for large settings lists".to_string(),
        ],
        "Profile Screen" => vec![
            "Display key user info prominently".to_string(),
            "Make edit actions easily accessible".to_string(),
            "Use avatar/photo as visual anchor".to_string(),
            "Consider privacy controls visibility".to_string(),
        ],
        "Search Screen" => vec![
            "Auto-focus the search input".to_string(),
            "Show recent searches".to_string(),
            "Provide search suggestions".to_string(),
            "Display results as user types".to_string(),
        ],
        "Home Screen" | "Dashboard" => vec![
            "Prioritize most-used actions".to_string(),
            "Use cards to group related content".to_string(),
            "Consider personalization".to_string(),
            "Keep navigation clear and accessible".to_string(),
        ],
        "Detail View" => vec![
            "Show key information first".to_string(),
            "Use hierarchy to organize content".to_string(),
            "Provide clear back navigation".to_string(),
            "Consider sharing and action options".to_string(),
        ],
        "Form/Input Screen" => vec![
            "Use appropriate input types".to_string(),
            "Validate input in real-time".to_string(),
            "Show progress for multi-step forms".to_string(),
            "Save draft content automatically".to_string(),
        ],
        "Chat/Messaging" => vec![
            "Keep message bubbles readable".to_string(),
            "Show delivery/read status".to_string(),
            "Support rich content (images, links)".to_string(),
            "Optimize keyboard interactions".to_string(),
        ],
        _ => vec![
            "Follow Material Design guidelines".to_string(),
            "Maintain consistent spacing and typography".to_string(),
            "Test on multiple screen sizes".to_string(),
        ],
    }
}

/// Get curated accessibility notes for a pattern
fn get_curated_accessibility_notes(pattern_name: &str) -> Vec<String> {
    match pattern_name {
        "Login Screen" => vec![
            "Label all form fields properly".to_string(),
            "Ensure sufficient color contrast".to_string(),
            "Support password visibility toggle".to_string(),
            "Provide clear focus indicators".to_string(),
        ],
        "List View" => vec![
            "Ensure list items are focusable".to_string(),
            "Provide content descriptions for images".to_string(),
            "Support keyboard navigation".to_string(),
        ],
        "Navigation Drawer" => vec![
            "Announce drawer open/close state".to_string(),
            "Trap focus within open drawer".to_string(),
            "Support escape key to close".to_string(),
        ],
        "Settings Screen" => vec![
            "Use proper heading levels".to_string(),
            "Announce toggle/switch states".to_string(),
            "Provide descriptive labels for all controls".to_string(),
        ],
        "Profile Screen" => vec![
            "Describe images with alt text".to_string(),
            "Use semantic headings".to_string(),
            "Make edit buttons clearly labeled".to_string(),
        ],
        "Search Screen" => vec![
            "Label the search field".to_string(),
            "Announce result counts".to_string(),
            "Make results keyboard navigable".to_string(),
        ],
        _ => vec![
            "Follow WCAG 2.1 guidelines".to_string(),
            "Test with TalkBack/VoiceOver".to_string(),
            "Ensure touch targets are at least 48dp".to_string(),
        ],
    }
}

/// Derive layout description from component frequencies
fn derive_layout_description(pattern_name: &str, frequencies: &[ComponentFrequency]) -> String {
    // Start with pattern-specific layouts
    let base = match pattern_name {
        "Login Screen" => "Centered form with logo at top, inputs in middle, buttons at bottom",
        "List View" => "Vertical scrolling list with consistent item heights",
        "Navigation Drawer" => "Slide-in panel from left with menu items",
        "Settings Screen" => "Scrollable list of grouped preference items",
        "Profile Screen" => "Header with avatar, followed by user details and actions",
        "Search Screen" => "Search bar at top, results list below",
        "Home Screen" | "Dashboard" => "Grid or list of cards/tiles with key actions",
        "Detail View" => "Hero image or header, followed by content sections",
        "Form/Input Screen" => "Vertical stack of labeled input fields with submit action",
        "Chat/Messaging" => "Message list with input bar at bottom",
        _ => "Layout varies by implementation",
    };

    // Add component insights if we have data
    if !frequencies.is_empty() {
        let top_components: Vec<&str> = frequencies
            .iter()
            .take(3)
            .map(|c| c.name.as_str())
            .collect();
        format!("{}. Common components: {}", base, top_components.join(", "))
    } else {
        base.to_string()
    }
}
