use std::cmp::Ordering;
use std::num::NonZeroU32;
use std::num::NonZeroUsize;

use serde::Deserialize;
use serde::Serialize;

use crate::batch_size::BatchSize;
use crate::invalid_inference_parameters::InvalidInferenceParameters;
use crate::kv_cache_dtype::KvCacheDtype;
use crate::pooling_type::PoolingType;
use crate::raw_inference_parameters::RawInferenceParameters;

const ALL_GPU_LAYERS: i32 = -1;
const DEFAULT_BATCH_SIZE: BatchSize = BatchSize::DEFAULT;
const DEFAULT_CONTEXT_SIZE: NonZeroU32 = NonZeroU32::new(8192).unwrap();
const DEFAULT_EMBEDDING_BATCH_SIZE: NonZeroUsize = NonZeroUsize::new(256).unwrap();
const DEFAULT_IMAGE_RESIZE_TO_FIT: NonZeroU32 = NonZeroU32::new(1024).unwrap();
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
#[serde(try_from = "RawInferenceParameters")]
pub struct InferenceParameters {
    pub n_batch: BatchSize,
    pub context_size: NonZeroU32,
    pub embedding_batch_size: NonZeroUsize,
    pub enable_embeddings: bool,
    pub image_resize_to_fit: NonZeroU32,
    pub k_cache_dtype: KvCacheDtype,
    pub v_cache_dtype: KvCacheDtype,
    /// The minimum probability for a token to be considered, relative to the probability of the most likely token
    pub min_p: f32,
    /// Number of model layers to offload to GPU. 0 = CPU-only, -1 = offload all layers.
    /// Set to a value >= the model's transformer block count for full GPU offload.
    pub n_gpu_layers: i32,
    pub penalty_frequency: f32,
    /// How many tokens to scan for repetitions (0 = disabled)
    pub penalty_last_n: i32,
    pub penalty_presence: f32,
    /// Penalty for repeating tokens (1.0 = disabled)
    pub penalty_repeat: f32,
    pub pooling_type: PoolingType,
    /// Adjust the randomness of the generated text (0.0 = greedy/deterministic)
    pub temperature: f32,
    /// Limit the next token selection to the K most probable tokens
    pub top_k: i32,
    /// Limit the next token selection to a subset of tokens with a cumulative probability above a threshold P
    pub top_p: f32,
}

impl InferenceParameters {
    #[must_use]
    pub fn deterministic() -> Self {
        Self {
            min_p: 0.0,
            penalty_frequency: 0.0,
            penalty_last_n: 0,
            penalty_presence: 0.0,
            penalty_repeat: NEUTRAL_PENALTY_REPEAT,
            temperature: 0.0,
            top_k: 1,
            top_p: 1.0,
            ..Self::default()
        }
    }
}

impl Default for InferenceParameters {
    fn default() -> Self {
        Self {
            n_batch: DEFAULT_BATCH_SIZE,
            context_size: DEFAULT_CONTEXT_SIZE,
            embedding_batch_size: DEFAULT_EMBEDDING_BATCH_SIZE,
            enable_embeddings: false,
            image_resize_to_fit: DEFAULT_IMAGE_RESIZE_TO_FIT,
            k_cache_dtype: KvCacheDtype::Q80,
            v_cache_dtype: KvCacheDtype::Q80,
            min_p: 0.05,
            n_gpu_layers: 0,
            penalty_frequency: 0.0,
            penalty_last_n: 0,
            penalty_presence: 0.0,
            penalty_repeat: NEUTRAL_PENALTY_REPEAT,
            pooling_type: PoolingType::Last,
            temperature: 0.8,
            top_k: 80,
            top_p: 0.8,
        }
    }
}

impl TryFrom<RawInferenceParameters> for InferenceParameters {
    type Error = InvalidInferenceParameters;

