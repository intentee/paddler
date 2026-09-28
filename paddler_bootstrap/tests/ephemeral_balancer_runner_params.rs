use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::time::Duration;

use paddler_balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler_balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
use paddler_balancer::state_database_type::StateDatabaseType;
use paddler_bootstrap::balancer_runner_params::BalancerRunnerParams;
use tokio_util::sync::CancellationToken;
use trzcina::ServiceShutdownOptions;

const EPHEMERAL_LOOPBACK_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);

pub fn ephemeral_balancer_runner_params(
    cancellation_token: CancellationToken,
) -> BalancerRunnerParams {
    BalancerRunnerParams {
        buffered_request_timeout: Duration::from_secs(10),
        inference_service_configuration: InferenceServiceConfiguration {
            addr: EPHEMERAL_LOOPBACK_ADDR,
            cors_allowed_hosts: vec![],
            inference_item_timeout: Duration::from_secs(30),
        },
        management_service_configuration: ManagementServiceConfiguration {
            addr: EPHEMERAL_LOOPBACK_ADDR,
            cors_allowed_hosts: vec![],
        },
        max_buffered_requests: 30,
        openai_service_configuration: None,
        cancellation_token,
        shutdown_options: ServiceShutdownOptions::default(),
        state_database_type: StateDatabaseType::Memory(Box::default()),
        statsd_prefix: "paddler_bootstrap_test_".to_owned(),
        statsd_service_configuration: None,
        #[cfg(feature = "web_admin_panel")]
        web_admin_panel_service_configuration: None,
    }
}
