use std::num::NonZeroU32;

use serde::Deserialize;
use serde::Serialize;

use crate::batch_size::BatchSize;
use crate::constants::ALL_GPU_LAYERS;
use crate::invalid_inference_parameters::InvalidInferenceParameters;
use crate::kv_cache_dtype::KvCacheDtype;
use crate::model_default_or::ModelDefaultOr;
use crate::raw_desired_model_runtime_parameters::RawDesiredModelRuntimeParameters;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "RawDesiredModelRuntimeParameters")]
pub struct DesiredModelRuntimeParameters {
    pub context_size: ModelDefaultOr<NonZeroU32>,
    pub k_cache_dtype: ModelDefaultOr<KvCacheDtype>,
    pub n_batch: ModelDefaultOr<BatchSize>,
    /// Number of model layers to offload to GPU. 0 = CPU-only, -1 = offload all layers.
    pub n_gpu_layers: ModelDefaultOr<i32>,
    pub v_cache_dtype: ModelDefaultOr<KvCacheDtype>,
}

impl TryFrom<RawDesiredModelRuntimeParameters> for DesiredModelRuntimeParameters {
    type Error = InvalidInferenceParameters;

    fn try_from(
        RawDesiredModelRuntimeParameters {
            context_size,
            k_cache_dtype,
            n_batch,
            n_gpu_layers,
            v_cache_dtype,
        }: RawDesiredModelRuntimeParameters,
    ) -> Result<Self, Self::Error> {
        if let ModelDefaultOr::Explicit(explicit_gpu_layers) = n_gpu_layers
            && explicit_gpu_layers < ALL_GPU_LAYERS
        {
            return Err(InvalidInferenceParameters::GpuLayersBelowAllLayers {
                n_gpu_layers: explicit_gpu_layers,
            });
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
    use serde_json::from_value;
    use serde_json::json;

    use super::DesiredModelRuntimeParameters;
    use crate::model_default_or::ModelDefaultOr;

    #[test]
    fn rejects_gpu_layers_below_the_all_layers_value() {
        assert!(
            from_value::<DesiredModelRuntimeParameters>(json!({ "n_gpu_layers": -2 })).is_err()
        );
    }

    #[test]
    fn omitted_parameters_follow_the_model() {
        assert_eq!(
            from_value::<DesiredModelRuntimeParameters>(json!({ "n_gpu_layers": -1 })).unwrap(),
            DesiredModelRuntimeParameters {
                n_gpu_layers: ModelDefaultOr::Explicit(-1),
                ..DesiredModelRuntimeParameters::default()
            }
        );
    }
}
