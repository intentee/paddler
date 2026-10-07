use crate::normalized_probabilities::normalized_probabilities;

#[expect(
    clippy::cast_precision_loss,
    reason = "option counts stay within the 255 options a TypeSafe question may carry"
)]
#[must_use]
pub fn choice_confidence(probabilities: &[f64]) -> f64 {
    let option_count = probabilities.len() as f64;

    if probabilities.len() == 1 {
        return 1.0;
    }

    let largest_probability = normalized_probabilities(probabilities)
        .into_iter()
        .fold(f64::NEG_INFINITY, f64::max);

    (largest_probability - 1.0 / option_count) / (1.0 - 1.0 / option_count)
}
