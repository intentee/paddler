use std::ffi::OsStr;

use anyhow::Result;
use tokio_util::sync::CancellationToken;

use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;

use crate::signalled_subprocess::SignalledSubprocess;
use crate::spawn_balancer_subprocess::spawn_balancer_subprocess;
use crate::spawned_balancer_subprocess::SpawnedBalancerSubprocess;
use crate::subprocess_agents_cluster_connection::SubprocessAgentsClusterConnection;
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
    let cluster = SubprocessAgentsClusterConnection {
        agent_binary_path: binary_path,
        cancellation_token: CancellationToken::new(),
        desired_state,
        running_balancer,
    }
    .connect()
    .await?;

    Ok(SubprocessCluster {
        balancer_signals,
        cluster,
    })
}
