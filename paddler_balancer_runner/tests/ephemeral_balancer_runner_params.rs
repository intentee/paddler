use tokio_util::sync::CancellationToken;

use paddler_balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler_balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;
use paddler_balancer_runner::balancer_runner_config::BalancerRunnerConfig;
use paddler_balancer_runner::balancer_runner_params::BalancerRunnerParams;
use paddler_balancer_runner::balancer_serving_mode::BalancerServingMode;
use paddler_state_database::state_database_type::StateDatabaseType;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;
use paddler_test_cluster_harness::longer_than_any_test_run::LONGER_THAN_ANY_TEST_RUN;
use paddler_test_cluster_harness::unexpiring_shutdown_options::unexpiring_shutdown_options;

pub fn ephemeral_balancer_runner_params(
    cancellation_token: CancellationToken,
) -> BalancerRunnerParams {
    BalancerRunnerParams {
        runner_config: BalancerRunnerConfig {
            buffered_request_timeout: LONGER_THAN_ANY_TEST_RUN,
            inference_service_configuration: InferenceServiceConfiguration {
                addr: ResolvedSocketAddr::from(EPHEMERAL_LOOPBACK_ADDR),
                cors_allowed_hosts: vec![],
                inference_item_timeout: LONGER_THAN_ANY_TEST_RUN,
            },
            management_service_configuration: ManagementServiceConfiguration {
                addr: ResolvedSocketAddr::from(EPHEMERAL_LOOPBACK_ADDR),
                cors_allowed_hosts: vec![],
            },
            max_buffered_requests: 30,
            serving_mode: BalancerServingMode::TextGeneration {
                openai_service_configuration: None,
            },
            state_database_type: StateDatabaseType::Memory,
            statsd_prefix: "paddler_balancer_runner_test_".to_owned(),
            statsd_service_configuration: None,
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_service_configuration: None,
        },
        cancellation_token,
        shutdown_options: unexpiring_shutdown_options(),
    }
}
