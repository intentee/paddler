use std::num::NonZeroU32;

use serde::Deserialize;
use serde::Serialize;

use crate::all_gpu_layers::ALL_GPU_LAYERS;
use crate::batch_size::BatchSize;
use crate::invalid_inference_parameters::InvalidInferenceParameters;
use crate::kv_cache_dtype::KvCacheDtype;
use crate::raw_model_runtime_parameters::RawModelRuntimeParameters;

const DEFAULT_CONTEXT_SIZE: NonZeroU32 = NonZeroU32::new(8192).unwrap();

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "RawModelRuntimeParameters")]
pub struct ModelRuntimeParameters {
    pub context_size: NonZeroU32,
    pub k_cache_dtype: KvCacheDtype,
    pub n_batch: BatchSize,
    /// Number of model layers to offload to GPU. 0 = CPU-only, -1 = offload all layers.
    /// Set to a value >= the model's transformer block count for full GPU offload.
    pub n_gpu_layers: i32,
    pub v_cache_dtype: KvCacheDtype,
}

impl Default for ModelRuntimeParameters {
    fn default() -> Self {
        Self {
            context_size: DEFAULT_CONTEXT_SIZE,
            k_cache_dtype: KvCacheDtype::Q80,
            n_batch: BatchSize::DEFAULT,
            n_gpu_layers: 0,
            v_cache_dtype: KvCacheDtype::Q80,
        }
    }
}

impl TryFrom<RawModelRuntimeParameters> for ModelRuntimeParameters {
    type Error = InvalidInferenceParameters;

    fn try_from(
        RawModelRuntimeParameters {
            context_size,
            k_cache_dtype,
            n_batch,
            n_gpu_layers,
            v_cache_dtype,
        }: RawModelRuntimeParameters,
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

        Ok(Self {
            context_size,
            k_cache_dtype,
            n_batch,
            n_gpu_layers,
            v_cache_dtype,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use serde_json::from_value;
    use serde_json::json;
    use serde_json::to_value;

    use super::ModelRuntimeParameters;
    use crate::batch_size::BatchSize;
    use crate::invalid_inference_parameters::InvalidInferenceParameters;
    use crate::kv_cache_dtype::KvCacheDtype;
    use crate::raw_model_runtime_parameters::RawModelRuntimeParameters;

    fn valid_raw() -> RawModelRuntimeParameters {
        RawModelRuntimeParameters {
            context_size: NonZeroU32::new(4096).unwrap(),
            k_cache_dtype: KvCacheDtype::F16,
            n_batch: BatchSize::try_from(512).unwrap(),
            n_gpu_layers: -1,
            v_cache_dtype: KvCacheDtype::F16,
        }
    }

    #[test]
    fn accepts_parameters_that_satisfy_every_rule() {
        assert!(ModelRuntimeParameters::try_from(valid_raw()).is_ok());
    }

    #[test]
    fn deserialization_rejects_parameters_that_fail_validation() {
        let mut parameters = to_value(ModelRuntimeParameters::default()).unwrap();

        parameters["n_gpu_layers"] = json!(-2);

        assert_eq!(
            from_value::<ModelRuntimeParameters>(parameters)
                .unwrap_err()
                .to_string(),
            InvalidInferenceParameters::GpuLayersBelowAllLayers { n_gpu_layers: -2 }.to_string()
        );
    }

    #[test]
    fn rejects_a_batch_larger_than_the_context() {
        assert_eq!(
            ModelRuntimeParameters::try_from(RawModelRuntimeParameters {
                n_batch: BatchSize::try_from(8192).unwrap(),
                ..valid_raw()
            })
            .err(),
            Some(InvalidInferenceParameters::BatchSizeExceedsContextSize {
                n_batch: 8192,
                context_size: 4096,
            })
        );
    }

    #[test]
    fn rejects_gpu_layers_below_the_all_layers_value() {
        assert_eq!(
            ModelRuntimeParameters::try_from(RawModelRuntimeParameters {
                n_gpu_layers: -2,
                ..valid_raw()
            })
            .err(),
            Some(InvalidInferenceParameters::GpuLayersBelowAllLayers { n_gpu_layers: -2 })
        );
    }
}
