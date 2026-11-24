#![cfg(feature = "tests_that_use_llms")]

use paddler_balancer_desired_state_fixture::desired_state_with_multimodal_projection::desired_state_with_multimodal_projection;
use paddler_cluster_harness::agent_desired_model_at::agent_desired_model_at;
use paddler_cluster_harness::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;
use paddler_file_fixture::fixture_path::fixture_path;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_model_card_fixture::model_card::ModelCard;
use paddler_model_card_fixture::qwen3_0_6b::qwen3_0_6b;
use paddler_model_source::model_source::ModelSource;

#[tokio::test(flavor = "multi_thread")]
async fn a_text_generation_pipeline_reports_an_invalid_multimodal_projection_as_unloadable() {
    let ModelCard { reference } = qwen3_0_6b();
    let projection_path = fixture_path("invalid_mmproj.gguf").display().to_string();
    let mut cluster =
        start_single_agent_cluster_with_desired_state(desired_state_with_multimodal_projection(
            BalancerDesiredState {
                model: ModelSource::HuggingFace(reference).into_agent_desired_model(),
                ..BalancerDesiredState::default()
            },
            agent_desired_model_at(&fixture_path("invalid_mmproj.gguf"))
                .expect("the invalid projection fixture path must be absolute"),
        ))
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
