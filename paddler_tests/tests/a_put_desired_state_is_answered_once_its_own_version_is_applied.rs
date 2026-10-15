use reqwest::StatusCode;
use serde_json::json;
use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_tests::cluster_without_agents_serving::cluster_without_agents_serving;
use paddler_tests::start_cluster::start_cluster;

const DECISION_MODEL_URI: &str = "agent:///decision-model.gguf";

#[tokio::test(flavor = "multi_thread")]
async fn a_put_desired_state_is_answered_once_its_own_version_is_applied() {
    let cluster = start_cluster(cluster_without_agents_serving(InferenceMode::Embeddings))
        .await
        .expect("a balancer serving embeddings must start");

    cluster
        .client_management
        .put_balancer_desired_state(
            CancellationToken::new(),
            &BalancerDesiredState {
                inference_mode: InferenceMode::Decision,
                model: AgentDesiredModel::Uri(DECISION_MODEL_URI.to_owned()),
                ..BalancerDesiredState::default()
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
        (
            StatusCode::OK,
            json!({"models": [{
                "description": format!("Decision model {DECISION_MODEL_URI} served by Paddler"),
                "name": DECISION_MODEL_URI,
                "release_date": "unknown",
            }]})
        )
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
