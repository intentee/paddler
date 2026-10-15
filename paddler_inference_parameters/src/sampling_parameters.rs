use std::cmp::Ordering;

use serde::Deserialize;
use serde::Serialize;

use crate::invalid_inference_parameters::InvalidInferenceParameters;
use crate::raw_sampling_parameters::RawSamplingParameters;

const NEUTRAL_PENALTY_REPEAT: f32 = 1.0;

fn is_probability(value: f32) -> bool {
    (0.0..=1.0).contains(&value)
}

fn penalties_are_neutral(
    penalty_repeat: f32,
    penalty_frequency: f32,
    penalty_presence: f32,
) -> bool {
    penalty_repeat.total_cmp(&NEUTRAL_PENALTY_REPEAT) == Ordering::Equal
        && penalty_frequency == 0.0
        && penalty_presence == 0.0
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(try_from = "RawSamplingParameters")]
pub struct SamplingParameters {
    /// The minimum probability for a token to be considered, relative to the probability of the most likely token
    pub min_p: f32,
    pub penalty_frequency: f32,
    /// How many tokens to scan for repetitions (0 = disabled)
    pub penalty_last_n: i32,
    pub penalty_presence: f32,
    /// Penalty for repeating tokens (1.0 = disabled)
    pub penalty_repeat: f32,
    /// Adjust the randomness of the generated text (0.0 = greedy/deterministic)
    pub temperature: f32,
    /// Limit the next token selection to the K most probable tokens
    pub top_k: i32,
    /// Limit the next token selection to a subset of tokens with a cumulative probability above a threshold P
    pub top_p: f32,
}

impl SamplingParameters {
    #[must_use]
    pub const fn deterministic() -> Self {
        Self {
            min_p: 0.0,
            penalty_frequency: 0.0,
            penalty_last_n: 0,
            penalty_presence: 0.0,
            penalty_repeat: NEUTRAL_PENALTY_REPEAT,
            temperature: 0.0,
            top_k: 1,
            top_p: 1.0,
        }
    }
}

impl Default for SamplingParameters {
    fn default() -> Self {
        Self {
            min_p: 0.05,
            penalty_frequency: 0.0,
            penalty_last_n: 0,
            penalty_presence: 0.0,
            penalty_repeat: NEUTRAL_PENALTY_REPEAT,
            temperature: 0.8,
            top_k: 80,
            top_p: 0.8,
        }
    }
}

impl TryFrom<RawSamplingParameters> for SamplingParameters {
    type Error = InvalidInferenceParameters;

