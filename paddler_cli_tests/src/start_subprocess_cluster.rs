use std::process::Stdio;

use anyhow::Context as _;
use anyhow::Result;

use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::running_balancer::RunningBalancer;

use tokio_util::sync::CancellationToken;

use crate::paddler_command::paddler_command;
use crate::read_balancer_addresses::read_balancer_addresses;
use crate::subprocess_agent_spawner::SubprocessAgentSpawner;
use crate::subprocess_cluster_error::SubprocessClusterError;
use crate::subprocess_process::SubprocessProcess;

const EPHEMERAL_LOOPBACK_ADDR: &str = "127.0.0.1:0";

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
    let mut balancer_command = paddler_command(binary_path);

    balancer_command
        .arg("balancer")
        .arg("--inference-addr")
        .arg(EPHEMERAL_LOOPBACK_ADDR)
        .arg("--management-addr")
        .arg(EPHEMERAL_LOOPBACK_ADDR)
        .arg("--compat-openai-addr")
        .arg(EPHEMERAL_LOOPBACK_ADDR)
        .arg("--state-database")
        .arg(&state_database_url)
        .arg("--max-buffered-requests")
        .arg(max_buffered_requests.to_string())
        .arg("--buffered-request-timeout")
        .arg(buffered_request_timeout.as_millis().to_string())
        .arg("--inference-item-timeout")
        .arg(inference_item_timeout.as_millis().to_string())
        .stdout(Stdio::piped())
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

    let mut balancer_subprocess = balancer_command
        .spawn()
        .context("failed to spawn paddler balancer subprocess")?;
    let balancer_stdout = balancer_subprocess
        .stdout
        .take()
        .ok_or(SubprocessClusterError::StdoutNotPiped)?;
    let addresses = read_balancer_addresses(balancer_stdout).await?;

    let running_balancer = RunningBalancer::new(
        addresses,
        Box::new(SubprocessProcess::new(balancer_subprocess)),
    );

    let mut cluster = Cluster::connect(
        CancellationToken::new(),
        running_balancer,
        Box::new(SubprocessAgentSpawner::new(
            binary_path.to_owned(),
            addresses.management,
        )),
        desired_state.as_ref(),
    )
    .await?;

    cluster
        .register_agents(&agents, wait_for_slots_ready)
        .await?;

    Ok(cluster)
}
