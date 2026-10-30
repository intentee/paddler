#![cfg(feature = "tests_that_use_llms")]

use serde_json::json;

use paddler_test_cluster_harness::system_one_request::system_one_request;
use paddler_test_cluster_harness::typesafe_api_response::TypeSafeApiResponse;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::start_decision_cluster::start_decision_cluster;

pub const TYPESAFE_TONE_AND_PAYMENT_REQUEST_ID: &str = "typesafe-request";

pub async fn typesafe_tone_and_payment_decision() -> TypeSafeApiResponse {
    let cluster = start_decision_cluster(DecisionClusterParams::default())
        .await
        .expect("the cluster must start");
    let typesafe_decision = cluster
        .typesafe_system_one(
            TYPESAFE_TONE_AND_PAYMENT_REQUEST_ID,
            &system_one_request(
                &json!("The invoice was paid on time."),
                &json!({
                    "tone": {"type": "choice", "criteria": {"calm": null, "angry": "Shouting"}},
                    "paid": {"type": "noul", "instructions": "Was it paid?"},
                }),
            ),
        )
        .await
        .expect("the System One request must be answered");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");

    typesafe_decision
}
