use anyhow::Result;
use tokio_util::sync::CancellationToken;

use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;

use crate::spawn_balancer_subprocess::spawn_balancer_subprocess;
use crate::spawned_balancer_subprocess::SpawnedBalancerSubprocess;
use crate::subprocess_agent_spawner::SubprocessAgentSpawner;

pub async fn restart_balancer(
    binary_path: &str,
    Cluster {
        agents, balancer, ..
    }: Cluster,
) -> Result<Cluster> {
    let management_addr = balancer.addresses.management;

    balancer.shutdown().await?;

    let SpawnedBalancerSubprocess {
        running_balancer, ..
    } = spawn_balancer_subprocess(
        binary_path,
        [
            "--inference-addr".to_owned(),
            EPHEMERAL_LOOPBACK_ADDR.to_string(),
            "--management-addr".to_owned(),
            management_addr.to_string(),
        ],
    )
    .await?;
    let mut restarted_cluster = Cluster::connect(
        CancellationToken::new(),
        running_balancer,
        Box::new(SubprocessAgentSpawner::new(
            binary_path.to_owned(),
            management_addr,
        )),
        &ClusterDesiredState::KeepStored,
    )
    .await?;

    restarted_cluster.agents = agents;

    Ok(restarted_cluster)
}
