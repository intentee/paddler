use std::ffi::c_int;

use paddler_messaging::model_metadata::ModelMetadata;

use crate::decision_error::DecisionError;

const ARCHITECTURE_KEY: &str = "general.architecture";

fn softmax(logits: &[f32]) -> Vec<f32> {
    let largest_logit = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let exponentials: Vec<f32> = logits
        .iter()
        .map(|logit| (logit - largest_logit).exp())
        .collect();
    let exponential_sum: f32 = exponentials.iter().sum();

    exponentials
        .iter()
        .map(|exponential| exponential / exponential_sum)
        .collect()
}

fn read_temperature(
    ModelMetadata { metadata }: &ModelMetadata,
    architecture: &str,
    question_type: &str,
) -> Result<f32, DecisionError> {
    let key = format!("{architecture}.decision.temperature.{question_type}");
    let declared = metadata
        .get(&key)
        .ok_or_else(|| DecisionError::KevTemperatureMissing { key: key.clone() })?;
    let temperature =
        declared
            .parse::<f32>()
            .map_err(|source| DecisionError::KevTemperatureUnparseable {
                declared: declared.clone(),
                key: key.clone(),
                source,
            })?;

    if temperature.is_finite() && temperature > 0.0 {
        Ok(temperature)
    } else {
        Err(DecisionError::KevTemperatureOutOfRange { key, temperature })
    }
}

fn read_shared_temperature(model_metadata: &ModelMetadata) -> Result<f32, DecisionError> {
    let architecture = model_metadata
        .metadata
        .get(ARCHITECTURE_KEY)
        .ok_or(DecisionError::KevArchitectureUnknown)?;
    let choice = read_temperature(model_metadata, architecture, "choice")?;
    let noul = read_temperature(model_metadata, architecture, "noul")?;
    let score = read_temperature(model_metadata, architecture, "score")?;

    if choice.to_bits() == noul.to_bits() && choice.to_bits() == score.to_bits() {
        Ok(choice)
    } else {
        Err(DecisionError::KevTemperaturesDiffer {
            choice,
            noul,
            score,
        })
    }
}

pub struct KevOptionScorer {
    head_dim: u16,
    tempered_logit_scale: f32,
}

impl KevOptionScorer {
    /// # Errors
    ///
    /// Returns [`DecisionError::KevHeadDimensionUnrepresentable`] when half of the model's output width does not fit a head dimension, [`DecisionError::KevArchitectureUnknown`] when the model metadata names no architecture, [`DecisionError::KevTemperatureMissing`] when it lacks the temperature of a question type, [`DecisionError::KevTemperatureUnparseable`] when a temperature is not a number, [`DecisionError::KevTemperatureOutOfRange`] when it is not a finite positive number, and [`DecisionError::KevTemperaturesDiffer`] when the question types declare different temperatures.
    pub fn of(model_metadata: &ModelMetadata, n_embd_out: c_int) -> Result<Self, DecisionError> {
        let head_dim = u16::try_from(n_embd_out / 2).map_err(|source| {
            DecisionError::KevHeadDimensionUnrepresentable { n_embd_out, source }
        })?;

        Ok(Self {
            head_dim,
            tempered_logit_scale: f32::from(head_dim).sqrt().recip()
                / read_shared_temperature(model_metadata)?,
        })
    }

