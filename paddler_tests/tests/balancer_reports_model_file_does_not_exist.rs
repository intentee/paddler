use std::path::Path;

use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_test_cluster_harness::agent_desired_model_at::agent_desired_model_at;
use paddler_tests::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_model_file_does_not_exist() {
    let model_path_on_agent = "/nonexistent/model.gguf".to_owned();
    let mut cluster = start_single_agent_cluster_with_desired_state(BalancerDesiredState {
        model: agent_desired_model_at(Path::new(&model_path_on_agent))
            .expect("the missing model path must be absolute"),
        ..BalancerDesiredState::default()
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
