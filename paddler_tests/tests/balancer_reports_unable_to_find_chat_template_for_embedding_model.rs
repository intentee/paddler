#![cfg(feature = "tests_that_use_llms")]

use std::mem::discriminant;

use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_model_source::model_source::ModelSource;
use paddler_test_cluster_harness::model_card::nomic_embed_text_v1_5::nomic_embed_text_v1_5;
use paddler_tests::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_unable_to_find_chat_template_for_embedding_model() {
    let model_card = nomic_embed_text_v1_5();
    let expected_issue = AgentIssue::UnableToFindChatTemplate(ModelPath::from(
        model_card
            .cached_path()
            .expect("the model must be in the Hugging Face cache")
            .as_path(),
    ));
    let mut cluster = start_single_agent_cluster_with_desired_state(BalancerDesiredState {
        model: ModelSource::HuggingFace(model_card.reference).into_agent_desired_model(),
        ..BalancerDesiredState::default()
    })
    .await
    .expect("a single-agent cluster must start");

    let snapshot = cluster
        .wait_for_first_agent_issue(|issue| discriminant(issue) == discriminant(&expected_issue))
        .await
        .expect("the agent must report UnableToFindChatTemplate for an embedding-only model");
    let issues = &snapshot.agents[0].status.issues;

    assert!(issues.contains(&expected_issue), "{issues:?}");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
