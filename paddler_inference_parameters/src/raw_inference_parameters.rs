use std::num::NonZeroU32;
use std::num::NonZeroUsize;

use serde::Deserialize;

use crate::batch_size::BatchSize;
use crate::kv_cache_dtype::KvCacheDtype;
use crate::pooling_type::PoolingType;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawInferenceParameters {
    pub n_batch: BatchSize,
    pub context_size: NonZeroU32,
    pub embedding_batch_size: NonZeroUsize,
    pub enable_embeddings: bool,
    pub image_resize_to_fit: NonZeroU32,
    pub k_cache_dtype: KvCacheDtype,
    pub v_cache_dtype: KvCacheDtype,
    pub min_p: f32,
    pub n_gpu_layers: i32,
    pub penalty_frequency: f32,
    pub penalty_last_n: i32,
    pub penalty_presence: f32,
    pub penalty_repeat: f32,
    pub pooling_type: PoolingType,
    pub temperature: f32,
    pub top_k: i32,
    pub top_p: f32,
}
