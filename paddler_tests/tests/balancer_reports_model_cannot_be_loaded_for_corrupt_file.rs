use std::io::Write as _;

use tempfile::NamedTempFile;

use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_tests::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_model_cannot_be_loaded_for_corrupt_file() {
    let mut corrupt_model = NamedTempFile::new().expect("a temporary model file must be creatable");

    corrupt_model
        .write_all(b"this is not a valid gguf model file")
        .expect("the corrupt model contents must be writable");

    let corrupt_model_path = corrupt_model
        .path()
        .to_str()
        .expect("the temporary model path must be valid UTF-8")
        .to_owned();
    let mut cluster = start_single_agent_cluster_with_desired_state(BalancerDesiredState {
        model: AgentDesiredModel::LocalToAgent(corrupt_model_path.clone()),
        ..BalancerDesiredState::default()
    })
    .await
    .expect("a single-agent cluster must start");

    cluster
        .wait_for_first_agent_issue(|issue| {
            matches!(issue, AgentIssue::ModelCannotBeLoaded(model_path) if model_path.model_path == corrupt_model_path)
        })
        .await
        .expect("the agent must report ModelCannotBeLoaded for the corrupt file");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
