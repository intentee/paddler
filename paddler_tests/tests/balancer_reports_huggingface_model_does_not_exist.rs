#![cfg(feature = "tests_that_use_llms")]

use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::huggingface_model_reference::HuggingFaceModelReference;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_tests::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_huggingface_model_does_not_exist() {
    let mut cluster = start_single_agent_cluster_with_desired_state(BalancerDesiredState {
        model: AgentDesiredModel::HuggingFace(HuggingFaceModelReference {
            filename: "nonexistent.gguf".to_owned(),
            repo_id: "nonexistent-org/nonexistent-model-gguf".to_owned(),
            revision: "main".to_owned(),
        }),
        ..BalancerDesiredState::unconfigured(InferenceMode::TextGeneration)
    })
    .await
    .expect("a single-agent cluster must start");

    cluster
        .wait_for_first_agent_issue(|issue| {
            matches!(
                issue,
                AgentIssue::HuggingFaceModelDoesNotExist(_) | AgentIssue::HuggingFacePermissions(_)
            )
        })
        .await
        .expect("the agent must report a Hugging Face lookup issue for a nonexistent repository");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
