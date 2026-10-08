use reqwest::Client;
use reqwest::StatusCode;
use serde_json::json;

use paddler_balancer::compatibility::typesafe_service::typesafe_api_path::TypeSafeApiPath;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_tests::cluster_without_agents_serving::cluster_without_agents_serving;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_rejects_malformed_requests_as_unprocessable() {
    let cluster = start_cluster(cluster_without_agents_serving(InferenceMode::Decision))
        .await
        .expect("a decision balancer without agents must start");

    let without_questions = cluster
        .typesafe_system_one(
            "malformed-request",
            &json!({"state": "state", "questions": {}}),
        )
        .await
        .expect("the request must be answered");

    assert_eq!(without_questions.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        without_questions.body,
        json!({"detail": "a System One request needs at least one question"})
    );

    let not_json = Client::new()
        .post(
            cluster
                .balancer
                .compat_typesafe_base_url()
                .expect("the cluster serves TypeSafe")
                .join(TypeSafeApiPath::SYSTEM_ONE)
                .expect("the path must join"),
        )
        .header("content-type", "application/json")
        .body("not json")
        .send()
        .await
        .expect("the request must be answered");

    assert_eq!(not_json.status(), StatusCode::UNPROCESSABLE_ENTITY);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
