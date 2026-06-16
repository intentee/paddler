use std::time::Duration;

pub struct BalancerServiceConfig {
    pub buffered_request_timeout: Duration,
    pub inference_cors_allowed_hosts: Vec<String>,
    pub inference_item_timeout: Duration,
    pub management_cors_allowed_hosts: Vec<String>,
    pub max_buffered_requests: i32,
    pub state_database_url: String,
}

impl Default for BalancerServiceConfig {
    fn default() -> Self {
        Self {
            buffered_request_timeout: Duration::from_secs(10),
            inference_cors_allowed_hosts: Vec::new(),
            inference_item_timeout: Duration::from_secs(30),
            management_cors_allowed_hosts: Vec::new(),
            max_buffered_requests: 10,
            state_database_url: "memory://".to_owned(),
        }
    }
}
