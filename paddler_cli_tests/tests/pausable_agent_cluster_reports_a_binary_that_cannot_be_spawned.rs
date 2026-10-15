use std::io;
use std::io::ErrorKind;

use paddler_cli_tests::missing_binary_path::MISSING_BINARY_PATH;
use paddler_cli_tests::pausable_agent_cluster_params::PausableAgentClusterParams;
use paddler_cli_tests::start_pausable_agent_cluster::start_pausable_agent_cluster;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;

#[tokio::test(flavor = "multi_thread")]
async fn pausable_agent_cluster_reports_a_binary_that_cannot_be_spawned() {
    let start_error = start_pausable_agent_cluster(PausableAgentClusterParams {
        binary_path: MISSING_BINARY_PATH.to_owned(),
        desired_state: ClusterDesiredState::KeepStored,
        expected_slots_total: 1,
        slot_count: 1,
    })
    .await
    .err()
    .expect("a missing binary must not start a cluster");

    assert_eq!(
        start_error
            .root_cause()
            .downcast_ref::<io::Error>()
            .map(io::Error::kind),
        Some(ErrorKind::NotFound)
    );
}
