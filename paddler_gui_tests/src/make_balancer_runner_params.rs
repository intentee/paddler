use std::time::Duration;

use paddler::balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler::balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
use paddler::balancer::state_database_type::StateDatabaseType;
use paddler_bootstrap::balancer_runner::BalancerRunnerParams;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use tokio_util::sync::CancellationToken;

use crate::bind_addresses::BindAddresses;

pub fn make_balancer_runner_params(
    addrs: BindAddresses,
    cancellation_token: CancellationToken,
) -> BalancerRunnerParams {
    BalancerRunnerParams {
        buffered_request_timeout: Duration::from_secs(10),
        inference_listener: None,
        inference_service_configuration: InferenceServiceConfiguration {
            addr: addrs.inference_addr,
            cors_allowed_hosts: vec![],
            inference_item_timeout: Duration::from_secs(30),
        },
        management_listener: None,
        management_service_configuration: ManagementServiceConfiguration {
            addr: addrs.management_addr,
            cors_allowed_hosts: vec![],
        },
        max_buffered_requests: 30,
        openai_listener: None,
        openai_service_configuration: None,
        cancellation_token,
        state_database_type: StateDatabaseType::Memory(Box::new(BalancerDesiredState::default())),
        statsd_prefix: "paddler_test_".to_owned(),
        statsd_service_configuration: None,
        #[cfg(feature = "web_admin_panel")]
        web_admin_panel_listener: None,
        #[cfg(feature = "web_admin_panel")]
        web_admin_panel_service_configuration: None,
    }
}
