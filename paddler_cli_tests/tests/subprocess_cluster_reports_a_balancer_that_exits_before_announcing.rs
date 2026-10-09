use paddler_cli_tests::clap_usage_error_exit_code::CLAP_USAGE_ERROR_EXIT_CODE;
use paddler_cli_tests::start_subprocess_cluster::start_subprocess_cluster;
use paddler_cli_tests::subprocess_cluster_error::SubprocessClusterError;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn subprocess_cluster_reports_a_balancer_that_exits_before_announcing() {
    let start_error = start_subprocess_cluster(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        ClusterParams {
            agents: Vec::new(),
            state_database_url: "unsupported-scheme://state".to_owned(),
            wait_for_slots_ready: false,
            ..ClusterParams::default()
        },
    )
    .await
    .err()
    .expect("a balancer that rejects its arguments must fail the cluster start");

    assert!(matches!(
        start_error.downcast_ref::<SubprocessClusterError>(),
        Some(SubprocessClusterError::BalancerExitedBeforeAnnouncing { exit_status })
            if exit_status.code() == Some(CLAP_USAGE_ERROR_EXIT_CODE)
    ));
}
