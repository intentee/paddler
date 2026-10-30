use paddler_state_database::state_database_error::StateDatabaseError;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn start_cluster_reports_a_state_database_url_it_cannot_parse() {
    let start_error = start_cluster(ClusterParams {
        state_database_url: "unsupported-scheme://state".to_owned(),
        ..ClusterParams::without_agents()
    })
    .await
    .err()
    .expect("a state database URL with an unsupported scheme must fail the cluster start");

    assert!(matches!(
        start_error.downcast_ref::<StateDatabaseError>(),
        Some(StateDatabaseError::SchemeUnsupported { scheme }) if scheme == "unsupported-scheme"
    ));
}
