use std::time::Duration;

pub const BALANCER_DEFAULT_BUFFERED_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
pub const BALANCER_DEFAULT_INFERENCE_ITEM_TIMEOUT: Duration = Duration::from_secs(30);
pub const BALANCER_DEFAULT_INFERENCE_PORT: u16 = 8061;
pub const BALANCER_DEFAULT_MANAGEMENT_PORT: u16 = 8060;
pub const BALANCER_DEFAULT_MAX_BUFFERED_REQUESTS: u64 = 30;
pub const BALANCER_DEFAULT_STATSD_PREFIX: &str = "paddler_";
pub const BALANCER_DEFAULT_STATSD_REPORTING_INTERVAL: Duration = Duration::from_secs(10);
pub const CUDA_DISCLAIMER_DOCS: &str = "
This software includes NVIDIA CUDA runtime components, subject to the NVIDIA CUDA Toolkit End User License Agreement: https://docs.nvidia.com/cuda/eula/index.html
This software contains source code provided by NVIDIA Corporation.
Paddler is not affiliated with, endorsed by, or sponsored by NVIDIA Corporation.";
pub const LLAMA_CPP_MAX_SEQUENCES: u16 = 256;
