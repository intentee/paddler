use std::time::Duration;

pub const BALANCER_DEFAULT_BUFFERED_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
pub const BALANCER_DEFAULT_INFERENCE_ITEM_TIMEOUT: Duration = Duration::from_secs(30);
pub const BALANCER_DEFAULT_INFERENCE_PORT: u16 = 8061;
pub const BALANCER_DEFAULT_MANAGEMENT_PORT: u16 = 8060;
pub const BALANCER_DEFAULT_MAX_BUFFERED_REQUESTS: u64 = 30;
pub const BALANCER_DEFAULT_STATSD_PREFIX: &str = "paddler_";
pub const BALANCER_DEFAULT_STATSD_REPORTING_INTERVAL: Duration = Duration::from_secs(10);
pub const LLAMA_CPP_MAX_SEQUENCES: u16 = 256;
