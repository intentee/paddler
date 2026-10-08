use crate::normalized_probabilities::normalized_probabilities;

#[expect(
    clippy::cast_precision_loss,
    reason = "option counts stay within the 255 options a TypeSafe question may carry"
)]
#[must_use]
pub fn choice_confidence(probabilities: &[f64], most_likely_option: usize) -> f64 {
    let option_count = probabilities.len() as f64;

    if probabilities.len() == 1 {
        return 1.0;
    }

    let largest_probability = normalized_probabilities(probabilities)[most_likely_option];

    (largest_probability - 1.0 / option_count) / (1.0 - 1.0 / option_count)
}

#[cfg(test)]
mod tests {
    use super::choice_confidence;

    #[test]
    fn measures_the_confidence_on_normalized_probabilities() {
        assert!((choice_confidence(&[1.0, 3.0], 1) - 0.5).abs() < f64::EPSILON);
    }
}
