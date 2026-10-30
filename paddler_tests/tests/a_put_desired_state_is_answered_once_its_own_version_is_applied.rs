use reqwest::StatusCode;
use serde_json::json;
use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::desired_state_serving_decisions::desired_state_serving_decisions;
use paddler_tests::start_cluster::start_cluster;

const DECISION_MODEL_URI: &str = "agent:///decision-model.gguf";

#[tokio::test(flavor = "multi_thread")]
async fn a_put_desired_state_is_answered_once_its_own_version_is_applied() {
    let cluster = start_cluster(ClusterParams::without_agents())
        .await
        .expect("a balancer serving text generation and embeddings must start");

    cluster
        .client_management
        .put_balancer_desired_state(
            CancellationToken::new(),
            &BalancerDesiredState {
                model: AgentDesiredModel::Uri(DECISION_MODEL_URI.to_owned()),
                ..desired_state_serving_decisions()
            },
        )
        .await
        .expect("the balancer must accept the desired state");

    let response = cluster
        .typesafe_models()
        .await
        .expect("the models request must be answered");

    assert_eq!(
        (response.status, response.body),
        (StatusCode::OK, json!({"models": []}))
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
