use std::net::SocketAddr;
use std::sync::Arc;
#[cfg(feature = "statsd_reporter")]
use std::time::Duration;
use log::info;

use anyhow::Result;
use pingora::proxy::http_proxy_service;
use pingora::server::configuration::Opt;
use pingora::server::Server;

use crate::balancer::management_service::ManagementService;
use crate::balancer::proxy_service::ProxyService;
#[cfg(feature = "statsd_reporter")]
use crate::balancer::statsd_service::StatsdService;
use crate::balancer::upstream_peer_pool::UpstreamPeerPool;

#[expect(clippy::too_many_arguments)]
pub fn handle(
    buffered_request_timeout: Duration,
    management_addr: &SocketAddr,
    management_cors_allowed_hosts: Vec<String>,
    #[cfg(feature = "web_dashboard")] management_dashboard_enable: bool,
    max_buffered_requests: usize,
    metrics_endpoint_enable: bool,
    reverseproxy_addr: &SocketAddr,
    rewrite_host_header: bool,
    check_model: bool,
    slots_endpoint_enable: bool,
    #[cfg(feature = "statsd_reporter")] statsd_addr: Option<SocketAddr>,
    #[cfg(feature = "statsd_reporter")] statsd_prefix: String,
    #[cfg(feature = "statsd_reporter")] statsd_reporting_interval: Duration,
) -> Result<()> {
    let mut pingora_server = Server::new(Opt {
        upgrade: false,
        daemon: false,
        nocapture: false,
        test: false,
        conf: None,
    })?;

    pingora_server.bootstrap();

    let upstream_peer_pool = Arc::new(UpstreamPeerPool::new());

    let mut proxy_service = http_proxy_service(
        &pingora_server.configuration,
        ProxyService::new(
            rewrite_host_header,
            check_model,
            slots_endpoint_enable,
            upstream_peer_pool.clone(),
            buffered_request_timeout,
            max_buffered_requests,
        ),
    );

    proxy_service.add_tcp(&reverseproxy_addr.clone().to_string());

    pingora_server.add_service(proxy_service);
    pingora_server.add_service(ManagementService::new(
        *management_addr,
        management_cors_allowed_hosts,
        #[cfg(feature = "web_dashboard")]
        management_dashboard_enable,
        metrics_endpoint_enable,
        upstream_peer_pool.clone(),
    ));

    #[cfg(feature = "statsd_reporter")]
    if let Some(statsd_addr) = statsd_addr {
        let statsd_service = StatsdService::new(
            statsd_addr,
            statsd_prefix,
            statsd_reporting_interval,
            upstream_peer_pool.clone(),
        )?;

        pingora_server.add_service(statsd_service);
    }

    info!("rewrite_host_header? {} check_model? {} slots_endpoint_enable? {}", rewrite_host_header, check_model, slots_endpoint_enable);

    pingora_server.run_forever();
}
