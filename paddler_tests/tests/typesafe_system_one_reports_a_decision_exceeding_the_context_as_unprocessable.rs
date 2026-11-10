#![cfg(feature = "tests_that_use_llms")]

use reqwest::StatusCode;
use serde_json::json;

use paddler_messaging::constants::DECISION_SLOTS_MINIMUM;
use paddler_test_cluster_harness::noul_system_one_request_about::noul_system_one_request_about;
use paddler_tests::constants::SMALL_DECISION_CONTEXT_SIZE;
use paddler_tests::oversized_text::oversized_text;
use paddler_tests::small_context_decision_cluster_params::small_context_decision_cluster_params;

const TOKENS_REQUIRED_BY_THE_OVERSIZED_STATE_DECISION: usize = 410;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_reports_a_decision_exceeding_the_context_as_unprocessable() {
    let cluster = small_context_decision_cluster_params()
        .start_cluster()
        .await
        .expect("the cluster must start");

    let response = cluster
        .typesafe_system_one(
            "oversized-decision",
            &noul_system_one_request_about(&json!(oversized_text())),
        )
        .await
        .expect("the request must be answered");

    assert_eq!(
        (response.status, response.body),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({
                "detail": format!(
                    "the request needs {TOKENS_REQUIRED_BY_THE_OVERSIZED_STATE_DECISION} tokens in the cache, more than the {} the context holds",
                    SMALL_DECISION_CONTEXT_SIZE.get() / u32::from(DECISION_SLOTS_MINIMUM.get())
                )
            })
        )
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
