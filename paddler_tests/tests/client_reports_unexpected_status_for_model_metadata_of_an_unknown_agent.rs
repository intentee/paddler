use paddler_client::error::Error;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;
use reqwest::StatusCode;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn client_reports_unexpected_status_for_model_metadata_of_an_unknown_agent() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a cluster without agents must start");

    let rejection = cluster
        .client_management
        .get_model_metadata(CancellationToken::new(), "unknown-agent")
        .await
        .err()
        .expect("the balancer must reject a request about an agent it does not know");

    assert!(matches!(
        rejection,
        Error::UnexpectedResponseStatus { status, .. } if status == StatusCode::NOT_FOUND
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
