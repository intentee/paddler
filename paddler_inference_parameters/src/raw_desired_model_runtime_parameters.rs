use std::num::NonZeroU32;

use serde::Deserialize;

use crate::batch_size::BatchSize;
use crate::kv_cache_dtype::KvCacheDtype;
use crate::model_default_or::ModelDefaultOr;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawDesiredModelRuntimeParameters {
    #[serde(default)]
    pub context_size: ModelDefaultOr<NonZeroU32>,
    #[serde(default)]
    pub k_cache_dtype: ModelDefaultOr<KvCacheDtype>,
    #[serde(default)]
    pub n_batch: ModelDefaultOr<BatchSize>,
    #[serde(default)]
    pub n_gpu_layers: ModelDefaultOr<i32>,
    #[serde(default)]
    pub v_cache_dtype: ModelDefaultOr<KvCacheDtype>,
}
