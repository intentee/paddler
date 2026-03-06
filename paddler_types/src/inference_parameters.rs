use serde::Deserialize;
use serde::Serialize;

use crate::pooling_type::PoolingType;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InferenceParameters {
    pub batch_n_tokens: usize,
    pub context_size: u32,
    pub embedding_n_seq_max: u32,
    pub enable_embeddings: bool,
    pub image_resize_to_fit: u32,
    /// The minimum probability for a token to be considered, relative to the probability of the most likely token
    pub min_p: f32,
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

impl Default for InferenceParameters {
    fn default() -> Self {
        Self {
            batch_n_tokens: 512,
            context_size: 8192,
            embedding_n_seq_max: 16,
            enable_embeddings: false,
            image_resize_to_fit: 1024,
            min_p: 0.05,
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
