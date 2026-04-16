use anyhow::Result;
use anyhow::bail;
use llama_cpp_bindings::llama_backend::LlamaBackend;
use llama_cpp_bindings::llama_backend_device::LlamaBackendDevice;
use llama_cpp_bindings::llama_backend_device::LlamaBackendDeviceType;
use llama_cpp_bindings::llama_backend_device::list_llama_ggml_backend_devices;

pub fn require_cuda_device() -> Result<Vec<LlamaBackendDevice>> {
    let backend = LlamaBackend::init()?;
    if !backend.supports_gpu_offload() {
        bail!("binary built without GPU offload support; rebuild with --features cuda");
    }
    drop(backend);

    let cuda_devices: Vec<LlamaBackendDevice> = list_llama_ggml_backend_devices()
        .into_iter()
        .filter(|device| {
            device.backend == "CUDA" && device.device_type == LlamaBackendDeviceType::Gpu
        })
        .collect();

    if cuda_devices.is_empty() {
        bail!("no CUDA GPU devices detected at runtime");
    }

    Ok(cuda_devices)
}
