use std::time::Duration;

use crate::resolved_socket_addr::ResolvedSocketAddr;

#[derive(Clone)]
pub struct Configuration {
    pub addr: ResolvedSocketAddr,
    pub cors_allowed_hosts: Vec<String>,
    pub inference_item_timeout: Duration,
}
