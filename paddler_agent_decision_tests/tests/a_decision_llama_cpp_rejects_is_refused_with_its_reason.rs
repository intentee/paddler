#![cfg(feature = "tests_that_use_llms")]

use indexmap::IndexMap;
use llama_cpp_bindings_types::decision_question::DecisionQuestion;
use llama_cpp_bindings_types::decision_request::DecisionRequest;
use serde_json::Value;
use serde_json::json;
use tokio_util::sync::CancellationToken;

use paddler_cluster_harness::decision_cluster_params::DecisionClusterParams;
use paddler_messaging::decision_result::DecisionResult;

#[tokio::test(flavor = "multi_thread")]
async fn a_decision_llama_cpp_rejects_is_refused_with_its_reason() {
    let cluster = DecisionClusterParams::default()
        .start_cluster()
        .await
        .expect("the cluster must start");

    let collected = cluster
        .decide(
            CancellationToken::new(),
            &DecisionRequest {
                images: Vec::new(),
                questions: IndexMap::from([(
                    "paid".to_owned(),
                    DecisionQuestion::Noul {
                        criteria: IndexMap::new(),
                        instructions: Value::Null,
                    },
                )]),
                state: json!("Ada paid the invoice."),
            },
        )
        .await
        .expect("the decision must be answered with a refusal");

    assert_eq!(collected.answers, Vec::new());
    assert!(matches!(
        collected.terminal_result,
        DecisionResult::RequestRejected(_)
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
