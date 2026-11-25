use reqwest::StatusCode;

use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_cluster_harness::cluster_with_an_agent_downloading_its_model::ClusterWithAnAgentDownloadingItsModel;
use paddler_cluster_harness::noul_system_one_request::noul_system_one_request;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_reports_a_full_buffer_as_service_unavailable() {
    let cluster_with_a_downloading_agent =
        ClusterWithAnAgentDownloadingItsModel::start(ClusterParams {
            max_buffered_requests: 0,
            ..ClusterParams::default()
        })
        .await
        .expect("a cluster whose agent downloads its model must start");
    let cluster = &cluster_with_a_downloading_agent.cluster;

    assert_eq!(
        cluster
            .typesafe_system_one("overflowing-request", &noul_system_one_request())
            .await
            .expect("the request must be answered")
            .status,
        StatusCode::SERVICE_UNAVAILABLE
    );

    cluster_with_a_downloading_agent
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
