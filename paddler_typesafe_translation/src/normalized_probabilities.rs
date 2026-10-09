#[expect(
    clippy::cast_precision_loss,
    reason = "option counts stay within the 255 options a TypeSafe question may carry"
)]
#[must_use]
pub fn normalized_probabilities(probabilities: &[f64]) -> Vec<f64> {
    let total: f64 = probabilities.iter().sum();

    if total == 0.0 {
        let uniform_probability = 1.0 / probabilities.len() as f64;

        return vec![uniform_probability; probabilities.len()];
    }

    probabilities
        .iter()
        .map(|probability| probability / total)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::normalized_probabilities;

    #[test]
    fn scales_probabilities_to_sum_to_one() {
        assert_eq!(normalized_probabilities(&[1.0, 3.0]), vec![0.25, 0.75]);
    }

    #[test]
    fn spreads_missing_probability_mass_evenly() {
        assert_eq!(normalized_probabilities(&[0.0, 0.0]), vec![0.5, 0.5]);
    }
}
