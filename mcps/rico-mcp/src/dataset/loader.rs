//! NPY and JSON loading for RICO dataset

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use anyhow::{Context, Result};
use ndarray::{Array2, ArrayView1};
use ndarray_npy::ReadNpyExt;
use serde::Deserialize;
use tracing::{debug, info};

use crate::config::RicoConfig;
use crate::types::{ComponentClass, IconClass, LayoutVector, ScreenMetadata, TextButtonConcept};

/// Raw metadata from JSON file
#[derive(Debug, Deserialize)]
struct RawMetadata {
    screen_id: u32,
    app_package: String,
    #[serde(default)]
    app_name: Option<String>,
}

/// Raw semantic annotation from JSON
#[derive(Debug, Deserialize)]
struct RawAnnotation {
    #[serde(default)]
    components: Vec<RawComponent>,
    #[serde(default)]
    text_buttons: Vec<RawTextButton>,
    #[serde(default)]
    icons: Vec<RawIcon>,
}

#[derive(Debug, Deserialize)]
struct RawComponent {
    class: u32,
    #[serde(default)]
    confidence: f32,
}

#[derive(Debug, Deserialize)]
struct RawTextButton {
    concept_id: u32,
    text: String,
}

#[derive(Debug, Deserialize)]
struct RawIcon {
    class_id: u32,
    name: String,
}

/// Dataset loader for RICO data files
pub struct DatasetLoader {
    config: RicoConfig,
    /// 64-dimensional vectors for each screen, indexed by screen_id
    vectors: Array2<f32>,
    /// Screen ID to row index mapping
    screen_to_row: HashMap<u32, usize>,
    /// Metadata for each screen
    metadata: HashMap<u32, ScreenMetadata>,
    /// Whether semantic annotations are loaded
    annotations_loaded: bool,
}

impl DatasetLoader {
    /// Load dataset from configured paths
    pub fn load(config: &RicoConfig) -> Result<Self> {
        info!("Loading RICO dataset from {:?}", config.data_dir);

        // Load vectors
        let vectors = Self::load_vectors(&config.vectors_path())?;
        info!("Loaded {} vectors", vectors.nrows());

        // Build screen ID to row mapping (assuming sequential IDs starting at 0)
        let screen_to_row: HashMap<u32, usize> =
            (0..vectors.nrows()).map(|i| (i as u32, i)).collect();

        // Load metadata
        let mut metadata = Self::load_metadata(&config.metadata_path())?;
        info!("Loaded metadata for {} screens", metadata.len());

        // Try to load semantic annotations (optional)
        let annotations_loaded = if config.annotations_dir().exists() {
            match Self::load_annotations(&config.annotations_dir(), &mut metadata) {
                Ok(count) => {
                    info!("Loaded annotations for {} screens", count);
                    true
                }
                Err(e) => {
                    debug!("Could not load annotations: {}", e);
                    false
                }
            }
        } else {
            debug!("Annotations directory not found, skipping");
            false
        };

        Ok(Self {
            config: config.clone(),
            vectors,
            screen_to_row,
            metadata,
            annotations_loaded,
        })
    }

    /// Load vectors from NPY file
    fn load_vectors(path: &Path) -> Result<Array2<f32>> {
        let file = File::open(path)
            .with_context(|| format!("Failed to open vectors file: {}", path.display()))?;
        let reader = BufReader::new(file);
        let vectors: Array2<f32> =
            Array2::read_npy(reader).with_context(|| "Failed to parse NPY file")?;

        if vectors.ncols() != 64 {
            anyhow::bail!("Expected 64-dimensional vectors, got {}", vectors.ncols());
        }

        Ok(vectors)
    }

