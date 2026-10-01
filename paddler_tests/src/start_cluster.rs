use std::str::FromStr as _;

use anyhow::Context as _;
use anyhow::Result;
use log::LevelFilter;
use log::set_max_level;
use tokio_util::sync::CancellationToken;

use paddler_balancer::compatibility::openai_service::configuration::Configuration as OpenAIServiceConfiguration;
use paddler_balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler_balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;
use paddler_bootstrap::balancer_bootstrap_config::BalancerBootstrapConfig;
use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_bootstrap::balancer_runner_params::BalancerRunnerParams;
use paddler_state_database::state_database_type::StateDatabaseType;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;
use paddler_test_cluster_harness::running_balancer::RunningBalancer;
use paddler_test_cluster_harness::unexpiring_shutdown_options::unexpiring_shutdown_options;

use crate::in_process_agent_spawner::InProcessAgentSpawner;
use crate::in_process_balancer::InProcessBalancer;

pub async fn start_cluster(
    ClusterParams {
        agents,
        buffered_request_timeout,
        desired_state,
        inference_cors_allowed_hosts,
        inference_item_timeout,
        management_cors_allowed_hosts,
        max_buffered_requests,
        state_database_url,
        wait_for_slots_ready,
    }: ClusterParams,
) -> Result<Cluster> {
    set_max_level(LevelFilter::Trace);

    let state_database_type = StateDatabaseType::from_str(&state_database_url)
        .context("failed to parse state_database_url")?;

    let balancer_runner = BalancerRunner::start(BalancerRunnerParams {
        bootstrap_config: BalancerBootstrapConfig {
            buffered_request_timeout,
            inference_service_configuration: InferenceServiceConfiguration {
                addr: ResolvedSocketAddr::from(EPHEMERAL_LOOPBACK_ADDR),
                cors_allowed_hosts: inference_cors_allowed_hosts,
                inference_item_timeout,
            },
            management_service_configuration: ManagementServiceConfiguration {
                addr: ResolvedSocketAddr::from(EPHEMERAL_LOOPBACK_ADDR),
                cors_allowed_hosts: management_cors_allowed_hosts,
            },
            max_buffered_requests,
            openai_service_configuration: Some(OpenAIServiceConfiguration {
                addr: ResolvedSocketAddr::from(EPHEMERAL_LOOPBACK_ADDR),
            }),
            state_database_type,
            statsd_prefix: "paddler_tests_".to_owned(),
            statsd_service_configuration: None,
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_service_configuration: None,
        },
        cancellation_token: CancellationToken::new(),
        shutdown_options: unexpiring_shutdown_options(),
    })
    .await
    .context("failed to start in-process BalancerRunner")?;

    let management_address = balancer_runner.addresses.management.to_string();
    let running_balancer = RunningBalancer::new(
        balancer_runner.addresses,
        Box::new(InProcessBalancer::new(balancer_runner)),
    );

    let mut cluster = Cluster::connect(
        CancellationToken::new(),
        running_balancer,
        Box::new(InProcessAgentSpawner::new(management_address)),
        desired_state.as_ref(),
    )
    .await?;

    cluster
        .register_agents(&agents, wait_for_slots_ready)
        .await?;

    Ok(cluster)
}
