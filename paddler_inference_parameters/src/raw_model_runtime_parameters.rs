use std::num::NonZeroU32;

use serde::Deserialize;

use crate::batch_size::BatchSize;
use crate::kv_cache_dtype::KvCacheDtype;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawModelRuntimeParameters {
    pub context_size: NonZeroU32,
    pub k_cache_dtype: KvCacheDtype,
    pub n_batch: BatchSize,
    pub n_gpu_layers: i32,
    pub v_cache_dtype: KvCacheDtype,
}
