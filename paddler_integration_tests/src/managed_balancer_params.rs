use std::time::Duration;

pub struct ManagedBalancerParams {
    pub buffered_request_timeout: Duration,
    pub compat_openai_addr: String,
    pub inference_addr: String,
    pub inference_cors_allowed_hosts: Vec<String>,
    pub inference_item_timeout: Option<Duration>,
    pub management_addr: String,
    pub management_cors_allowed_hosts: Vec<String>,
    pub max_buffered_requests: i32,
    pub state_database_url: String,
}
