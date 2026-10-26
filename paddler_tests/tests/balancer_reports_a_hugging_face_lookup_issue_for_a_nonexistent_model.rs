#![cfg(feature = "tests_that_use_llms")]

use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;
use paddler_model_source::model_source::ModelSource;
use paddler_tests::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_a_hugging_face_lookup_issue_for_a_nonexistent_model() {
    let reference = HuggingFaceModelReference {
        filename: "nonexistent.gguf".to_owned(),
        repository_name: "nonexistent-model-gguf".to_owned(),
        repository_owner: "nonexistent-org".to_owned(),
        revision: "main".to_owned(),
    };
    let lookup_issues = [
        AgentIssue::HuggingFaceModelDoesNotExist(reference.model_path()),
        AgentIssue::HuggingFacePermissions(reference.model_path()),
    ];
    let mut cluster = start_single_agent_cluster_with_desired_state(BalancerDesiredState {
        model: ModelSource::HuggingFace(reference).into_agent_desired_model(),
        ..BalancerDesiredState::default()
    })
    .await
    .expect("a single-agent cluster must start");

    cluster
        .wait_for_first_agent_issue(|issue| lookup_issues.contains(issue))
        .await
        .expect("the agent must report a Hugging Face lookup issue for a nonexistent repository");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
