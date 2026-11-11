use anyhow::Result;

use paddler_test_cluster_harness::running_balancer::RunningBalancer;

use crate::ephemeral_balancer_arguments::ephemeral_balancer_arguments;
use crate::signalled_subprocess::SignalledSubprocess;
use crate::spawn_balancer_subprocess::spawn_balancer_subprocess;
use crate::spawned_balancer_subprocess::SpawnedBalancerSubprocess;

/// # Errors
///
/// Fails when the balancer subprocess cannot be spawned, signalled or addressed, and when it does not announce its addresses.
pub async fn running_balancer_subprocess(binary_path: &str) -> Result<RunningBalancer> {
    spawn_balancer_subprocess(
        binary_path,
        ephemeral_balancer_arguments(),
        SignalledSubprocess::new,
    )
    .await
    .map(
        |SpawnedBalancerSubprocess {
             running_balancer, ..
         }| running_balancer,
    )
}