    #[must_use]
    pub fn option_probabilities(&self, output_rows: &[Vec<f32>]) -> Vec<f32> {
        let Some((decide_row, option_rows)) = output_rows.split_last() else {
            return Vec::new();
        };
        let head_dim = usize::from(self.head_dim);

        softmax(
            &option_rows
                .iter()
                .map(|option_row| {
                    let alignment: f32 = decide_row
                        .iter()
                        .take(head_dim)
                        .zip(option_row.iter().skip(head_dim))
                        .map(|(query_value, key_value)| query_value * key_value)
                        .sum();

                    alignment * self.tempered_logit_scale
                })
                .collect::<Vec<f32>>(),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use paddler_messaging::model_metadata::ModelMetadata;

    use super::KevOptionScorer;
    use crate::decision_error::DecisionError;

    fn kev_metadata_with(
        choice_temperature: &str,
        noul_temperature: &str,
        score_temperature: &str,
    ) -> ModelMetadata {
        ModelMetadata {
            metadata: BTreeMap::from([
                ("general.architecture".to_owned(), "qwen35".to_owned()),
                (
                    "qwen35.decision.temperature.choice".to_owned(),
                    choice_temperature.to_owned(),
                ),
                (
                    "qwen35.decision.temperature.noul".to_owned(),
                    noul_temperature.to_owned(),
                ),
                (
                    "qwen35.decision.temperature.score".to_owned(),
                    score_temperature.to_owned(),
                ),
            ]),
        }
    }

    fn kev_metadata(temperature: &str) -> ModelMetadata {
        kev_metadata_with(temperature, temperature, temperature)
    }

    #[test]
    fn scores_each_option_by_its_key_against_the_query_of_the_decide_row() {
        let aligned_option_logit = 1.0 * 2.0_f32.sqrt().recip() / 1.0;
        let orthogonal_option_exponential = (0.0 - aligned_option_logit).exp();
        let exponential_sum = 1.0 + orthogonal_option_exponential;

        assert_eq!(
            KevOptionScorer::of(&kev_metadata("1.0"), 4)
                .unwrap()
                .option_probabilities(&[
                    vec![0.0, 0.0, 1.0, 0.0],
                    vec![0.0, 0.0, 0.0, 1.0],
                    vec![1.0, 0.0, 9.0, 9.0]
                ]),
            vec![
                1.0 / exponential_sum,
                orthogonal_option_exponential / exponential_sum
            ]
        );
    }

    #[test]
    fn divides_the_option_logits_by_the_temperature() {
        let rows = [
            vec![0.0, 0.0, 1.0, 0.0],
            vec![0.0, 0.0, 0.0, 1.0],
            vec![1.0, 0.0, 9.0, 9.0],
        ];
        let cold = KevOptionScorer::of(&kev_metadata("0.5"), 4)
            .unwrap()
            .option_probabilities(&rows);
        let warm = KevOptionScorer::of(&kev_metadata("2.0"), 4)
            .unwrap()
            .option_probabilities(&rows);

        assert!(cold.first() > warm.first());
    }

    #[test]
    fn an_answer_without_output_rows_has_no_option_probabilities() {
        assert_eq!(
            KevOptionScorer::of(&kev_metadata("1.0"), 4)
                .unwrap()
                .option_probabilities(&[]),
            Vec::<f32>::new()
        );
    }

    #[test]
    fn a_model_without_an_architecture_has_no_decision_temperature() {
        assert!(matches!(
            KevOptionScorer::of(&ModelMetadata::default(), 4),
            Err(DecisionError::KevArchitectureUnknown)
        ));
    }

    #[test]
    fn a_decision_temperature_that_is_not_a_number_cannot_score_options() {
        assert!(matches!(
            KevOptionScorer::of(&kev_metadata("warm"), 4),
            Err(DecisionError::KevTemperatureUnparseable { declared, .. }) if declared == "warm"
        ));
    }

    #[test]
    fn a_model_declaring_different_temperatures_per_question_type_cannot_score_options() {
        assert!(matches!(
            KevOptionScorer::of(&kev_metadata_with("1.0", "1.0", "2.0"), 4),
            Err(DecisionError::KevTemperaturesDiffer { .. })
        ));
    }

    #[test]
    fn a_model_lacking_the_temperature_of_any_question_type_cannot_score_options() {
        for missing_temperature_key in [
            "qwen35.decision.temperature.choice",
            "qwen35.decision.temperature.noul",
            "qwen35.decision.temperature.score",
        ] {
            let mut model_metadata = kev_metadata("1.0");

            model_metadata.metadata.remove(missing_temperature_key);

            assert!(matches!(
                KevOptionScorer::of(&model_metadata, 4),
                Err(DecisionError::KevTemperatureMissing { key })
                    if key == missing_temperature_key
            ));
        }
    }

    #[test]
    fn a_decision_temperature_that_is_not_positive_cannot_score_options() {
        assert!(matches!(
            KevOptionScorer::of(&kev_metadata("0.0"), 4),
            Err(DecisionError::KevTemperatureOutOfRange { .. })
        ));
    }

    #[test]
    fn an_output_width_beyond_a_head_dimension_cannot_score_options() {
        assert!(matches!(
            KevOptionScorer::of(&kev_metadata("1.0"), -2),
            Err(DecisionError::KevHeadDimensionUnrepresentable { n_embd_out: -2, .. })
        ));
    }
}
