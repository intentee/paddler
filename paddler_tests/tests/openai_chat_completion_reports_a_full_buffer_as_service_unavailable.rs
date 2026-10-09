use reqwest::StatusCode;

use paddler_messaging::inference_mode::InferenceMode;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::cluster_without_agents_serving::cluster_without_agents_serving;
use paddler_tests::openai_chat_completion_failure_status::openai_chat_completion_failure_status;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn openai_chat_completion_reports_a_full_buffer_as_service_unavailable() {
    let cluster = start_cluster(ClusterParams {
        max_buffered_requests: 0,
        ..cluster_without_agents_serving(InferenceMode::TextGeneration)
    })
    .await
    .expect("a balancer without agents must start");

    assert_eq!(
        openai_chat_completion_failure_status(&cluster)
            .await
            .expect("the chat completion must fail with an OpenAI error body"),
        StatusCode::SERVICE_UNAVAILABLE
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
