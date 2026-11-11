use std::num::NonZeroU8;

use serde_json::Map;
use serde_json::Value;

use crate::choice_confidence::choice_confidence;
use crate::most_likely_option::MostLikelyOption;
use crate::round_probability::round_probability;
use crate::score_confidence::score_confidence;
use crate::system_one_answer::SystemOneAnswer;
use crate::typesafe_translation_error::TypeSafeTranslationError;

const NOUL_OPTIONS: usize = 2;

fn rounded_probabilities<'keys>(
    keys: impl Iterator<Item = &'keys String>,
    probabilities: &[f64],
) -> Map<String, Value> {
    keys.zip(probabilities)
        .map(|(key, probability)| (key.clone(), Value::from(round_probability(*probability))))
        .collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SystemOneAnswerLayout {
    Choice {
        id: String,
        keys: Vec<String>,
        option_count: NonZeroU8,
    },
    Noul {
        id: String,
    },
    Score {
        id: String,
        legend: Vec<String>,
        level_count: NonZeroU8,
    },
}

impl SystemOneAnswerLayout {
    /// # Errors
    ///
    /// Returns [`TypeSafeTranslationError::AnswerProbabilitiesMismatch`] when the number of probabilities differs from the number of options the question offers.
    pub fn answer(
        &self,
        probabilities: &[f32],
    ) -> Result<SystemOneAnswer, TypeSafeTranslationError> {
        let probabilities: Vec<f64> = probabilities.iter().copied().map(f64::from).collect();

        self.matching_answer(&probabilities).ok_or_else(|| {
            TypeSafeTranslationError::AnswerProbabilitiesMismatch {
                expected: self.expected_probability_count(),
                question_id: self.id().to_owned(),
                received: probabilities.len(),
            }
        })
    }

    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::Choice { id, .. } | Self::Noul { id } | Self::Score { id, .. } => id,
        }
    }

    fn expected_probability_count(&self) -> usize {
        match self {
            Self::Choice { option_count, .. } => usize::from(option_count.get()),
            Self::Noul { .. } => NOUL_OPTIONS,
            Self::Score { level_count, .. } => usize::from(level_count.get()),
        }
    }

    fn matching_answer(&self, probabilities: &[f64]) -> Option<SystemOneAnswer> {
        match self {
            Self::Choice {
                keys, option_count, ..
            } => {
                let MostLikelyOption {
                    option: choice,
                    probability,
                } = MostLikelyOption::among(keys.iter(), probabilities)?;

                Some(SystemOneAnswer::Choice {
                    choice: choice.clone(),
                    confidence: round_probability(choice_confidence(probability, *option_count)),
                    probabilities: rounded_probabilities(keys.iter(), probabilities),
                })
            }
            Self::Noul { .. } => {
                let &[_, yes_probability] = probabilities else {
                    return None;
                };

                Some(SystemOneAnswer::Noul {
                    noul: round_probability(yes_probability),
                })
            }
            Self::Score {
                legend,
                level_count,
                ..
            } => {
                let MostLikelyOption {
                    option: most_likely_level,
                    ..
                } = MostLikelyOption::among(0..level_count.get(), probabilities)?;
                let level_keys: Vec<String> = (0..level_count.get())
                    .map(|level| level.to_string())
                    .collect();
                let score: f64 = (0..=u8::MAX)
                    .zip(probabilities)
                    .map(|(level, probability)| f64::from(level) * probability)
                    .sum();

                Some(SystemOneAnswer::Score {
                    score: round_probability(score),
                    legend: level_keys
                        .iter()
                        .cloned()
                        .zip(legend.iter().cloned().map(Value::from))
                        .collect(),
                    probabilities: rounded_probabilities(level_keys.iter(), probabilities),
                    confidence: round_probability(score_confidence(
                        probabilities,
                        most_likely_level,
                        *level_count,
                    )),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU8;

    use super::SystemOneAnswerLayout;
    use crate::typesafe_translation_error::TypeSafeTranslationError;

    #[test]
    fn a_choice_answer_whose_probabilities_do_not_match_its_options_is_refused() {
        let layout = SystemOneAnswerLayout::Choice {
            id: "color".to_owned(),
            keys: vec!["blue".to_owned(), "red".to_owned()],
            option_count: NonZeroU8::new(2).expect("two options are a valid count"),
        };

        assert!(matches!(
            layout.answer(&[1.0]),
            Err(TypeSafeTranslationError::AnswerProbabilitiesMismatch {
                expected: 2,
                question_id,
                received: 1,
            }) if question_id == "color"
        ));
    }

    #[test]
    fn a_score_answer_whose_probabilities_do_not_match_its_levels_is_refused() {
        let layout = SystemOneAnswerLayout::Score {
            id: "rating".to_owned(),
            legend: vec!["low".to_owned(), "mid".to_owned(), "high".to_owned()],
            level_count: NonZeroU8::new(3).expect("three levels are a valid count"),
        };

        assert!(matches!(
            layout.answer(&[0.5, 0.5]),
            Err(TypeSafeTranslationError::AnswerProbabilitiesMismatch {
                expected: 3,
                question_id,
                received: 2,
            }) if question_id == "rating"
        ));
    }
}
