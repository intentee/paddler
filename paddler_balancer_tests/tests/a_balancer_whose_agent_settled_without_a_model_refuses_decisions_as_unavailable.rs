use indexmap::IndexMap;
use llama_cpp_bindings_types::decision_question::DecisionQuestion;
use llama_cpp_bindings_types::decision_request::DecisionRequest;
use serde_json::Value;
use serde_json::json;
use tokio_util::sync::CancellationToken;

use paddler_client::error::Error as ClientError;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_params::ClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn a_balancer_whose_agent_settled_without_a_model_refuses_decisions_as_unavailable() {
    let mut cluster = Cluster::start_in_process(ClusterParams {
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a cluster whose agent serves no model must start");

    cluster
        .wait_for_first_agent_to_apply_its_state()
        .await
        .expect("the agent must apply its state without a model");

    let decision_rejection = cluster
        .client_inference
        .post_decide(
            CancellationToken::new(),
            &DecisionRequest {
                images: Vec::new(),
                questions: IndexMap::from([(
                    "paid".to_owned(),
                    DecisionQuestion::Choice {
                        criteria: IndexMap::from([
                            ("no".to_owned(), Value::Null),
                            ("yes".to_owned(), Value::Null),
                        ]),
                        instructions: json!("Was it paid?"),
                    },
                )]),
                state: json!("The invoice was paid on time."),
            },
        )
        .await
        .err()
        .expect("a cluster whose agent settled without a model must refuse a decision");

    assert!(matches!(
        decision_rejection,
        ClientError::ServiceUnavailable { .. }
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
