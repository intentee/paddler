use std::time::Duration;

use crate::resolved_socket_addr::ResolvedSocketAddr;

#[derive(Clone)]
pub struct TemplateData {
    pub buffered_request_timeout: Duration,
    pub compat_openai_addr: Option<ResolvedSocketAddr>,
    pub inference_addr: ResolvedSocketAddr,
    pub management_addr: ResolvedSocketAddr,
    pub max_buffered_requests: i32,
    pub statsd_addr: Option<ResolvedSocketAddr>,
    pub statsd_prefix: String,
    pub statsd_reporting_interval: Duration,
}
