use std::time::Duration;

use crate::resolved_socket_addr::ResolvedSocketAddr;
use crate::statsd_service::configuration::Configuration as StatsdServiceConfiguration;

#[derive(Clone)]
pub struct TemplateData {
    pub buffered_request_timeout: Duration,
    pub compat_openai_addr: Option<ResolvedSocketAddr>,
    pub inference_addr: ResolvedSocketAddr,
    pub management_addr: ResolvedSocketAddr,
    pub max_buffered_requests: u64,
    pub statsd_prefix: String,
    pub statsd_service_configuration: Option<StatsdServiceConfiguration>,
}
