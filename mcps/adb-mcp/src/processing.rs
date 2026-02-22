//! Screenshot image processing pipeline.
//!
//! Converts raw PNG screenshots to optimized JPEG (or PNG) with optional
//! cropping and downsampling, reducing typical 10MB screenshots to ~200-400KB.

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat};
use std::io::Cursor;

/// Options controlling screenshot post-processing.
#[derive(Debug, Clone)]
pub struct ProcessOptions {
    /// Output format: `"jpeg"` or `"png"`.
    pub format: String,
    /// JPEG quality (1-100). Ignored for PNG.
    pub quality: u8,
    /// Maximum output width. Image is downscaled (preserving aspect ratio) if
    /// wider than this value. `0` disables resizing.
    pub max_width: u32,
    /// Maximum output height. Image is downscaled (preserving aspect ratio) if
    /// taller than this value. `0` disables height constraint.
    pub max_height: u32,
    /// Optional crop region applied before resizing.
    pub crop: Option<CropRect>,
}

/// Pixel-coordinate crop rectangle.
#[derive(Debug, Clone)]
pub struct CropRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Result of processing a screenshot.
pub struct ProcessedImage {
    /// Encoded image bytes (JPEG or PNG).
    pub data: Vec<u8>,
    /// MIME type of the output (`image/jpeg` or `image/png`).
    pub mime_type: &'static str,
    /// Output width in pixels.
    pub width: u32,
    /// Output height in pixels.
    pub height: u32,
}

impl Default for ProcessOptions {
    fn default() -> Self {
        Self {
            format: "jpeg".into(),
            quality: 80,
            max_width: 1024,
            max_height: 1920,
            crop: None,
        }
    }
}

/// Process a raw PNG screenshot: decode → crop → resize → re-encode.
pub fn process_screenshot(
    png_data: &[u8],
    opts: &ProcessOptions,
) -> Result<ProcessedImage, String> {
    // Decode
    let img = image::load_from_memory_with_format(png_data, ImageFormat::Png)
        .map_err(|e| format!("Failed to decode PNG: {e}"))?;

    // Crop (optional)
    let img = if let Some(crop) = &opts.crop {
        apply_crop(&img, crop)?
    } else {
        img
    };

    // Resize if either dimension exceeds its limit (preserving aspect ratio)
    let img = {
        let scale_w = if opts.max_width > 0 && img.width() > opts.max_width {
            opts.max_width as f64 / img.width() as f64
        } else {
            1.0
        };
        let scale_h = if opts.max_height > 0 && img.height() > opts.max_height {
            opts.max_height as f64 / img.height() as f64
        } else {
            1.0
        };
        let scale = scale_w.min(scale_h);
        if scale < 1.0 {
            let new_width = (img.width() as f64 * scale) as u32;
            let new_height = (img.height() as f64 * scale) as u32;
            img.resize_exact(new_width, new_height, FilterType::Lanczos3)
        } else {
            img
        }
    };

    let (width, height) = (img.width(), img.height());

    // Encode
    let (data, mime_type) = encode_image(&img, &opts.format, opts.quality)?;

    Ok(ProcessedImage {
        data,
        mime_type,
        width,
        height,
    })
}

fn apply_crop(img: &DynamicImage, crop: &CropRect) -> Result<DynamicImage, String> {
    if crop.width == 0 || crop.height == 0 {
        return Err("Crop width and height must be > 0".into());
    }

    let (iw, ih) = (img.width(), img.height());

    // Clamp to image bounds
    let x = crop.x.min(iw.saturating_sub(1));
    let y = crop.y.min(ih.saturating_sub(1));
    let w = crop.width.min(iw.saturating_sub(x));
    let h = crop.height.min(ih.saturating_sub(y));

    if w == 0 || h == 0 {
        return Err(format!(
            "Crop region ({},{} {}x{}) is outside image bounds ({}x{})",
            crop.x, crop.y, crop.width, crop.height, iw, ih
        ));
    }

    Ok(img.crop_imm(x, y, w, h))
}

