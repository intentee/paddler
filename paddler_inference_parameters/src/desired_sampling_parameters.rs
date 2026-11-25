use serde::Deserialize;
use serde::Serialize;

use crate::invalid_inference_parameters::InvalidInferenceParameters;
use crate::model_default_or::ModelDefaultOr;
use crate::raw_desired_sampling_parameters::RawDesiredSamplingParameters;
use crate::sampling_parameters::SamplingParameters;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(try_from = "RawDesiredSamplingParameters")]
pub struct DesiredSamplingParameters {
    pub min_p: ModelDefaultOr<f32>,
    pub penalty_frequency: ModelDefaultOr<f32>,
    pub penalty_last_n: ModelDefaultOr<i32>,
    pub penalty_presence: ModelDefaultOr<f32>,
    pub penalty_repeat: ModelDefaultOr<f32>,
    pub temperature: ModelDefaultOr<f32>,
    pub top_k: ModelDefaultOr<i32>,
    pub top_p: ModelDefaultOr<f32>,
}

impl DesiredSamplingParameters {
    #[must_use]
    pub fn resolve(
        self,
        SamplingParameters {
            min_p,
            penalty_frequency,
            penalty_last_n,
            penalty_presence,
            penalty_repeat,
            temperature,
            top_k,
            top_p,
        }: &SamplingParameters,
    ) -> SamplingParameters {
        SamplingParameters {
            min_p: self.min_p.resolve(*min_p),
            penalty_frequency: self.penalty_frequency.resolve(*penalty_frequency),
            penalty_last_n: self.penalty_last_n.resolve(*penalty_last_n),
            penalty_presence: self.penalty_presence.resolve(*penalty_presence),
            penalty_repeat: self.penalty_repeat.resolve(*penalty_repeat),
            temperature: self.temperature.resolve(*temperature),
            top_k: self.top_k.resolve(*top_k),
            top_p: self.top_p.resolve(*top_p),
        }
    }
}

impl From<SamplingParameters> for DesiredSamplingParameters {
    fn from(
        SamplingParameters {
            min_p,
            penalty_frequency,
            penalty_last_n,
            penalty_presence,
            penalty_repeat,
            temperature,
            top_k,
            top_p,
        }: SamplingParameters,
    ) -> Self {
        Self {
            min_p: ModelDefaultOr::Explicit(min_p),
            penalty_frequency: ModelDefaultOr::Explicit(penalty_frequency),
            penalty_last_n: ModelDefaultOr::Explicit(penalty_last_n),
            penalty_presence: ModelDefaultOr::Explicit(penalty_presence),
            penalty_repeat: ModelDefaultOr::Explicit(penalty_repeat),
            temperature: ModelDefaultOr::Explicit(temperature),
            top_k: ModelDefaultOr::Explicit(top_k),
            top_p: ModelDefaultOr::Explicit(top_p),
        }
    }
}

impl TryFrom<RawDesiredSamplingParameters> for DesiredSamplingParameters {
    type Error = InvalidInferenceParameters;

    fn try_from(
        RawDesiredSamplingParameters {
            min_p,
            penalty_frequency,
            penalty_last_n,
            penalty_presence,
            penalty_repeat,
            temperature,
            top_k,
            top_p,
        }: RawDesiredSamplingParameters,
    ) -> Result<Self, Self::Error> {
        let desired_sampling_parameters = Self {
            min_p,
            penalty_frequency,
            penalty_last_n,
            penalty_presence,
            penalty_repeat,
            temperature,
            top_k,
            top_p,
        };

        desired_sampling_parameters
            .resolve(&SamplingParameters::deterministic())
            .validate()?;

        Ok(desired_sampling_parameters)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::from_value;
    use serde_json::json;

    use super::DesiredSamplingParameters;
    use crate::model_default_or::ModelDefaultOr;
    use crate::sampling_parameters::SamplingParameters;

    #[test]
    fn explicit_values_override_the_model_defaults_field_by_field() {
        let model_defaults = SamplingParameters {
            min_p: 0.05,
            penalty_frequency: 0.0,
            penalty_last_n: 64,
            penalty_presence: 0.0,
            penalty_repeat: 1.0,
            temperature: 0.6,
            top_k: 20,
            top_p: 0.95,
        };

        assert_eq!(
            DesiredSamplingParameters {
                temperature: ModelDefaultOr::Explicit(0.2),
                top_k: ModelDefaultOr::Explicit(5),
                ..DesiredSamplingParameters::default()
            }
            .resolve(&model_defaults),
            SamplingParameters {
                temperature: 0.2,
                top_k: 5,
                ..model_defaults
            }
        );
    }

    #[test]
    fn fully_explicit_parameters_ignore_the_model_defaults() {
        let deterministic = SamplingParameters::deterministic();
        let model_defaults = SamplingParameters {
            temperature: 0.9,
            ..deterministic
        };

        assert_eq!(
            DesiredSamplingParameters::from(deterministic).resolve(&model_defaults),
            deterministic
        );
    }

    #[test]
    fn an_explicit_value_llama_cpp_cannot_sample_with_is_refused_on_arrival() {
        assert!(from_value::<DesiredSamplingParameters>(json!({ "top_p": 1.5 })).is_err());
    }

    #[test]
    fn omitted_fields_follow_the_model() {
        assert_eq!(
            from_value::<DesiredSamplingParameters>(json!({ "temperature": 0.3 })).unwrap(),
            DesiredSamplingParameters {
                temperature: ModelDefaultOr::Explicit(0.3),
                ..DesiredSamplingParameters::default()
            }
        );
    }
}
