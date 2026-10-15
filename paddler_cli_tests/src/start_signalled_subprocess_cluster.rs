use anyhow::Result;

use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;

use crate::connect_subprocess_balancer::connect_subprocess_balancer;
use crate::subprocess_cluster::SubprocessCluster;

pub async fn start_signalled_subprocess_cluster(
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
) -> Result<SubprocessCluster> {
    let ephemeral_loopback_addr = EPHEMERAL_LOOPBACK_ADDR.to_string();
    let mut balancer_arguments = vec![
        "--compat-openai-addr".to_owned(),
        ephemeral_loopback_addr.clone(),
        "--compat-typesafe-addr".to_owned(),
        ephemeral_loopback_addr.clone(),
        "--inference-addr".to_owned(),
        ephemeral_loopback_addr.clone(),
        "--management-addr".to_owned(),
        ephemeral_loopback_addr,
        "--state-database".to_owned(),
        state_database_url,
        "--max-buffered-requests".to_owned(),
        max_buffered_requests.to_string(),
        "--buffered-request-timeout".to_owned(),
        buffered_request_timeout.as_millis().to_string(),
        "--inference-item-timeout".to_owned(),
        inference_item_timeout.as_millis().to_string(),
    ];

    for allowed_host in inference_cors_allowed_hosts {
        balancer_arguments.extend(["--inference-cors-allowed-host".to_owned(), allowed_host]);
    }

    for allowed_host in management_cors_allowed_hosts {
        balancer_arguments.extend(["--management-cors-allowed-host".to_owned(), allowed_host]);
    }

    let SubprocessCluster {
        balancer_signals,
        mut cluster,
    } = connect_subprocess_balancer(binary_path, balancer_arguments, &desired_state).await?;

    cluster
        .register_agents(&agents, wait_for_slots_ready)
        .await?;

    Ok(SubprocessCluster {
        balancer_signals,
        cluster,
    })
}
