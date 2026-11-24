#![cfg(feature = "tests_that_use_llms")]

use reqwest::StatusCode;
use serde_json::json;

use paddler_test_cluster_harness::noul_system_one_request_about::noul_system_one_request_about;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::with_nul_byte::with_nul_byte;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_reports_a_state_with_a_nul_byte_as_unprocessable() {
    let cluster = DecisionClusterParams::default()
        .start_cluster()
        .await
        .expect("the cluster must start");
    let state_with_nul_byte = with_nul_byte("state");

    let response = cluster
        .typesafe_system_one(
            "state-with-a-nul-byte",
            &noul_system_one_request_about(&json!(state_with_nul_byte)),
        )
        .await
        .expect("the request must be answered");

    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
