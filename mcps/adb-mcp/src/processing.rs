//! Screenshot image processing pipeline.
//!
//! Re-exports shared image processing from `mcp_common::imaging`.

pub use mcp_common::imaging::{
    process_screenshot, to_base64, CropRect, ProcessOptions, ProcessedImage,
};
