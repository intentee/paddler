use paddler_cli_tests::signalled_subprocess::SignalledSubprocess;
use paddler_cli_tests::spawn_balancer_subprocess::spawn_balancer_subprocess;
use paddler_test_cluster_harness::running_balancer::RunningBalancer;

use crate::ephemeral_balancer_arguments::ephemeral_balancer_arguments;

pub async fn running_balancer_subprocess() -> RunningBalancer {
    spawn_balancer_subprocess(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        ephemeral_balancer_arguments(),
        SignalledSubprocess::new,
    )
    .await
    .expect("the balancer must start and announce its addresses")
    .running_balancer
}
