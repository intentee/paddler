#![cfg(feature = "tests_that_use_llms")]

use reqwest::StatusCode;
use serde_json::Value;
use serde_json::json;

use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler_messaging::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_tests::decision_cluster_params::DecisionClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_models_lists_the_model_the_cluster_serves() {
    let mut cluster = DecisionClusterParams::default()
        .start_cluster()
        .await
        .expect("the decision cluster must start");
    let served_models = cluster
        .agents_watcher
        .until(|AgentControllerPoolSnapshot { agents }| {
            agents
                .iter()
                .all(|AgentControllerSnapshot { status, .. }| status.model_path.is_some())
        })
        .await
        .expect("every agent must report the model it serves")
        .agents
        .into_iter()
        .filter_map(|AgentControllerSnapshot { status, .. }| status.model_path)
        .map(|model_path| {
            json!({
                "name": model_path,
                "description": format!("Decision model {model_path} served by Paddler"),
                "release_date": "unknown",
            })
        })
        .collect::<Vec<Value>>();
    let response = cluster
        .typesafe_models()
        .await
        .expect("the models request must be answered");

    assert_eq!(
        (response.status, response.body),
        (StatusCode::OK, json!({ "models": served_models }))
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