fn encode_image(
    img: &DynamicImage,
    format: &str,
    quality: u8,
) -> Result<(Vec<u8>, &'static str), String> {
    let mut buf = Vec::new();

    match format {
        "jpeg" | "jpg" => {
            let quality = quality.clamp(1, 100);
            let encoder = JpegEncoder::new_with_quality(&mut buf, quality);
            img.write_with_encoder(encoder)
                .map_err(|e| format!("JPEG encode failed: {e}"))?;
            Ok((buf, "image/jpeg"))
        }
        "png" => {
            img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
                .map_err(|e| format!("PNG encode failed: {e}"))?;
            Ok((buf, "image/png"))
        }
        other => Err(format!(
            "Unsupported format \"{other}\", use \"jpeg\" or \"png\""
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a minimal valid PNG in memory for testing.
    fn make_test_png(width: u32, height: u32) -> Vec<u8> {
        let img = DynamicImage::new_rgba8(width, height);
        let mut buf = Vec::new();
        img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .expect("encode test PNG");
        buf
    }

    #[test]
    fn noop_passthrough_png() {
        let png = make_test_png(100, 200);
        let opts = ProcessOptions {
            format: "png".into(),
            quality: 80,
            max_width: 0, // disable resize
            max_height: 0,
            crop: None,
        };
        let result = process_screenshot(&png, &opts).unwrap();
        assert_eq!(result.mime_type, "image/png");
        assert_eq!(result.width, 100);
        assert_eq!(result.height, 200);
    }

    #[test]
    fn resize_only() {
        let png = make_test_png(2000, 4000);
        let opts = ProcessOptions {
            format: "jpeg".into(),
            quality: 80,
            max_width: 1000,
            max_height: 0,
            crop: None,
        };
        let result = process_screenshot(&png, &opts).unwrap();
        assert_eq!(result.width, 1000);
        assert_eq!(result.height, 2000);
        assert_eq!(result.mime_type, "image/jpeg");
    }

    #[test]
    fn no_upscale_when_smaller() {
        let png = make_test_png(500, 1000);
        let opts = ProcessOptions {
            format: "jpeg".into(),
            quality: 80,
            max_width: 1024,
            max_height: 0,
            crop: None,
        };
        let result = process_screenshot(&png, &opts).unwrap();
        assert_eq!(result.width, 500);
        assert_eq!(result.height, 1000);
    }

    #[test]
    fn crop_and_resize() {
        let png = make_test_png(2000, 4000);
        let opts = ProcessOptions {
            format: "jpeg".into(),
            quality: 80,
            max_width: 500,
            max_height: 0,
            crop: Some(CropRect {
                x: 0,
                y: 0,
                width: 1000,
                height: 2000,
            }),
        };
        let result = process_screenshot(&png, &opts).unwrap();
        assert_eq!(result.width, 500);
        assert_eq!(result.height, 1000);
    }

    #[test]
    fn crop_clamped_to_bounds() {
        let png = make_test_png(100, 100);
        let opts = ProcessOptions {
            format: "png".into(),
            quality: 80,
            max_width: 0,
            max_height: 0,
            crop: Some(CropRect {
                x: 50,
                y: 50,
                width: 200, // exceeds bounds — clamped to 50
                height: 200,
            }),
        };
        let result = process_screenshot(&png, &opts).unwrap();
        assert_eq!(result.width, 50);
        assert_eq!(result.height, 50);
    }

    #[test]
    fn zero_crop_dimensions_rejected() {
        let png = make_test_png(100, 100);
        let opts = ProcessOptions {
            format: "png".into(),
            quality: 80,
            max_width: 0,
            max_height: 0,
            crop: Some(CropRect {
                x: 0,
                y: 0,
                width: 0,
                height: 50,
            }),
        };
        assert!(process_screenshot(&png, &opts).is_err());
    }

    #[test]
    fn disable_resize_with_zero() {
        let png = make_test_png(3000, 6000);
        let opts = ProcessOptions {
            format: "jpeg".into(),
            quality: 80,
            max_width: 0,
            max_height: 0,
            crop: None,
        };
        let result = process_screenshot(&png, &opts).unwrap();
        assert_eq!(result.width, 3000);
        assert_eq!(result.height, 6000);
    }

    #[test]
    fn invalid_format_rejected() {
        let png = make_test_png(100, 100);
        let opts = ProcessOptions {
            format: "webp".into(),
            quality: 80,
            max_width: 0,
            max_height: 0,
            crop: None,
        };
        assert!(process_screenshot(&png, &opts).is_err());
    }

    #[test]
    fn height_constrained_galaxy_s22() {
        // 1080×2340 phone: width fits in 1024 after width-scale, but height still > 1920
        let png = make_test_png(1080, 2340);
        let opts = ProcessOptions {
            format: "jpeg".into(),
            quality: 80,
            max_width: 1024,
            max_height: 1920,
            crop: None,
        };
        let result = process_screenshot(&png, &opts).unwrap();
        // Height is the binding constraint: scale = 1920/2340 ≈ 0.8205
        assert!(result.height <= 1920, "height {} > 1920", result.height);
        assert!(result.width <= 1024, "width {} > 1024", result.width);
    }

    #[test]
    fn height_constrained_fold() {
        // Galaxy Z Fold: 904×2316 — width < 1024 so no width resize, but height > 1920
        let png = make_test_png(904, 2316);
        let opts = ProcessOptions {
            format: "jpeg".into(),
            quality: 80,
            max_width: 1024,
            max_height: 1920,
            crop: None,
        };
        let result = process_screenshot(&png, &opts).unwrap();
        assert!(result.height <= 1920, "height {} > 1920", result.height);
        assert!(result.width <= 904, "width {} > 904", result.width);
    }

    #[test]
    fn both_limits_zero_no_resize() {
        let png = make_test_png(3000, 6000);
        let opts = ProcessOptions {
            format: "jpeg".into(),
            quality: 80,
            max_width: 0,
            max_height: 0,
            crop: None,
        };
        let result = process_screenshot(&png, &opts).unwrap();
        assert_eq!(result.width, 3000);
        assert_eq!(result.height, 6000);
    }

    #[test]
    fn both_formats_encode_successfully() {
        let png = make_test_png(500, 500);
        let png_result = process_screenshot(
            &png,
            &ProcessOptions {
                format: "png".into(),
                max_width: 0,
                ..Default::default()
            },
        )
        .unwrap();
        let jpeg_result = process_screenshot(
            &png,
            &ProcessOptions {
                format: "jpeg".into(),
                max_width: 0,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(png_result.mime_type, "image/png");
        assert_eq!(jpeg_result.mime_type, "image/jpeg");
        assert!(!png_result.data.is_empty());
        assert!(!jpeg_result.data.is_empty());
    }
}
