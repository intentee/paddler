use std::process::Stdio;

use anyhow::Context as _;
use anyhow::Result;

use paddler_test_cluster_harness::balancer_addresses::BalancerAddresses;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::running_balancer::RunningBalancer;

use tokio_util::sync::CancellationToken;

use crate::paddler_command::paddler_command;
use crate::subprocess_agent_spawner::SubprocessAgentSpawner;
use crate::subprocess_process::SubprocessProcess;

pub async fn start_subprocess_cluster(
    binary_path: &str,
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
    let addresses = BalancerAddresses::pick()?;
    let management_addr = addresses.management;

    let mut balancer_command = paddler_command(binary_path);

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

    let balancer_subprocess = balancer_command
        .spawn()
        .context("failed to spawn paddler balancer subprocess")?;

    let running_balancer = RunningBalancer::new(
        addresses,
        Box::new(SubprocessProcess::new(balancer_subprocess)),
    );

    let mut cluster = Cluster::connect(
        CancellationToken::new(),
        running_balancer,
        Box::new(SubprocessAgentSpawner::new(
            binary_path.to_owned(),
            management_addr,
        )),
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
            .context("not all subprocess agents registered")?,
    };

    cluster.agent_ids = registered_snapshot
        .agents
        .iter()
        .map(|registered_agent| registered_agent.id.clone())
        .collect();

    Ok(cluster)
}
