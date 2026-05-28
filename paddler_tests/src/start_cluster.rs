use std::str::FromStr as _;

use anyhow::Context as _;
use anyhow::Result;
use paddler::balancer::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler::balancer::compatibility::openai_service::configuration::Configuration as OpenAIServiceConfiguration;
use paddler::balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler::balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
use paddler::balancer::state_database_type::StateDatabaseType;
use paddler_bootstrap::agent_runner::AgentRunner;
use paddler_bootstrap::agent_runner::AgentRunnerParams;
use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_bootstrap::balancer_runner::BalancerRunnerParams;
use paddler_client::PaddlerClient;
use tokio_util::sync::CancellationToken;

use crate::agents_stream_watcher::AgentsStreamWatcher;
use crate::balancer_addresses::BalancerAddresses;
use crate::buffered_requests_stream_watcher::BufferedRequestsStreamWatcher;
use crate::cluster_handle::ClusterHandle;
use crate::cluster_handle_params::ClusterHandleParams;
use crate::cluster_params::ClusterParams;
use crate::wait_until_healthy::wait_until_healthy;

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
) -> Result<ClusterHandle> {
    let addresses = BalancerAddresses::pick()?;
    let cancel_token = CancellationToken::new();
    let state_database_type = StateDatabaseType::from_str(&state_database_url)
        .context("failed to parse state_database_url")?;

    let balancer = BalancerRunner::start(BalancerRunnerParams {
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
        cancellation_token: cancel_token.clone(),
        state_database_type,
        statsd_prefix: "paddler_tests_".to_owned(),
        statsd_service_configuration: None,
        #[cfg(feature = "web_admin_panel")]
        web_admin_panel_service_configuration: None,
    })
    .await
    .context("failed to start in-process BalancerRunner")?;

    let management_base_url = addresses.management_base_url()?;
    let inference_base_url = addresses.inference_base_url()?;

    wait_until_healthy(&management_base_url, "health")
        .await
        .context("balancer did not become healthy")?;

    let paddler_client = PaddlerClient::new(inference_base_url, management_base_url, 1);

    if let Some(desired_state) = desired_state.as_ref() {
        paddler_client
            .management()
            .put_balancer_desired_state(desired_state)
            .await
            .map_err(anyhow::Error::new)
            .context("failed to PUT balancer desired state")?;
    }

    let mut agents_watcher = AgentsStreamWatcher::connect(&paddler_client.management()).await?;
    let buffered_requests_watcher =
        BufferedRequestsStreamWatcher::connect(&paddler_client.management()).await?;

    let expected_agent_count = agents.len();
    let mut agent_runners: Vec<AgentRunner> = Vec::with_capacity(expected_agent_count);
    let mut last_ready_snapshot: Option<AgentControllerPoolSnapshot> = None;

    for agent in &agents {
        let agent_runner = AgentRunner::start(AgentRunnerParams {
            agent_name: Some(agent.name.clone()),
            management_address: addresses.management.to_string(),
            cancellation_token: cancel_token.clone(),
            slots: agent.slot_count,
        });

        agent_runners.push(agent_runner);

        if wait_for_slots_ready {
            last_ready_snapshot = Some(
                agents_watcher
                    .wait_for_agent_ready(&agent.name, agent.slot_count)
                    .await?,
            );
        }
    }

    let registered_snapshot = match last_ready_snapshot {
        Some(snapshot) => snapshot,
        None => agents_watcher
            .until(move |snapshot| snapshot.agents.len() >= expected_agent_count)
            .await
            .context("not all in-process agents registered")?,
    };

    let agent_ids: Vec<String> = registered_snapshot
        .agents
        .iter()
        .map(|registered_agent| registered_agent.id.clone())
        .collect();

    Ok(ClusterHandle::new(ClusterHandleParams {
        addresses,
        agent_ids,
        agent_runners,
        agents: agents_watcher,
        balancer_runner: balancer,
        buffered_requests: buffered_requests_watcher,
        cancel_token,
        paddler_client,
    }))
}
