use anyhow::Result;
use anyhow::bail;
use serde::Deserialize;
use serde::Serialize;

use crate::kv_cache_dtype::KvCacheDtype;
use crate::pooling_type::PoolingType;
use crate::validates::Validates;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InferenceParameters {
    pub n_batch: usize,
    pub context_size: u32,
    pub embedding_batch_size: usize,
    pub enable_embeddings: bool,
    pub image_resize_to_fit: u32,
    pub k_cache_dtype: KvCacheDtype,
    pub v_cache_dtype: KvCacheDtype,
    /// The minimum probability for a token to be considered, relative to the probability of the most likely token
    pub min_p: f32,
    /// Number of model layers to offload to GPU. 0 = CPU-only.
    /// Set to a value >= the model's transformer block count for full GPU offload.
    pub n_gpu_layers: u32,
    pub penalty_frequency: f32,
    /// How many tokens to scan for repetitions (-1 = context size, 0 = disabled)
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

impl Validates<Self> for InferenceParameters {
    fn validate(self) -> Result<Self> {
        if self.image_resize_to_fit == 0 {
            bail!("image_resize_to_fit must be greater than zero");
        }

        if self.embedding_batch_size == 0 {
            bail!("embedding_batch_size must be greater than zero");
        }

        Ok(self)
    }
}

impl Default for InferenceParameters {
    fn default() -> Self {
        Self {
            n_batch: 2048,
            context_size: 8192,
            embedding_batch_size: 256,
            enable_embeddings: false,
            image_resize_to_fit: 1024,
            k_cache_dtype: KvCacheDtype::Q80,
            v_cache_dtype: KvCacheDtype::Q80,
            min_p: 0.05,
            n_gpu_layers: 0,
            penalty_frequency: 0.0,
            penalty_last_n: -1,
            penalty_presence: 0.8,
            penalty_repeat: 1.1,
            pooling_type: PoolingType::Last,
            temperature: 0.8,
            top_k: 80,
            top_p: 0.8,
        }
    }
}

impl InferenceParameters {
    #[must_use]
    pub fn deterministic() -> Self {
        Self {
            min_p: 0.0,
            penalty_frequency: 0.0,
            penalty_presence: 0.0,
            penalty_repeat: 1.0,
            temperature: 0.0,
            top_k: 1,
            top_p: 1.0,
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_succeeds_with_default_params() {
        let params = InferenceParameters::default();

        assert!(params.validate().is_ok());
    }

    #[test]
    fn validate_fails_when_image_resize_to_fit_is_zero() {
        let params = InferenceParameters {
            image_resize_to_fit: 0,
            ..InferenceParameters::default()
        };

        assert!(params.validate().is_err());
    }

    #[test]
    fn validate_fails_when_embedding_batch_size_is_zero() {
        let params = InferenceParameters {
            embedding_batch_size: 0,
            ..InferenceParameters::default()
        };

        assert!(params.validate().is_err());
    }

    #[test]
    fn default_embedding_batch_size_is_256() {
        let params = InferenceParameters::default();

        assert_eq!(params.embedding_batch_size, 256);
    }

    #[test]
    fn deterministic_applies_greedy_sampling_over_defaults() {
        assert_eq!(
            InferenceParameters::deterministic(),
            InferenceParameters {
                min_p: 0.0,
                penalty_frequency: 0.0,
                penalty_presence: 0.0,
                penalty_repeat: 1.0,
                temperature: 0.0,
                top_k: 1,
                top_p: 1.0,
                ..InferenceParameters::default()
            }
        );
    }
}
