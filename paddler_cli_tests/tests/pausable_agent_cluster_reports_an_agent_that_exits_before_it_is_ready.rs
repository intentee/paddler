use paddler_cli_tests::agent_exited_before_ready_with_code::agent_exited_before_ready_with_code;
use paddler_cli_tests::clap_usage_error_exit_code::CLAP_USAGE_ERROR_EXIT_CODE;
use paddler_cli_tests::pausable_agent_cluster_params::PausableAgentClusterParams;
use paddler_cli_tests::start_pausable_agent_cluster::start_pausable_agent_cluster;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;

const SLOTS_THE_AGENT_REJECTS: u16 = 0;

#[tokio::test(flavor = "multi_thread")]
async fn pausable_agent_cluster_reports_an_agent_that_exits_before_it_is_ready() {
    let start_error = start_pausable_agent_cluster(PausableAgentClusterParams {
        binary_path: env!("CARGO_BIN_EXE_paddler_cluster_node").to_owned(),
        desired_state: ClusterDesiredState::KeepStored,
        expected_slots_total: SLOTS_THE_AGENT_REJECTS,
        slot_count: SLOTS_THE_AGENT_REJECTS,
    })
    .await
    .err()
    .expect("an agent that exits must fail the cluster start");

    assert!(agent_exited_before_ready_with_code(
        &start_error,
        CLAP_USAGE_ERROR_EXIT_CODE
    ));
}
