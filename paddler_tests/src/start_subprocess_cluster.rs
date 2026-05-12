use std::process::Stdio;

use anyhow::Context as _;
use anyhow::Result;
use paddler_client::PaddlerClient;
use tokio::process::Child;
use tokio_util::sync::CancellationToken;

use crate::agents_stream_watcher::AgentsStreamWatcher;
use crate::balancer_addresses::BalancerAddresses;
use crate::buffered_requests_stream_watcher::BufferedRequestsStreamWatcher;
use crate::cluster_completion::ClusterCompletion;
use crate::cluster_handle::ClusterHandle;
use crate::cluster_handle_params::ClusterHandleParams;
use crate::paddler_command::paddler_command;
use crate::subprocess_cluster_params::SubprocessClusterParams;
use crate::wait_until_healthy::wait_until_healthy;

pub async fn start_subprocess_cluster(
    SubprocessClusterParams {
        agents,
        buffered_request_timeout,
        desired_state,
        inference_cors_allowed_hosts,
        inference_item_timeout,
        management_cors_allowed_hosts,
        max_buffered_requests,
        state_database_url,
        wait_for_slots_ready,
    }: SubprocessClusterParams,
) -> Result<ClusterHandle> {
    let addresses = BalancerAddresses::pick()?;

    let mut balancer_command = paddler_command();

    balancer_command
        .arg("balancer")
        .arg("--inference-addr")
        .arg(addresses.inference.to_string())
        .arg("--management-addr")
        .arg(addresses.management.to_string())
        .arg("--compat-openai-addr")
        .arg(addresses.compat_openai.to_string())
        .arg("--state-database")
        .arg(&state_database_url)
        .arg("--max-buffered-requests")
        .arg(max_buffered_requests.to_string())
        .arg("--buffered-request-timeout")
        .arg(buffered_request_timeout.as_millis().to_string())
        .arg("--inference-item-timeout")
        .arg(inference_item_timeout.as_millis().to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    for allowed_host in &inference_cors_allowed_hosts {
        balancer_command
            .arg("--inference-cors-allowed-host")
            .arg(allowed_host);
    }

    for allowed_host in &management_cors_allowed_hosts {
        balancer_command
            .arg("--management-cors-allowed-host")
            .arg(allowed_host);
    }

    let balancer = balancer_command
        .spawn()
        .context("failed to spawn paddler balancer subprocess")?;

    let management_base_url = addresses.management_base_url()?;
    let inference_base_url = addresses.inference_base_url()?;

    wait_until_healthy(&management_base_url, "health")
        .await
        .context("subprocess balancer did not become healthy")?;

    let paddler_client = PaddlerClient::new(inference_base_url, management_base_url, 1);

    if let Some(desired_state) = desired_state.as_ref() {
        paddler_client
            .management()
            .put_balancer_desired_state(desired_state)
            .await
            .map_err(anyhow::Error::new)
            .context("failed to PUT desired state on subprocess balancer")?;
    }

    let mut agents_watcher = AgentsStreamWatcher::connect(&paddler_client.management()).await?;
    let buffered_requests_watcher =
        BufferedRequestsStreamWatcher::connect(&paddler_client.management()).await?;

    let expected_agent_count = agents.len();
    let mut agent_children: Vec<Child> = Vec::with_capacity(expected_agent_count);

    for agent in &agents {
        let agent_child = paddler_command()
            .arg("agent")
            .arg("--management-addr")
            .arg(addresses.management.to_string())
            .arg("--name")
            .arg(&agent.name)
            .arg("--slots")
            .arg(agent.slot_count.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("failed to spawn paddler agent subprocess")?;

        agent_children.push(agent_child);
    }

    let registered_snapshot = agents_watcher
        .until(move |snapshot| snapshot.agents.len() >= expected_agent_count)
        .await
        .context("not all subprocess agents registered")?;

    let agent_ids: Vec<String> = registered_snapshot
        .agents
        .iter()
        .map(|registered_agent| registered_agent.id.clone())
        .collect();

    if wait_for_slots_ready {
        let expected_slot_counts: Vec<i32> = agents.iter().map(|agent| agent.slot_count).collect();

        agents_watcher
            .wait_for_slots_ready(&expected_slot_counts)
            .await?;
    }

    Ok(ClusterHandle::new(ClusterHandleParams {
        addresses,
        agent_ids,
        agents: agents_watcher,
        buffered_requests: buffered_requests_watcher,
        cancel_token: CancellationToken::new(),
        completion: ClusterCompletion::Subprocess {
            agents: agent_children,
            balancer,
        },
        paddler_client,
    }))
}
