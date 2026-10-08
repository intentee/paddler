use reqwest::StatusCode;

use paddler_messaging::inference_mode::InferenceMode;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::noul_system_one_request::noul_system_one_request;
use paddler_tests::cluster_without_agents_serving::cluster_without_agents_serving;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_reports_a_full_buffer_as_service_unavailable() {
    let cluster = start_cluster(ClusterParams {
        max_buffered_requests: 0,
        ..cluster_without_agents_serving(InferenceMode::Decision)
    })
    .await
    .expect("a decision balancer without agents must start");

    assert_eq!(
        cluster
            .typesafe_system_one("overflowing-request", &noul_system_one_request())
            .await
            .expect("the request must be answered")
            .status,
        StatusCode::SERVICE_UNAVAILABLE
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
