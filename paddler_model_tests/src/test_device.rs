use anyhow::Result;
use paddler_types::inference_parameters::InferenceParameters;

#[cfg(feature = "cuda")]
use crate::test_device_cuda::require_cuda_device;
#[cfg(feature = "metal")]
use crate::test_device_metal::require_metal_device;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TestDevice {
    Cpu,
    #[cfg(feature = "cuda")]
    Cuda,
    #[cfg(feature = "metal")]
    Metal,
}

impl TestDevice {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            #[cfg(feature = "cuda")]
            Self::Cuda => "cuda",
            #[cfg(feature = "metal")]
            Self::Metal => "metal",
        }
    }

    #[cfg_attr(
        not(any(feature = "cuda", feature = "metal")),
        expect(
            clippy::missing_const_for_fn,
            reason = "branches are non-const when GPU features are enabled"
        )
    )]
    pub fn require_available(self) -> Result<()> {
        match self {
            Self::Cpu => Ok(()),
            #[cfg(feature = "cuda")]
            Self::Cuda => require_cuda_device().map(|_| ()),
            #[cfg(feature = "metal")]
            Self::Metal => require_metal_device().map(|_| ()),
        }
    }

    #[must_use]
    pub fn inference_parameters_for_full_offload(
        self,
        gpu_layer_count: u32,
    ) -> InferenceParameters {
        #[cfg(not(any(feature = "cuda", feature = "metal")))]
        let _ = gpu_layer_count;

        match self {
            Self::Cpu => InferenceParameters::default(),
            #[cfg(feature = "cuda")]
            Self::Cuda => InferenceParameters {
                n_gpu_layers: gpu_layer_count,
                ..InferenceParameters::default()
            },
            #[cfg(feature = "metal")]
            Self::Metal => InferenceParameters {
                n_gpu_layers: gpu_layer_count,
                ..InferenceParameters::default()
            },
        }
    }
}
