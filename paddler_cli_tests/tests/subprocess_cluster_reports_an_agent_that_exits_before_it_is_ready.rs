use paddler_cli_tests::agent_exited_before_ready_with_code::agent_exited_before_ready_with_code;
use paddler_cli_tests::clap_usage_error_exit_code::CLAP_USAGE_ERROR_EXIT_CODE;
use paddler_cli_tests::start_subprocess_cluster::start_subprocess_cluster;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

const SLOTS_THE_AGENT_REJECTS: u16 = 0;

#[tokio::test(flavor = "multi_thread")]
async fn subprocess_cluster_reports_an_agent_that_exits_before_it_is_ready() {
    let start_error = start_subprocess_cluster(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        ClusterParams {
            agents: vec![AgentConfig::single(SLOTS_THE_AGENT_REJECTS)],
            wait_for_slots_ready: false,
            ..ClusterParams::default()
        },
    )
    .await
    .err()
    .expect("an agent that exits must fail the cluster start");

    assert!(agent_exited_before_ready_with_code(
        &start_error,
        CLAP_USAGE_ERROR_EXIT_CODE
    ));
}
