use std::str::FromStr as _;

use anyhow::Context as _;
use anyhow::Result;
use paddler_balancer::compatibility::openai_service::configuration::Configuration as OpenAIServiceConfiguration;
use paddler_balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler_balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
use paddler_balancer::state_database_type::StateDatabaseType;
use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_bootstrap::balancer_runner::BalancerRunnerParams;
use tokio_util::sync::CancellationToken;
use trzcina::ServiceShutdownOptions;

use crate::in_process_agent_spawner::InProcessAgentSpawner;
use crate::in_process_balancer::InProcessBalancer;
use paddler_test_cluster_harness::balancer_addresses::BalancerAddresses;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::running_balancer::RunningBalancer;

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
    log::set_max_level(log::LevelFilter::Trace);

    let addresses = BalancerAddresses::pick()?;
    let management_address = addresses.management.to_string();
    let state_database_type = StateDatabaseType::from_str(&state_database_url)
        .context("failed to parse state_database_url")?;

    let balancer_runner = BalancerRunner::start(BalancerRunnerParams {
        buffered_request_timeout,
        inference_service_configuration: InferenceServiceConfiguration {
            addr: addresses.inference,
            cors_allowed_hosts: inference_cors_allowed_hosts,
            inference_item_timeout,
        },
        management_service_configuration: ManagementServiceConfiguration {
            addr: addresses.management,
            cors_allowed_hosts: management_cors_allowed_hosts,
        },
        max_buffered_requests,
        openai_service_configuration: Some(OpenAIServiceConfiguration {
            addr: addresses.compat_openai,
        }),
        cancellation_token: CancellationToken::new(),
        shutdown_options: ServiceShutdownOptions::default(),
        state_database_type,
        statsd_prefix: "paddler_tests_".to_owned(),
        statsd_service_configuration: None,
        #[cfg(feature = "web_admin_panel")]
        web_admin_panel_service_configuration: None,
    })
    .await
    .context("failed to start in-process BalancerRunner")?;

    let running_balancer =
        RunningBalancer::new(addresses, Box::new(InProcessBalancer::new(balancer_runner)));

    let mut cluster = Cluster::connect(
        CancellationToken::new(),
        running_balancer,
        Box::new(InProcessAgentSpawner::new(management_address)),
        desired_state.as_ref(),
    )
    .await?;

    let expected_agent_count = agents.len();
    let mut last_ready_snapshot = None;

    for agent in &agents {
        cluster.spawn_additional_agent(agent)?;

        if wait_for_slots_ready {
            last_ready_snapshot = Some(
                cluster
                    .wait_for_agent_ready(&agent.name, agent.slot_count)
                    .await?,
            );
        }
    }

    let registered_snapshot = match last_ready_snapshot {
        Some(snapshot) => snapshot,
        None => cluster
            .wait_for_agent_count(expected_agent_count)
            .await
            .context("not all in-process agents registered")?,
    };

    cluster.agent_ids = registered_snapshot
        .agents
        .iter()
        .map(|registered_agent| registered_agent.id.clone())
        .collect();

    Ok(cluster)
}
