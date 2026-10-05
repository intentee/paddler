use std::time::Duration;

use crate::resolved_socket_addr::ResolvedSocketAddr;

#[derive(Clone)]
pub struct Configuration {
    pub statsd_addr: ResolvedSocketAddr,
    pub statsd_reporting_interval: Duration,
}