    /// Load metadata from JSON file
    fn load_metadata(path: &Path) -> Result<HashMap<u32, ScreenMetadata>> {
        let file = File::open(path)
            .with_context(|| format!("Failed to open metadata file: {}", path.display()))?;
        let reader = BufReader::new(file);
        let raw: Vec<RawMetadata> =
            serde_json::from_reader(reader).with_context(|| "Failed to parse metadata JSON")?;

        let metadata: HashMap<u32, ScreenMetadata> = raw
            .into_iter()
            .map(|r| {
                (
                    r.screen_id,
                    ScreenMetadata {
                        screen_id: r.screen_id,
                        app_package: r.app_package,
                        app_name: r.app_name,
                        components: Vec::new(),
                        text_buttons: Vec::new(),
                        icon_classes: Vec::new(),
                        screenshot_path: None,
                        hierarchy_path: None,
                    },
                )
            })
            .collect();

        Ok(metadata)
    }

    /// Load semantic annotations from directory
    fn load_annotations(dir: &Path, metadata: &mut HashMap<u32, ScreenMetadata>) -> Result<usize> {
        let mut count = 0;

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().is_some_and(|e| e == "json") {
                if let Some(screen_id) = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .and_then(|s| s.parse::<u32>().ok())
                {
                    if let Some(meta) = metadata.get_mut(&screen_id) {
                        if let Ok(annotation) = Self::load_annotation(&path) {
                            meta.components = annotation
                                .components
                                .into_iter()
                                .map(|c| ComponentClass {
                                    class_id: c.class,
                                    name: crate::types::COMPONENT_TYPES
                                        .get(c.class as usize)
                                        .map(|s| s.to_string())
                                        .unwrap_or_else(|| format!("Unknown({})", c.class)),
                                    confidence: c.confidence,
                                })
                                .collect();

                            meta.text_buttons = annotation
                                .text_buttons
                                .into_iter()
                                .map(|t| TextButtonConcept {
                                    concept_id: t.concept_id,
                                    name: t.text,
                                })
                                .collect();

                            meta.icon_classes = annotation
                                .icons
                                .into_iter()
                                .map(|i| IconClass {
                                    class_id: i.class_id,
                                    name: i.name,
                                })
                                .collect();

                            count += 1;
                        }
                    }
                }
            }
        }

        Ok(count)
    }

    fn load_annotation(path: &Path) -> Result<RawAnnotation> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let annotation: RawAnnotation = serde_json::from_reader(reader)?;
        Ok(annotation)
    }

    /// Get vector for a screen ID
    pub fn get_vector(&self, screen_id: u32) -> Option<LayoutVector> {
        self.screen_to_row.get(&screen_id).map(|&row| {
            let row_view: ArrayView1<f32> = self.vectors.row(row);
            let arr: Vec<f32> = row_view.iter().copied().collect();
            LayoutVector(arr)
        })
    }

    /// Get metadata for a screen ID
    pub fn get_metadata(&self, screen_id: u32) -> Option<&ScreenMetadata> {
        self.metadata.get(&screen_id)
    }

    /// Get all vectors as a reference to the array
    pub fn all_vectors(&self) -> &Array2<f32> {
        &self.vectors
    }

    /// Get screen ID for a row index
    pub fn row_to_screen(&self, row: usize) -> Option<u32> {
        self.screen_to_row
            .iter()
            .find(|(_, &r)| r == row)
            .map(|(&id, _)| id)
    }

    /// Number of screens loaded
    pub fn screen_count(&self) -> usize {
        self.vectors.nrows()
    }

    /// Whether annotations are loaded
    pub fn has_annotations(&self) -> bool {
        self.annotations_loaded
    }

    /// Check if screenshot exists for screen
    pub fn screenshot_exists(&self, screen_id: u32) -> bool {
        self.config.screenshot_path(screen_id).exists()
    }

    /// Get screenshot path if it exists
    pub fn screenshot_path(&self, screen_id: u32) -> Option<String> {
        let path = self.config.screenshot_path(screen_id);
        if path.exists() {
            Some(path.to_string_lossy().to_string())
        } else {
            None
        }
    }

    /// Get all screen IDs
    pub fn screen_ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.screen_to_row.keys().copied()
    }
}