    fn try_from(
        RawInferenceParameters {
            n_batch,
            context_size,
            embedding_batch_size,
            enable_embeddings,
            image_resize_to_fit,
            k_cache_dtype,
            v_cache_dtype,
            min_p,
            n_gpu_layers,
            penalty_frequency,
            penalty_last_n,
            penalty_presence,
            penalty_repeat,
            pooling_type,
            temperature,
            top_k,
            top_p,
        }: RawInferenceParameters,
    ) -> Result<Self, Self::Error> {
        if n_batch.tokens() > context_size {
            return Err(InvalidInferenceParameters::BatchSizeExceedsContextSize {
                n_batch: n_batch.tokens().get(),
                context_size: context_size.get(),
            });
        }

        if n_gpu_layers < ALL_GPU_LAYERS {
            return Err(InvalidInferenceParameters::GpuLayersBelowAllLayers { n_gpu_layers });
        }

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
            n_batch,
            context_size,
            embedding_batch_size,
            enable_embeddings,
            image_resize_to_fit,
            k_cache_dtype,
            v_cache_dtype,
            min_p,
            n_gpu_layers,
            penalty_frequency,
            penalty_last_n,
            penalty_presence,
            penalty_repeat,
            pooling_type,
            temperature,
            top_k,
            top_p,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;
    use std::num::NonZeroUsize;

    use serde_json::json;

    use super::InferenceParameters;
    use crate::batch_size::BatchSize;
    use crate::invalid_inference_parameters::InvalidInferenceParameters;
    use crate::kv_cache_dtype::KvCacheDtype;
    use crate::pooling_type::PoolingType;
    use crate::raw_inference_parameters::RawInferenceParameters;

    fn valid_raw() -> RawInferenceParameters {
        RawInferenceParameters {
            n_batch: BatchSize::try_from(512).unwrap(),
            context_size: NonZeroU32::new(4096).unwrap(),
            embedding_batch_size: NonZeroUsize::new(64).unwrap(),
            enable_embeddings: false,
            image_resize_to_fit: NonZeroU32::new(512).unwrap(),
            k_cache_dtype: KvCacheDtype::F16,
            v_cache_dtype: KvCacheDtype::F16,
            min_p: 0.05,
            n_gpu_layers: -1,
            penalty_frequency: 0.5,
            penalty_last_n: 64,
            penalty_presence: 0.0,
            penalty_repeat: 1.1,
            pooling_type: PoolingType::Mean,
            temperature: 0.7,
            top_k: 40,
            top_p: 0.9,
        }
    }

    fn rejection(raw: RawInferenceParameters) -> Option<InvalidInferenceParameters> {
        InferenceParameters::try_from(raw).err()
    }

    #[test]
    fn accepts_parameters_that_satisfy_every_rule() {
        assert!(InferenceParameters::try_from(valid_raw()).is_ok());
    }

    #[test]
    fn deserialization_rejects_parameters_that_fail_validation() {
        let mut parameters = serde_json::to_value(InferenceParameters::default()).unwrap();

        parameters["top_k"] = json!(-1);

        assert_eq!(
            serde_json::from_value::<InferenceParameters>(parameters)
                .unwrap_err()
                .to_string(),
            InvalidInferenceParameters::TopKNegative { top_k: -1 }.to_string()
        );
    }

    #[test]
    fn rejects_a_batch_larger_than_the_context() {
        assert_eq!(
            rejection(RawInferenceParameters {
                n_batch: BatchSize::try_from(8192).unwrap(),
                ..valid_raw()
            }),
            Some(InvalidInferenceParameters::BatchSizeExceedsContextSize {
                n_batch: 8192,
                context_size: 4096,
            })
        );
    }

    #[test]
    fn rejects_gpu_layers_below_the_all_layers_value() {
        assert_eq!(
            rejection(RawInferenceParameters {
                n_gpu_layers: -2,
                ..valid_raw()
            }),
            Some(InvalidInferenceParameters::GpuLayersBelowAllLayers { n_gpu_layers: -2 })
        );
    }

    #[test]
    fn rejects_min_p_outside_zero_to_one() {
        assert_eq!(
            rejection(RawInferenceParameters {
                min_p: 1.5,
                ..valid_raw()
            }),
            Some(InvalidInferenceParameters::MinPOutOfRange { min_p: 1.5 })
        );
    }

    #[test]
    fn rejects_top_p_outside_zero_to_one() {
        assert_eq!(
            rejection(RawInferenceParameters {
                top_p: -0.1,
                ..valid_raw()
            }),
            Some(InvalidInferenceParameters::TopPOutOfRange { top_p: -0.1 })
        );
    }

    #[test]
    fn rejects_a_negative_temperature() {
        assert_eq!(
            rejection(RawInferenceParameters {
                temperature: -0.5,
                ..valid_raw()
            }),
            Some(InvalidInferenceParameters::TemperatureNegative { temperature: -0.5 })
        );
    }

    #[test]
    fn rejects_a_negative_top_k() {
        assert_eq!(
            rejection(RawInferenceParameters {
                top_k: -1,
                ..valid_raw()
            }),
            Some(InvalidInferenceParameters::TopKNegative { top_k: -1 })
        );
    }

    #[test]
    fn rejects_a_negative_penalty_window() {
        assert_eq!(
            rejection(RawInferenceParameters {
                penalty_last_n: -1,
                ..valid_raw()
            }),
            Some(InvalidInferenceParameters::PenaltyLastNNegative { penalty_last_n: -1 })
        );
    }

    #[test]
    fn rejects_a_repeat_penalty_that_is_not_positive() {
        assert_eq!(
            rejection(RawInferenceParameters {
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
            rejection(RawInferenceParameters {
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
            rejection(RawInferenceParameters {
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
            rejection(RawInferenceParameters {
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
            rejection(RawInferenceParameters {
                penalty_frequency: 0.0,
                penalty_repeat: 1.0,
                ..valid_raw()
            }),
            Some(InvalidInferenceParameters::PenaltyWindowWithoutPenalties { penalty_last_n: 64 })
        );
    }
}
