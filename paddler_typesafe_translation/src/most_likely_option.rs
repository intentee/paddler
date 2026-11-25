pub struct MostLikelyOption<TOption> {
    pub option: TOption,
    pub probability: f64,
}

impl<TOption> MostLikelyOption<TOption> {
    #[must_use]
    pub fn among<TOptions>(options: TOptions, probabilities: &[f64]) -> Option<Self>
    where
        TOptions: ExactSizeIterator<Item = TOption>,
    {
        if options.len() != probabilities.len() {
            return None;
        }

        options
            .zip(probabilities.iter().copied())
            .map(|(option, probability)| Self {
                option,
                probability,
            })
            .reduce(|most_likely, candidate| {
                if candidate.probability > most_likely.probability {
                    candidate
                } else {
                    most_likely
                }
            })
    }
}
