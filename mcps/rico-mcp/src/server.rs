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
use crate::types::{DatasetStatus, PatternGuidance, COMPONENT_TYPES};

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
        // Return pattern-specific guidance
        let guidance = match params.pattern_name.to_lowercase().as_str() {
            name if name.contains("login") => PatternGuidance {
                pattern_name: "Login Screen".to_string(),
                common_components: vec![
                    "Text Input (username/email)".to_string(),
                    "Text Input (password)".to_string(),
                    "Text Button (Sign In)".to_string(),
                    "Text Button (Forgot Password)".to_string(),
                    "Image (logo)".to_string(),
                ],
                typical_layout: "Centered form with logo at top, inputs in middle, buttons at bottom".to_string(),
                example_apps: vec!["Instagram".to_string(), "Facebook".to_string(), "Twitter".to_string()],
                design_tips: vec![
                    "Keep form fields minimal - username/email and password only".to_string(),
                    "Make the primary action button prominent".to_string(),
                    "Provide clear error states for invalid input".to_string(),
                    "Consider biometric login options".to_string(),
                ],
                accessibility_notes: if params.include_accessibility {
                    vec![
                        "Label all form fields properly".to_string(),
                        "Ensure sufficient color contrast".to_string(),
                        "Support password visibility toggle".to_string(),
                        "Provide clear focus indicators".to_string(),
                    ]
                } else {
                    vec![]
                },
            },
            name if name.contains("list") => PatternGuidance {
                pattern_name: "List View".to_string(),
                common_components: vec![
                    "List Item".to_string(),
                    "Image/Icon".to_string(),
                    "Text (title)".to_string(),
                    "Text (subtitle)".to_string(),
                ],
                typical_layout: "Vertical scrolling list with consistent item heights".to_string(),
                example_apps: vec!["Gmail".to_string(), "Contacts".to_string(), "Settings".to_string()],
                design_tips: vec![
                    "Use consistent item heights for smooth scrolling".to_string(),
                    "Include visual hierarchy in list items".to_string(),
                    "Support swipe actions for common operations".to_string(),
                    "Consider pull-to-refresh for updatable content".to_string(),
                ],
                accessibility_notes: if params.include_accessibility {
                    vec![
                        "Ensure list items are focusable".to_string(),
                        "Provide content descriptions for images".to_string(),
                        "Support keyboard navigation".to_string(),
                    ]
                } else {
                    vec![]
                },
            },
            name if name.contains("nav") || name.contains("drawer") => PatternGuidance {
                pattern_name: "Navigation Drawer".to_string(),
                common_components: vec![
                    "Drawer".to_string(),
                    "List Item".to_string(),
                    "Icon".to_string(),
                    "Text".to_string(),
                    "Image (profile)".to_string(),
                ],
                typical_layout: "Slide-in panel from left with menu items".to_string(),
                example_apps: vec!["Gmail".to_string(), "Google Drive".to_string(), "Spotify".to_string()],
                design_tips: vec![
                    "Group related navigation items".to_string(),
                    "Highlight the current section".to_string(),
                    "Keep navigation items to essential destinations".to_string(),
                    "Consider using bottom navigation for <5 destinations".to_string(),
                ],
                accessibility_notes: if params.include_accessibility {
                    vec![
                        "Announce drawer open/close state".to_string(),
                        "Trap focus within open drawer".to_string(),
                        "Support escape key to close".to_string(),
                    ]
                } else {
                    vec![]
                },
            },
            _ => PatternGuidance {
                pattern_name: params.pattern_name.clone(),
                common_components: vec!["See similar screens for component suggestions".to_string()],
                typical_layout: "Use search_by_vector to find similar patterns in RICO dataset".to_string(),
                example_apps: vec![],
                design_tips: vec![
                    "Follow Material Design guidelines".to_string(),
                    "Maintain consistent spacing and typography".to_string(),
                    "Test on multiple screen sizes".to_string(),
                ],
                accessibility_notes: if params.include_accessibility {
                    vec![
                        "Follow WCAG 2.1 guidelines".to_string(),
                        "Test with TalkBack/VoiceOver".to_string(),
                        "Ensure touch targets are at least 48dp".to_string(),
                    ]
                } else {
                    vec![]
                },
            },
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
            screenshots_available: self.loader.screen_ids().filter(|&id| self.loader.screenshot_exists(id)).count(),
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
                result["cohesion_rating"] = serde_json::json!(
                    if cohesion > 0.8 { "High - consistent UI flow" }
                    else if cohesion > 0.6 { "Medium - mostly consistent" }
                    else { "Low - consider improving consistency" }
                );
            }

            // Analyze component consistency
            let mut component_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
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
