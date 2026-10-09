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
    Choice { id: String, keys: Vec<String> },
    Noul { id: String },
    Score { id: String, legend: Vec<String> },
}

impl SystemOneAnswerLayout {
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::Choice { id, .. } | Self::Noul { id } | Self::Score { id, .. } => id,
        }
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "score levels stay within the 255 levels a TypeSafe question may carry"
    )]
    pub fn answer(
        &self,
        probabilities: &[f32],
    ) -> Result<SystemOneAnswer, TypeSafeTranslationError> {
        let probabilities: Vec<f64> = probabilities.iter().copied().map(f64::from).collect();
        let expected = match self {
            Self::Choice { keys, .. } => keys.len(),
            Self::Noul { .. } => NOUL_OPTIONS,
            Self::Score { legend, .. } => legend.len(),
        };

        if probabilities.len() != expected {
            return Err(TypeSafeTranslationError::AnswerProbabilitiesMismatch {
                expected,
                question_id: self.id().to_owned(),
                received: probabilities.len(),
            });
        }

        Ok(match self {
            Self::Choice { keys, .. } => {
                let most_likely = most_likely_option(&probabilities);

                SystemOneAnswer::Choice {
                    choice: keys[most_likely].clone(),
                    confidence: round_probability(choice_confidence(&probabilities, most_likely)),
                    probabilities: rounded_probabilities(keys.iter(), &probabilities),
                }
            }
            Self::Noul { .. } => SystemOneAnswer::Noul {
                noul: round_probability(probabilities[1]),
            },
            Self::Score { legend, .. } => {
                let level_keys: Vec<String> =
                    (0..legend.len()).map(|level| level.to_string()).collect();
                let score: f64 = probabilities
                    .iter()
                    .enumerate()
                    .map(|(level, probability)| level as f64 * probability)
                    .sum();

                SystemOneAnswer::Score {
                    score: round_probability(score),
                    legend: level_keys
                        .iter()
                        .cloned()
                        .zip(legend.iter().cloned().map(Value::from))
                        .collect(),
                    probabilities: rounded_probabilities(level_keys.iter(), &probabilities),
                    confidence: round_probability(score_confidence(&probabilities)),
                }
            }
        })
    }
}
