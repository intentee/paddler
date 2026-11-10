#![cfg(feature = "tests_that_use_llms")]

use reqwest::StatusCode;
use serde_json::Value;
use serde_json::json;

use paddler_test_cluster_harness::system_one_request::system_one_request;
use paddler_tests::decision_cluster_params::DecisionClusterParams;

const ROUNDED_PROBABILITY_SUM_TOLERANCE: f64 = 0.002;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_answers_every_question_type() {
    let cluster = DecisionClusterParams::default()
        .start_cluster()
        .await
        .expect("the cluster must start");

    let response = cluster
        .typesafe_system_one(
            "model-request",
            &system_one_request(
                &json!({"customer": "Ada", "message": "I was charged twice. Please help."}),
                &json!({
                    "billing": {"type": "noul", "instructions": "Is this about billing?"},
                    "tone": {
                        "type": "choice",
                        "instructions": "What is the tone?",
                        "criteria": {"calm": null, "angry": null},
                    },
                    "urgency": {
                        "type": "score",
                        "criteria": ["Can wait", "Needs attention today"],
                    },
                }),
            ),
        )
        .await
        .expect("the System One request must be answered");

    assert_eq!(response.status, StatusCode::OK);

    let body = &response.body;

    assert!(
        body.pointer("/answers/billing/noul")
            .and_then(Value::as_f64)
            .is_some_and(|noul| (0.0..=1.0).contains(&noul))
    );
    assert!(
        ["calm", "angry"].contains(
            &body
                .pointer("/answers/tone/choice")
                .and_then(Value::as_str)
                .unwrap_or_default()
        )
    );
    assert!(
        (body
            .pointer("/answers/urgency/probabilities")
            .and_then(Value::as_object)
            .expect("the score probabilities must be an object")
            .values()
            .filter_map(Value::as_f64)
            .sum::<f64>()
            - 1.0)
            .abs()
            < ROUNDED_PROBABILITY_SUM_TOLERANCE
    );
    assert!(
        body.pointer("/usage/input_tokens")
            .and_then(Value::as_u64)
            .is_some_and(|input_tokens| input_tokens > 0)
    );
    assert_eq!(body.pointer("/usage/output_tokens"), Some(&json!(0)));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
