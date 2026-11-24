use reqwest::StatusCode;

use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_cluster_harness::noul_system_one_request::noul_system_one_request;
use paddler_tests::cluster_with_an_agent_downloading_its_model::ClusterWithAnAgentDownloadingItsModel;
use paddler_tests::constants::SHORT_BUFFERED_REQUEST_TIMEOUT;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_reports_a_buffered_request_timeout_as_a_gateway_timeout() {
    let cluster_with_a_downloading_agent =
        ClusterWithAnAgentDownloadingItsModel::start(ClusterParams {
            buffered_request_timeout: SHORT_BUFFERED_REQUEST_TIMEOUT,
            ..ClusterParams::default()
        })
        .await
        .expect("a cluster whose agent downloads its model must start");
    let cluster = &cluster_with_a_downloading_agent.cluster;

    assert_eq!(
        cluster
            .typesafe_system_one("waiting-request", &noul_system_one_request())
            .await
            .expect("the request must be answered")
            .status,
        StatusCode::GATEWAY_TIMEOUT
    );

    cluster_with_a_downloading_agent
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
