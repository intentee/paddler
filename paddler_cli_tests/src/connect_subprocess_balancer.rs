use std::ffi::OsStr;

use anyhow::Result;
use tokio_util::sync::CancellationToken;

use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;

use crate::signalled_subprocess::SignalledSubprocess;
use crate::spawn_balancer_subprocess::spawn_balancer_subprocess;
use crate::spawned_balancer_subprocess::SpawnedBalancerSubprocess;
use crate::subprocess_agent_spawner::SubprocessAgentSpawner;
use crate::subprocess_cluster::SubprocessCluster;

pub async fn connect_subprocess_balancer<TArguments, TArgument>(
    binary_path: &str,
    balancer_arguments: TArguments,
    desired_state: &ClusterDesiredState,
) -> Result<SubprocessCluster>
where
    TArguments: IntoIterator<Item = TArgument>,
    TArgument: AsRef<OsStr>,
{
    let SpawnedBalancerSubprocess {
        running_balancer,
        signals: balancer_signals,
    } = spawn_balancer_subprocess(binary_path, balancer_arguments, SignalledSubprocess::new)
        .await?;
    let management_addr = running_balancer.addresses.management;
    let cluster = Cluster::connect(
        CancellationToken::new(),
        running_balancer,
        Box::new(SubprocessAgentSpawner::new(
            binary_path.to_owned(),
            management_addr,
        )),
        desired_state,
    )
    .await?;

    Ok(SubprocessCluster {
        balancer_signals,
        cluster,
    })
}
