use reqwest::StatusCode;
use serde_json::json;

use paddler_messaging::inference_mode::InferenceMode;
use paddler_tests::cluster_without_agents_serving::cluster_without_agents_serving;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_is_unavailable_while_the_cluster_serves_another_inference_mode() {
    let cluster = start_cluster(cluster_without_agents_serving(
        InferenceMode::TextGeneration,
    ))
    .await
    .expect("a balancer serving text generation must start");

    let response = cluster
        .typesafe_system_one(
            "text-generation-cluster-request",
            &json!({"state": "state", "questions": {"paid": {"type": "noul"}}}),
        )
        .await
        .expect("the System One request must be answered");

    assert_eq!(response.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        response.body,
        json!({"detail": "The cluster serves TextGeneration, not Decision"})
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
