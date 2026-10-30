#![cfg(feature = "tests_that_use_llms")]

use reqwest::StatusCode;
use serde_json::Map;
use serde_json::Value;
use serde_json::json;

use paddler_balancer::compatibility::typesafe_service::typesafe_api_path::TypeSafeApiPath;
use paddler_messaging::decision_slots_minimum::DECISION_SLOTS_MINIMUM;
use paddler_test_cluster_harness::half_closed_client::HalfClosedClient;
use paddler_test_cluster_harness::noul_system_one_request::noul_system_one_request;
use paddler_test_cluster_harness::system_one_request::system_one_request;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::questions_outlasting_a_disconnect::QUESTIONS_OUTLASTING_A_DISCONNECT;
use paddler_tests::sample_decision::sample_decision;
use paddler_tests::start_decision_cluster::start_decision_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_client_disconnecting_mid_decision_releases_its_sequences() {
    let mut cluster = start_decision_cluster(DecisionClusterParams::default())
        .await
        .expect("the cluster must start");
    let agent_id = cluster
        .agent_ids
        .first()
        .expect("the cluster must have an agent")
        .clone();
    let questions: Map<String, Value> = (0..QUESTIONS_OUTLASTING_A_DISCONNECT)
        .map(|question_index| (format!("paid-{question_index}"), json!({"type": "noul"})))
        .collect();
    let mut half_closed_client = HalfClosedClient::post_json_then_half_close(
        cluster
            .balancer
            .compat_typesafe_addr()
            .expect("the cluster must serve the TypeSafe API"),
        TypeSafeApiPath::SYSTEM_ONE,
        &system_one_request(&json!(sample_decision().state), &json!(questions)),
    )
    .await
    .expect("the System One request must be sent");

    cluster
        .wait_for_slots_processing(&agent_id, u64::from(DECISION_SLOTS_MINIMUM.get()))
        .await
        .expect("the agent must start deciding");
    half_closed_client
        .half_close()
        .await
        .expect("the System One request must be half-closed");
    cluster
        .wait_for_slots_processing(&agent_id, 0)
        .await
        .expect("the agent must release the abandoned decision");
    drop(half_closed_client);

    let response = cluster
        .typesafe_system_one("after-the-abandoned-decision", &noul_system_one_request())
        .await
        .expect("a System One request after the abandoned one must be answered");

    assert_eq!(response.status, StatusCode::OK);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
