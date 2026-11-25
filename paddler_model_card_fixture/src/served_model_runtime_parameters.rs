use paddler_inference_parameters::batch_size::BatchSize;
use paddler_inference_parameters::constants::ALL_GPU_LAYERS;
use paddler_inference_parameters::desired_model_runtime_parameters::DesiredModelRuntimeParameters;
use paddler_inference_parameters::kv_cache_dtype::KvCacheDtype;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;

use crate::constants::SERVED_CONTEXT_SIZE;

#[must_use]
pub fn served_model_runtime_parameters() -> DesiredModelRuntimeParameters {
    DesiredModelRuntimeParameters {
        context_size: ModelDefaultOr::Explicit(SERVED_CONTEXT_SIZE),
        k_cache_dtype: ModelDefaultOr::Explicit(KvCacheDtype::Q80),
        n_batch: ModelDefaultOr::Explicit(BatchSize::default()),
        n_gpu_layers: ModelDefaultOr::Explicit(ALL_GPU_LAYERS),
        v_cache_dtype: ModelDefaultOr::Explicit(KvCacheDtype::Q80),
    }
}
