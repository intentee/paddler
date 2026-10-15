const PROBABILITY_DECIMAL_SCALE: f64 = 10_000.0;

#[must_use]
pub fn round_probability(value: f64) -> f64 {
    (value * PROBABILITY_DECIMAL_SCALE).round_ties_even() / PROBABILITY_DECIMAL_SCALE
}

#[cfg(test)]
mod tests {
    use super::round_probability;

    #[test]
    fn keeps_four_decimals_and_rounds_exact_ties_to_even() {
        assert!((round_probability(0.876_543_22) - 0.8765).abs() < f64::EPSILON);
        assert!((round_probability(0.031_25) - 0.0312).abs() < f64::EPSILON);
    }
}
