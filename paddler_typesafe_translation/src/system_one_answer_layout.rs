use std::num::NonZeroU8;

use serde_json::Map;
use serde_json::Value;

use crate::choice_confidence::choice_confidence;
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

fn most_likely_option(probabilities: &[f64]) -> usize {
    let mut most_likely = 0;

    for (option, probability) in probabilities.iter().enumerate() {
        if *probability > probabilities[most_likely] {
            most_likely = option;
        }
    }

    most_likely
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
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::Choice { id, .. } | Self::Noul { id } | Self::Score { id, .. } => id,
        }
    }

    pub fn answer(
        &self,
        probabilities: &[f32],
    ) -> Result<SystemOneAnswer, TypeSafeTranslationError> {
        let probabilities: Vec<f64> = probabilities.iter().copied().map(f64::from).collect();
        let expected = match self {
            Self::Choice { option_count, .. } => usize::from(option_count.get()),
            Self::Noul { .. } => NOUL_OPTIONS,
            Self::Score { level_count, .. } => usize::from(level_count.get()),
        };

        if probabilities.len() != expected {
            return Err(TypeSafeTranslationError::AnswerProbabilitiesMismatch {
                expected,
                question_id: self.id().to_owned(),
                received: probabilities.len(),
            });
        }

        Ok(match self {
            Self::Choice {
                keys, option_count, ..
            } => {
                let most_likely = most_likely_option(&probabilities);

                SystemOneAnswer::Choice {
                    choice: keys[most_likely].clone(),
                    confidence: round_probability(choice_confidence(
                        &probabilities,
                        most_likely,
                        *option_count,
                    )),
                    probabilities: rounded_probabilities(keys.iter(), &probabilities),
                }
            }
            Self::Noul { .. } => SystemOneAnswer::Noul {
                noul: round_probability(probabilities[1]),
            },
            Self::Score {
                legend,
                level_count,
                ..
            } => {
                let level_keys: Vec<String> = (0..level_count.get())
                    .map(|level| level.to_string())
                    .collect();
                let score: f64 = (0..=u8::MAX)
                    .zip(&probabilities)
                    .map(|(level, probability)| f64::from(level) * probability)
                    .sum();

                SystemOneAnswer::Score {
                    score: round_probability(score),
                    legend: level_keys
                        .iter()
                        .cloned()
                        .zip(legend.iter().cloned().map(Value::from))
                        .collect(),
                    probabilities: rounded_probabilities(level_keys.iter(), &probabilities),
                    confidence: round_probability(score_confidence(&probabilities, *level_count)),
                }
            }
        })
    }
}
