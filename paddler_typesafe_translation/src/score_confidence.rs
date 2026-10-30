use std::num::NonZeroU8;

#[must_use]
pub fn score_confidence(probabilities: &[f64], level_count: NonZeroU8) -> f64 {
    if level_count == NonZeroU8::MIN {
        return 1.0;
    }

    let level_count_f64 = f64::from(level_count.get());
    let mut most_likely_level: u8 = 0;

    for (level, probability) in (0..=u8::MAX).zip(probabilities) {
        if *probability > probabilities[usize::from(most_likely_level)] {
            most_likely_level = level;
        }
    }

    let uniform_deviation = (0..level_count.get())
        .map(|level| (f64::from(level) - (level_count_f64 - 1.0) / 2.0).abs())
        .sum::<f64>()
        / level_count_f64;
    let expected_deviation: f64 = (0..=u8::MAX)
        .zip(probabilities)
        .map(|(level, probability)| {
            probability * (f64::from(level) - f64::from(most_likely_level)).abs()
        })
        .sum();

    (1.0 - expected_deviation / uniform_deviation).max(0.0)
}
