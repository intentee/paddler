use anyhow::Result;
#[cfg(any(feature = "cuda", feature = "metal"))]
use anyhow::bail;
#[cfg(any(feature = "cuda", feature = "metal"))]
use paddler::llama_cpp_bindings::llama_backend::LlamaBackend;
#[cfg(any(feature = "cuda", feature = "metal"))]
use paddler::llama_cpp_bindings::llama_backend_device::LlamaBackendDeviceType;
#[cfg(any(feature = "cuda", feature = "metal"))]
use paddler::llama_cpp_bindings::llama_backend_device::list_llama_ggml_backend_devices;
use paddler_types::inference_parameters::InferenceParameters;

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
            reason = "non-const branches appear under GPU feature flags"
        )
    )]
    pub fn require_available(self) -> Result<()> {
        match self {
            Self::Cpu => Ok(()),
            #[cfg(feature = "cuda")]
            Self::Cuda => require_backend_device("CUDA"),
            #[cfg(feature = "metal")]
            Self::Metal => require_backend_device("MTL"),
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

#[cfg(any(feature = "cuda", feature = "metal"))]
fn require_backend_device(backend_name: &str) -> Result<()> {
    let backend = LlamaBackend::init()?;

    if !backend.supports_gpu_offload() {
        bail!(
            "binary built without GPU offload support; rebuild with --features cuda or --features metal"
        );
    }

    drop(backend);

    let devices_found = list_llama_ggml_backend_devices().into_iter().any(|device| {
        device.backend == backend_name && device.device_type == LlamaBackendDeviceType::Gpu
    });

    if devices_found {
        Ok(())
    } else {
        bail!("no {backend_name} GPU devices detected at runtime")
    }
}
