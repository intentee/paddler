use std::num::NonZeroU8;

#[must_use]
pub fn choice_confidence(
    probabilities: &[f64],
    most_likely_option: usize,
    option_count: NonZeroU8,
) -> f64 {
    if option_count == NonZeroU8::MIN {
        return 1.0;
    }

    let uniform_probability = 1.0 / f64::from(option_count.get());
    let largest_probability = probabilities[most_likely_option];

    (largest_probability - uniform_probability) / (1.0 - uniform_probability)
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU8;

    use super::choice_confidence;

    #[test]
    fn measures_the_confidence_on_normalized_probabilities() {
        let option_count = NonZeroU8::new(2).expect("two options are a valid count");

        assert!((choice_confidence(&[0.25, 0.75], 1, option_count) - 0.5).abs() < f64::EPSILON);
    }
}
