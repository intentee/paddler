use std::mem::discriminant;
use std::os::unix::process::ExitStatusExt as _;
use std::process::ExitStatus;

use paddler_cli_tests::start_subprocess_cluster::start_subprocess_cluster;
use paddler_cli_tests::subprocess_cluster_error::SubprocessClusterError;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

use crate::slots_beyond_what_llama_cpp_can_serve::SLOTS_BEYOND_WHAT_LLAMA_CPP_CAN_SERVE;

#[tokio::test(flavor = "multi_thread")]
async fn subprocess_cluster_shutdown_reports_an_agent_that_exited_with_a_failure() {
    let mut cluster = start_subprocess_cluster(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        ClusterParams::without_agents(),
    )
    .await
    .expect("a cluster without agents must start");

    cluster
        .spawn_additional_agent(&AgentConfig::single(SLOTS_BEYOND_WHAT_LLAMA_CPP_CAN_SERVE))
        .expect("the agent process must spawn");

    assert_eq!(
        cluster
            .shutdown()
            .await
            .err()
            .as_ref()
            .and_then(|shutdown_error| shutdown_error.downcast_ref::<SubprocessClusterError>())
            .map(discriminant),
        Some(discriminant(
            &SubprocessClusterError::ProcessExitedWithFailure {
                exit_status: ExitStatus::from_raw(0),
            }
        ))
    );
}
