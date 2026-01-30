//! Analysis algorithms for pattern detection and health scoring

/// Compute health score for a run
///
/// Formula: weighted_sum([
///     (success_rate, 0.5),           // 50% - most critical
///     (avg_duration_normalized, 0.2), // 20% - performance
///     (tool_reliability, 0.2),        // 20% - stability
///     (resource_efficiency, 0.1)      // 10% - optimization
/// ])
pub fn compute_health_score(
    success_rate: f64,
    avg_duration_normalized: f64,
    tool_reliability: f64,
    resource_efficiency: f64,
) -> f64 {
    let weighted_sum = success_rate * 0.5
        + avg_duration_normalized * 0.2
        + tool_reliability * 0.2
        + resource_efficiency * 0.1;

    // Scale to 0-100
    weighted_sum * 100.0
}

/// Detect if a metric is trending upward, downward, or stable
///
/// Compares recent period (e.g., last 7 days) to historical (e.g., previous 30 days)
/// - Improving: recent > historical + 5%
/// - Degrading: recent < historical - 5%
/// - Stable: within ±5%
pub fn detect_trend(recent_value: f64, historical_value: f64, threshold: f64) -> String {
    let change_percent = ((recent_value - historical_value) / historical_value) * 100.0;

    if change_percent > threshold {
        "improving".to_string()
    } else if change_percent < -threshold {
        "degrading".to_string()
    } else {
        "stable".to_string()
    }
}

/// Compute Jaccard similarity between two sets of contexts
///
/// Used for correlating error patterns based on context similarity
pub fn jaccard_similarity(set1: &[String], set2: &[String]) -> f64 {
    let set1: std::collections::HashSet<_> = set1.iter().collect();
    let set2: std::collections::HashSet<_> = set2.iter().collect();

    let intersection = set1.intersection(&set2).count();
    let union = set1.union(&set2).count();

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Compute statistical confidence for pattern detection
///
/// Returns confidence level (0.0 to 1.0) based on:
/// - Sample size (more occurrences = higher confidence)
/// - Consistency (similar error messages = higher confidence)
/// - Temporal distribution (clustered vs spread out)
pub fn compute_pattern_confidence(
    occurrences: usize,
    context_similarity: f64,
    time_span_days: f64,
) -> f64 {
    // Base confidence from occurrence count
    let size_factor = (occurrences as f64).ln() / 10.0;
    let size_confidence = size_factor.min(1.0);

    // Adjust by context similarity
    let similarity_adjusted = size_confidence * (0.5 + context_similarity * 0.5);

    // Penalize if pattern is too spread out (low frequency)
    let frequency = occurrences as f64 / time_span_days;
    let frequency_factor = if frequency > 1.0 {
        1.0
    } else {
        frequency
    };

    similarity_adjusted * frequency_factor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_score_calculation() {
        let score = compute_health_score(0.9, 0.8, 0.95, 0.85);
        assert!((score - 88.0).abs() < 1.0); // ~88%
    }

    #[test]
    fn test_trend_detection() {
        assert_eq!(detect_trend(0.95, 0.85, 5.0), "improving");
        assert_eq!(detect_trend(0.75, 0.85, 5.0), "degrading");
        assert_eq!(detect_trend(0.87, 0.85, 5.0), "stable");
    }

    #[test]
    fn test_jaccard_similarity() {
        let set1 = vec!["error".to_string(), "timeout".to_string(), "kubernetes".to_string()];
        let set2 = vec!["error".to_string(), "timeout".to_string(), "docker".to_string()];
        let similarity = jaccard_similarity(&set1, &set2);
        assert!((similarity - 0.5).abs() < 0.01); // 2/4 = 0.5
    }

    #[test]
    fn test_pattern_confidence() {
        // High confidence: many occurrences, high similarity, frequent
        let conf1 = compute_pattern_confidence(10, 0.9, 7.0);
        assert!(conf1 > 0.8);

        // Low confidence: few occurrences, low similarity, infrequent
        let conf2 = compute_pattern_confidence(2, 0.3, 30.0);
        assert!(conf2 < 0.3);
    }
}