    fn try_from(
        RawSamplingParameters {
            min_p,
            penalty_frequency,
            penalty_last_n,
            penalty_presence,
            penalty_repeat,
            temperature,
            top_k,
            top_p,
        }: RawSamplingParameters,
    ) -> Result<Self, Self::Error> {
        if !is_probability(min_p) {
            return Err(InvalidInferenceParameters::MinPOutOfRange { min_p });
        }

        if !is_probability(top_p) {
            return Err(InvalidInferenceParameters::TopPOutOfRange { top_p });
        }

        if !(temperature.is_finite() && temperature >= 0.0) {
            return Err(InvalidInferenceParameters::TemperatureNegative { temperature });
        }

        if top_k < 0 {
            return Err(InvalidInferenceParameters::TopKNegative { top_k });
        }

        if penalty_last_n < 0 {
            return Err(InvalidInferenceParameters::PenaltyLastNNegative { penalty_last_n });
        }

        if !(penalty_repeat.is_finite() && penalty_repeat > 0.0) {
            return Err(InvalidInferenceParameters::PenaltyRepeatNotPositive { penalty_repeat });
        }

        if !penalty_frequency.is_finite() {
            return Err(InvalidInferenceParameters::PenaltyFrequencyNotFinite {
                penalty_frequency,
            });
        }

        if !penalty_presence.is_finite() {
            return Err(InvalidInferenceParameters::PenaltyPresenceNotFinite { penalty_presence });
        }

        let penalties_are_neutral =
            penalties_are_neutral(penalty_repeat, penalty_frequency, penalty_presence);

        if penalty_last_n == 0 && !penalties_are_neutral {
            return Err(InvalidInferenceParameters::PenaltiesWithoutWindow {
                penalty_frequency,
                penalty_presence,
                penalty_repeat,
            });
        }

        if penalty_last_n > 0 && penalties_are_neutral {
            return Err(InvalidInferenceParameters::PenaltyWindowWithoutPenalties {
                penalty_last_n,
            });
        }

        Ok(Self {
            min_p,
            penalty_frequency,
            penalty_last_n,
            penalty_presence,
            penalty_repeat,
            temperature,
            top_k,
            top_p,
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::from_value;
    use serde_json::json;
    use serde_json::to_value;

    use super::SamplingParameters;
    use crate::invalid_inference_parameters::InvalidInferenceParameters;
    use crate::raw_sampling_parameters::RawSamplingParameters;

    fn valid_raw() -> RawSamplingParameters {
        RawSamplingParameters {
            min_p: 0.05,
            penalty_frequency: 0.5,
            penalty_last_n: 64,
            penalty_presence: 0.0,
            penalty_repeat: 1.1,
            temperature: 0.7,
            top_k: 40,
            top_p: 0.9,
        }
    }

    fn rejection(raw: RawSamplingParameters) -> Option<InvalidInferenceParameters> {
        SamplingParameters::try_from(raw).err()
    }

    #[test]
    fn accepts_parameters_that_satisfy_every_rule() {
        assert!(SamplingParameters::try_from(valid_raw()).is_ok());
    }

    #[test]
    fn deserialization_rejects_parameters_that_fail_validation() {
        let mut parameters = to_value(SamplingParameters::default()).unwrap();

        parameters["top_k"] = json!(-1);

        assert_eq!(
            from_value::<SamplingParameters>(parameters)
                .unwrap_err()
                .to_string(),
            InvalidInferenceParameters::TopKNegative { top_k: -1 }.to_string()
        );
    }

    #[test]
    fn rejects_min_p_outside_zero_to_one() {
        assert_eq!(
            rejection(RawSamplingParameters {
                min_p: 1.5,
                ..valid_raw()
            }),
            Some(InvalidInferenceParameters::MinPOutOfRange { min_p: 1.5 })
        );
    }

    #[test]
    fn rejects_top_p_outside_zero_to_one() {
        assert_eq!(
            rejection(RawSamplingParameters {
                top_p: -0.1,
                ..valid_raw()
            }),
            Some(InvalidInferenceParameters::TopPOutOfRange { top_p: -0.1 })
        );
    }

    #[test]
    fn rejects_a_negative_temperature() {
        assert_eq!(
            rejection(RawSamplingParameters {
                temperature: -0.5,
                ..valid_raw()
            }),
            Some(InvalidInferenceParameters::TemperatureNegative { temperature: -0.5 })
        );
    }

    #[test]
    fn rejects_a_negative_top_k() {
        assert_eq!(
            rejection(RawSamplingParameters {
                top_k: -1,
                ..valid_raw()
            }),
            Some(InvalidInferenceParameters::TopKNegative { top_k: -1 })
        );
    }

    #[test]
    fn rejects_a_negative_penalty_window() {
        assert_eq!(
            rejection(RawSamplingParameters {
                penalty_last_n: -1,
                ..valid_raw()
            }),
            Some(InvalidInferenceParameters::PenaltyLastNNegative { penalty_last_n: -1 })
        );
    }

    #[test]
    fn rejects_a_repeat_penalty_that_is_not_positive() {
        assert_eq!(
            rejection(RawSamplingParameters {
                penalty_repeat: 0.0,
                ..valid_raw()
            }),
            Some(InvalidInferenceParameters::PenaltyRepeatNotPositive {
                penalty_repeat: 0.0
            })
        );
    }

    #[test]
    fn rejects_an_infinite_frequency_penalty() {
        assert_eq!(
            rejection(RawSamplingParameters {
                penalty_frequency: f32::INFINITY,
                ..valid_raw()
            }),
            Some(InvalidInferenceParameters::PenaltyFrequencyNotFinite {
                penalty_frequency: f32::INFINITY,
            })
        );
    }

    #[test]
    fn rejects_an_infinite_presence_penalty() {
        assert_eq!(
            rejection(RawSamplingParameters {
                penalty_presence: f32::NEG_INFINITY,
                ..valid_raw()
            }),
            Some(InvalidInferenceParameters::PenaltyPresenceNotFinite {
                penalty_presence: f32::NEG_INFINITY,
            })
        );
    }

    #[test]
    fn rejects_penalty_strengths_without_a_penalty_window() {
        assert_eq!(
            rejection(RawSamplingParameters {
                penalty_last_n: 0,
                ..valid_raw()
            }),
            Some(InvalidInferenceParameters::PenaltiesWithoutWindow {
                penalty_frequency: 0.5,
                penalty_presence: 0.0,
                penalty_repeat: 1.1,
            })
        );
    }

    #[test]
    fn rejects_a_penalty_window_without_penalty_strengths() {
        assert_eq!(
            rejection(RawSamplingParameters {
                penalty_frequency: 0.0,
                penalty_repeat: 1.0,
                ..valid_raw()
            }),
            Some(InvalidInferenceParameters::PenaltyWindowWithoutPenalties { penalty_last_n: 64 })
        );
    }
}
