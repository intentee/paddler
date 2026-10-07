#![cfg(feature = "tests_that_use_llms")]

use reqwest::StatusCode;
use serde_json::Value;
use serde_json::json;

use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::start_decision_cluster::start_decision_cluster;

const ROUNDED_PROBABILITY_SUM_TOLERANCE: f64 = 0.002;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_answers_with_the_decision_model() {
    let cluster = start_decision_cluster(DecisionClusterParams::default())
        .await
        .expect("the cluster must start");

    let response = cluster
        .typesafe_system_one(
            "model-request",
            &json!({
                "model": "kev-latest",
                "state": {"customer": "Ada", "message": "I was charged twice. Please help."},
                "questions": {
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
                },
            }),
        )
        .await
        .expect("the System One request must be answered");

    assert_eq!(response.status, StatusCode::OK);

    let body = &response.body;
    let answers = &body["answers"];

    assert!(
        answers["billing"]["noul"]
            .as_f64()
            .is_some_and(|noul| (0.0..=1.0).contains(&noul))
    );
    assert!(["calm", "angry"].contains(&answers["tone"]["choice"].as_str().unwrap_or_default()));
    assert!(
        (answers["urgency"]["probabilities"]
            .as_object()
            .expect("the score probabilities must be an object")
            .values()
            .filter_map(Value::as_f64)
            .sum::<f64>()
            - 1.0)
            .abs()
            < ROUNDED_PROBABILITY_SUM_TOLERANCE
    );
    assert!(
        body["usage"]["input_tokens"]
            .as_u64()
            .is_some_and(|input_tokens| input_tokens > 0)
    );
    assert_eq!(body["usage"]["output_tokens"], json!(0));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
