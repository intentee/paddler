#![cfg(feature = "tests_that_use_llms")]

use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_test_cluster_harness::model_card::ModelCard;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::desired_state_with_multimodal_projection::desired_state_with_multimodal_projection;
use paddler_tests::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_multimodal_projection_cannot_be_loaded_for_invalid_file() {
    let ModelCard { reference } = qwen3_0_6b();
    let projection_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/invalid_mmproj.gguf"
    )
    .to_owned();
    let mut cluster = start_single_agent_cluster_with_desired_state(
        desired_state_with_multimodal_projection(
            BalancerDesiredState {
                model: AgentDesiredModel::HuggingFace(reference),
                ..BalancerDesiredState::unconfigured(InferenceMode::TextGeneration)
            },
            AgentDesiredModel::LocalToAgent(projection_path.clone()),
        )
        .expect("a text generation state must accept a multimodal projection"),
    )
    .await
    .expect("a single-agent cluster must start");

    cluster
        .wait_for_first_agent_issue(|issue| {
            matches!(issue, AgentIssue::MultimodalProjectionCannotBeLoaded(model_path) if model_path.model_path == projection_path)
        })
        .await
        .expect("the agent must report MultimodalProjectionCannotBeLoaded for the configured path");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
