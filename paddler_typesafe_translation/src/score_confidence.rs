use crate::normalized_probabilities::normalized_probabilities;

#[expect(
    clippy::cast_precision_loss,
    reason = "score levels stay within the 255 levels a TypeSafe question may carry"
)]
#[must_use]
pub fn score_confidence(probabilities: &[f64]) -> f64 {
    if probabilities.len() == 1 {
        return 1.0;
    }

    let level_count = probabilities.len() as f64;
    let normalized = normalized_probabilities(probabilities);
    let mut most_likely_level = 0;

    for (level, probability) in normalized.iter().enumerate() {
        if *probability > normalized[most_likely_level] {
            most_likely_level = level;
        }
    }

    let uniform_deviation = (0..probabilities.len())
        .map(|level| (level as f64 - (level_count - 1.0) / 2.0).abs())
        .sum::<f64>()
        / level_count;
    let expected_deviation: f64 = normalized
        .iter()
        .enumerate()
        .map(|(level, probability)| probability * (level as f64 - most_likely_level as f64).abs())
        .sum();

    (1.0 - expected_deviation / uniform_deviation).max(0.0)
}
