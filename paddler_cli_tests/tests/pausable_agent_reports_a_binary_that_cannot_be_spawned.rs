use std::io;
use std::io::ErrorKind;

use paddler_cli_tests::pausable_agent::PausableAgent;
use paddler_cli_tests::pausable_agent_params::PausableAgentParams;
use paddler_cli_tests::start_subprocess_cluster::start_subprocess_cluster;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

const MISSING_BINARY_PATH: &str = "/paddler/definitely/missing/paddler";

#[tokio::test(flavor = "multi_thread")]
async fn pausable_agent_reports_a_binary_that_cannot_be_spawned() {
    let mut cluster = start_subprocess_cluster(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        ClusterParams {
            agents: Vec::new(),
            wait_for_slots_ready: false,
            ..ClusterParams::default()
        },
    )
    .await
    .expect("a cluster without agents must start");

    let join_error = PausableAgent::join(
        &mut cluster,
        PausableAgentParams {
            binary_path: MISSING_BINARY_PATH.to_owned(),
            config: AgentConfig::single(1),
            expected_slots_total: 1,
        },
    )
    .await
    .err()
    .expect("a missing binary must not join the cluster");

    assert_eq!(
        join_error
            .root_cause()
            .downcast_ref::<io::Error>()
            .map(io::Error::kind),
        Some(ErrorKind::NotFound)
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
