use std::time::Duration;

use reqwest::StatusCode;

use paddler_messaging::inference_mode::InferenceMode;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::noul_system_one_request::noul_system_one_request;
use paddler_tests::cluster_without_agents_serving::cluster_without_agents_serving;
use paddler_tests::start_cluster::start_cluster;

const SHORT_BUFFERED_REQUEST_TIMEOUT: Duration = Duration::from_millis(50);

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_reports_a_buffered_request_timeout_as_a_gateway_timeout() {
    let cluster = start_cluster(ClusterParams {
        buffered_request_timeout: SHORT_BUFFERED_REQUEST_TIMEOUT,
        ..cluster_without_agents_serving(InferenceMode::Decision)
    })
    .await
    .expect("a decision balancer without agents must start");

    assert_eq!(
        cluster
            .typesafe_system_one("waiting-request", &noul_system_one_request())
            .await
            .expect("the request must be answered")
            .status,
        StatusCode::GATEWAY_TIMEOUT
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
