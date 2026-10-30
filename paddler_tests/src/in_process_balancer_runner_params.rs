use std::str::FromStr as _;

use anyhow::Context as _;
use anyhow::Result;
use tokio_util::sync::CancellationToken;

use paddler_balancer::compatibility::compatibility_service_configuration::CompatibilityServiceConfiguration;
use paddler_balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler_balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;
use paddler_bootstrap::balancer_bootstrap_config::BalancerBootstrapConfig;
use paddler_bootstrap::balancer_runner_params::BalancerRunnerParams;
use paddler_state_database::state_database_type::StateDatabaseType;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;
use paddler_test_cluster_harness::unexpiring_shutdown_options::unexpiring_shutdown_options;

pub fn in_process_balancer_runner_params(
    ClusterParams {
        buffered_request_timeout,
        inference_cors_allowed_hosts,
        inference_item_timeout,
        management_cors_allowed_hosts,
        max_buffered_requests,
        state_database_url,
        ..
    }: &ClusterParams,
) -> Result<BalancerRunnerParams> {
    Ok(BalancerRunnerParams {
        bootstrap_config: BalancerBootstrapConfig {
            buffered_request_timeout: *buffered_request_timeout,
            inference_service_configuration: InferenceServiceConfiguration {
                addr: ResolvedSocketAddr::from(EPHEMERAL_LOOPBACK_ADDR),
                cors_allowed_hosts: inference_cors_allowed_hosts.clone(),
                inference_item_timeout: *inference_item_timeout,
            },
            management_service_configuration: ManagementServiceConfiguration {
                addr: ResolvedSocketAddr::from(EPHEMERAL_LOOPBACK_ADDR),
                cors_allowed_hosts: management_cors_allowed_hosts.clone(),
            },
            max_buffered_requests: *max_buffered_requests,
            openai_service_configuration: Some(CompatibilityServiceConfiguration {
                addr: ResolvedSocketAddr::from(EPHEMERAL_LOOPBACK_ADDR),
            }),
            state_database_type: StateDatabaseType::from_str(state_database_url)
                .context("failed to parse state_database_url")?,
            statsd_prefix: "paddler_tests_".to_owned(),
            statsd_service_configuration: None,
            typesafe_service_configuration: Some(CompatibilityServiceConfiguration {
                addr: ResolvedSocketAddr::from(EPHEMERAL_LOOPBACK_ADDR),
            }),
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_service_configuration: None,
        },
        cancellation_token: CancellationToken::new(),
        shutdown_options: unexpiring_shutdown_options(),
    })
}
