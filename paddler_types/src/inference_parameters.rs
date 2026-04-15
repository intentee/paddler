use anyhow::Result;
use anyhow::bail;
use serde::Deserialize;
use serde::Serialize;

use crate::kv_cache_type::KvCacheType;
use crate::pooling_type::PoolingType;
use crate::validates::Validates;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InferenceParameters {
    pub batch_n_tokens: usize,
    pub context_size: u32,
    pub enable_embeddings: bool,
    pub image_resize_to_fit: u32,
    pub kv_cache_type: KvCacheType,
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

        Ok(self)
    }
}

impl Default for InferenceParameters {
    fn default() -> Self {
        Self {
            batch_n_tokens: 512,
            context_size: 8192,
            enable_embeddings: false,
            image_resize_to_fit: 1024,
            kv_cache_type: KvCacheType::Q8_0,
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
}
