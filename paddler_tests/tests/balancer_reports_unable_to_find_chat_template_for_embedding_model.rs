#![cfg(feature = "tests_that_use_llms")]

use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_model_source::model_source::ModelSource;
use paddler_test_cluster_harness::model_card::ModelCard;
use paddler_test_cluster_harness::model_card::nomic_embed_text_v1_5::nomic_embed_text_v1_5;
use paddler_tests::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_unable_to_find_chat_template_for_embedding_model() {
    let ModelCard { reference } = nomic_embed_text_v1_5();
    let mut cluster = start_single_agent_cluster_with_desired_state(BalancerDesiredState {
        model: ModelSource::HuggingFace(reference).into_agent_desired_model(),
        ..BalancerDesiredState::default()
    })
    .await
    .expect("a single-agent cluster must start");

    cluster
        .wait_for_first_agent_issue(|issue| {
            matches!(issue, AgentIssue::UnableToFindChatTemplate(_))
        })
        .await
        .expect("the agent must report UnableToFindChatTemplate for an embedding-only model");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
