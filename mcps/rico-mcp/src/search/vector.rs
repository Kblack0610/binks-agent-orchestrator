//! Cosine similarity search for UI layout vectors

use ndarray::{Array1, Axis};
use ordered_float::OrderedFloat;
use rayon::prelude::*;

use crate::dataset::DatasetLoader;
use crate::types::SimilarityResult;

/// Vector similarity search engine
pub struct VectorSearch<'a> {
    loader: &'a DatasetLoader,
}

impl<'a> VectorSearch<'a> {
    /// Create a new search engine with the given dataset
    pub fn new(loader: &'a DatasetLoader) -> Self {
        Self { loader }
    }

    /// Find top-k most similar screens to the query vector
    pub fn search(
        &self,
        query: &[f32; 64],
        top_k: usize,
        min_similarity: f32,
        component_filter: Option<&[String]>,
    ) -> Vec<SimilarityResult> {
        let query_vec = Array1::from_vec(query.to_vec());
        let query_norm = l2_norm(&query_vec);

        if query_norm == 0.0 {
            return Vec::new();
        }

        let vectors = self.loader.all_vectors();

        // Compute similarities in parallel
        let mut similarities: Vec<(u32, f32)> = vectors
            .axis_iter(Axis(0))
            .into_par_iter()
            .enumerate()
            .filter_map(|(row, row_vec)| {
                let screen_id = self.loader.row_to_screen(row)?;

                // Apply component filter if specified
                if let Some(filter) = component_filter {
                    if let Some(meta) = self.loader.get_metadata(screen_id) {
                        let component_names: Vec<String> = meta.component_names();
                        let has_match = filter.iter().any(|f| {
                            component_names.iter().any(|c| c.to_lowercase().contains(&f.to_lowercase()))
                        });
                        if !has_match {
                            return None;
                        }
                    }
                }

                let row_norm = l2_norm_view(&row_vec);
                if row_norm == 0.0 {
                    return None;
                }

                let dot_product: f32 = query_vec.iter().zip(row_vec.iter()).map(|(a, b)| a * b).sum();
                let similarity = dot_product / (query_norm * row_norm);

                if similarity >= min_similarity {
                    Some((screen_id, similarity))
                } else {
                    None
                }
            })
            .collect();

        // Sort by similarity descending
        similarities.sort_by_key(|(_, sim)| std::cmp::Reverse(OrderedFloat(*sim)));

        // Take top-k and convert to results
        similarities
            .into_iter()
            .take(top_k)
            .map(|(screen_id, similarity)| {
                let meta = self.loader.get_metadata(screen_id);
                SimilarityResult {
                    screen_id,
                    similarity_score: similarity,
                    app_name: meta.and_then(|m| m.app_name.clone()),
                    app_package: meta
                        .map(|m| m.app_package.clone())
                        .unwrap_or_else(|| "unknown".to_string()),
                    components: meta
                        .map(|m| m.component_names())
                        .unwrap_or_default(),
                    screenshot_available: self.loader.screenshot_exists(screen_id),
                }
            })
            .collect()
    }

    /// Find screens similar to a given screen ID
    pub fn search_by_screen(
        &self,
        screen_id: u32,
        top_k: usize,
        min_similarity: f32,
    ) -> Option<Vec<SimilarityResult>> {
        let vector = self.loader.get_vector(screen_id)?;
        let mut results = self.search(&vector.as_array(), top_k + 1, min_similarity, None);

        // Remove the query screen itself if present
        results.retain(|r| r.screen_id != screen_id);

        // Ensure we only return top_k
        results.truncate(top_k);

        Some(results)
    }

    /// Compute average similarity between a set of screens (for flow cohesion)
    pub fn flow_cohesion(&self, screen_ids: &[u32]) -> Option<f32> {
        if screen_ids.len() < 2 {
            return None;
        }

        let vectors: Vec<Array1<f32>> = screen_ids
            .iter()
            .filter_map(|&id| {
                self.loader.get_vector(id).map(|v| Array1::from_vec(v.0.to_vec()))
            })
            .collect();

        if vectors.len() < 2 {
            return None;
        }

        let mut total_sim = 0.0f32;
        let mut count = 0;

        for i in 0..vectors.len() {
            for j in (i + 1)..vectors.len() {
                let norm_i = l2_norm(&vectors[i]);
                let norm_j = l2_norm(&vectors[j]);

                if norm_i > 0.0 && norm_j > 0.0 {
                    let dot: f32 = vectors[i].iter().zip(vectors[j].iter()).map(|(a, b)| a * b).sum();
                    total_sim += dot / (norm_i * norm_j);
                    count += 1;
                }
            }
        }

        if count > 0 {
            Some(total_sim / count as f32)
        } else {
            None
        }
    }
}

/// Compute L2 norm of a vector
fn l2_norm(v: &Array1<f32>) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}

/// Compute L2 norm of a view
fn l2_norm_view(v: &ndarray::ArrayView1<f32>) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_l2_norm() {
        let v = Array1::from_vec(vec![3.0, 4.0]);
        assert!((l2_norm(&v) - 5.0).abs() < 1e-6);
    }
}
