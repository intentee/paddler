use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_tests::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_model_file_does_not_exist() {
    let model_path_on_agent = "/nonexistent/model.gguf".to_owned();
    let mut cluster = start_single_agent_cluster_with_desired_state(BalancerDesiredState {
        model: AgentDesiredModel::LocalToAgent(model_path_on_agent.clone()),
        ..BalancerDesiredState::unconfigured(InferenceMode::TextGeneration)
    })
    .await
    .expect("a single-agent cluster must start");

    cluster
        .wait_for_first_agent_issue(|issue| {
            matches!(issue, AgentIssue::ModelFileDoesNotExist(model_path) if model_path.model_path == model_path_on_agent)
        })
        .await
        .expect("the agent must report ModelFileDoesNotExist for the configured path");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
