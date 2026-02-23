//! Layout vector encoding for RICO-compatible 64-dimensional feature vectors.
//!
//! Pure Rust replacement for `scripts/rico-encode.py`. Converts a screenshot
//! into a 64-dim layout vector using grid-based feature extraction:
//!
//! 1. Convert to grayscale
//! 2. Resize to 256x256 (Lanczos3)
//! 3. Divide into 8x8 grid (32px cells)
//! 4. Per cell: `feature = 0.4 * mean + 0.3 * sqrt(variance) + 0.3 * edge_density`
//! 5. L2-normalize to unit vector

use image::imageops::FilterType;
use image::DynamicImage;

const GRID_SIZE: usize = 8;
const TARGET_SIZE: u32 = (GRID_SIZE * 32) as u32; // 256
const CELL_SIZE: usize = 32;

/// Encode a `DynamicImage` into a 64-dimensional layout vector.
pub fn encode_layout_vector(img: &DynamicImage) -> [f32; 64] {
    // Grayscale + resize to 256x256
    let gray = img.to_luma8();
    let resized = image::imageops::resize(&gray, TARGET_SIZE, TARGET_SIZE, FilterType::Lanczos3);

    let mut features = [0.0f32; 64];

    for row in 0..GRID_SIZE {
        for col in 0..GRID_SIZE {
            let y1 = row * CELL_SIZE;
            let x1 = col * CELL_SIZE;

            // Extract cell pixels as f32 normalized to [0, 1]
            let mut cell = [[0.0f32; CELL_SIZE]; CELL_SIZE];
            for (dy, cell_row) in cell.iter_mut().enumerate() {
                for (dx, pixel) in cell_row.iter_mut().enumerate() {
                    *pixel =
                        resized.get_pixel((x1 + dx) as u32, (y1 + dy) as u32).0[0] as f32
                            / 255.0;
                }
            }

            // Mean intensity
            let n = (CELL_SIZE * CELL_SIZE) as f32;
            let mean_val: f32 = cell.iter().flat_map(|r| r.iter()).sum::<f32>() / n;

            // Variance
            let var_val: f32 = cell
                .iter()
                .flat_map(|r| r.iter())
                .map(|&v| (v - mean_val) * (v - mean_val))
                .sum::<f32>()
                / n;

            // Edge density via gradients (matches np.diff behavior)
            // grad_x = mean of |diff along columns| (axis=1 in numpy)
            let mut grad_x_sum = 0.0f32;
            let mut grad_x_count = 0u32;
            for row_pixels in &cell {
                for pair in row_pixels.windows(2) {
                    grad_x_sum += (pair[1] - pair[0]).abs();
                    grad_x_count += 1;
                }
            }
            let grad_x = if grad_x_count > 0 {
                grad_x_sum / grad_x_count as f32
            } else {
                0.0
            };

            // grad_y = mean of |diff along rows| (axis=0 in numpy)
            let mut grad_y_sum = 0.0f32;
            let mut grad_y_count = 0u32;
            for row_pair in cell.windows(2) {
                for (above, below) in row_pair[0].iter().zip(row_pair[1].iter()) {
                    grad_y_sum += (below - above).abs();
                    grad_y_count += 1;
                }
            }
            let grad_y = if grad_y_count > 0 {
                grad_y_sum / grad_y_count as f32
            } else {
                0.0
            };

            let edge_val = (grad_x + grad_y) / 2.0;

            // Combine features (weighted)
            let feature = 0.4 * mean_val + 0.3 * var_val.sqrt() + 0.3 * edge_val;
            features[row * GRID_SIZE + col] = feature;
        }
    }

    // L2-normalize
    let norm: f32 = features.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 1e-8 {
        for v in features.iter_mut() {
            *v /= norm;
        }
    } else {
        // Uniform fallback
        let uniform = 1.0 / (64.0f32).sqrt();
        features.fill(uniform);
    }

    features
}

/// Encode layout vector from a file path.
pub fn encode_layout_vector_from_file(path: &str) -> Result<[f32; 64], String> {
    let img = super::imaging::load_image_file(path)?;
    Ok(encode_layout_vector(&img))
}

/// Encode layout vector from raw image bytes.
pub fn encode_layout_vector_from_bytes(data: &[u8]) -> Result<[f32; 64], String> {
    let img = super::imaging::decode_image(data)?;
    Ok(encode_layout_vector(&img))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::DynamicImage;

    #[test]
    fn uniform_image_produces_unit_vector() {
        // A solid gray image should produce a valid unit vector
        let img = DynamicImage::new_luma8(256, 256);
        let vec = encode_layout_vector(&img);

        // Should be 64 elements
        assert_eq!(vec.len(), 64);

        // L2 norm should be ~1.0
        let norm: f32 = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!(
            (norm - 1.0).abs() < 1e-5,
            "Expected unit vector, got norm {}",
            norm
        );
    }

    #[test]
    fn different_images_produce_different_vectors() {
        // All-black vs all-white
        let black = DynamicImage::new_luma8(100, 100);
        let mut white_buf = image::GrayImage::new(100, 100);
        for p in white_buf.pixels_mut() {
            p.0[0] = 255;
        }
        let white = DynamicImage::ImageLuma8(white_buf);

        let v1 = encode_layout_vector(&black);
        let _v2 = encode_layout_vector(&white);

        // Uniform images should both produce uniform vectors (so they'll be similar)
        // but the values should differ in magnitude before normalization
        // After normalization, both uniform images map to the same direction
        // so let's test with a half-black/half-white image instead
        let mut half = image::GrayImage::new(100, 100);
        for y in 0..100 {
            for x in 0..100 {
                half.put_pixel(x, y, image::Luma([if x < 50 { 0 } else { 255 }]));
            }
        }
        let v3 = encode_layout_vector(&DynamicImage::ImageLuma8(half));

        // v3 should differ from v1 (all-black)
        let cosine: f32 = v1.iter().zip(v3.iter()).map(|(a, b)| a * b).sum();
        assert!(
            cosine < 0.999,
            "Half image should differ from uniform, cosine={}",
            cosine
        );
    }

    #[test]
    fn small_image_works() {
        // Even a 1x1 image should not panic
        let img = DynamicImage::new_luma8(1, 1);
        let vec = encode_layout_vector(&img);
        let norm: f32 = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }

    #[test]
    fn large_image_works() {
        let img = DynamicImage::new_luma8(1080, 1920);
        let vec = encode_layout_vector(&img);
        let norm: f32 = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }

    #[test]
    fn encode_from_bytes_works() {
        use std::io::Cursor;
        let img = DynamicImage::new_rgba8(100, 200);
        let mut buf = Vec::new();
        img.write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
            .unwrap();
        let vec = encode_layout_vector_from_bytes(&buf).unwrap();
        let norm: f32 = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }
}
