#![cfg(feature = "tests_that_use_llms")]

use std::mem::discriminant;

use paddler_cluster_harness::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_model_card_fixture::nomic_embed_text_v1_5::nomic_embed_text_v1_5;
use paddler_model_source::model_source::ModelSource;

#[tokio::test(flavor = "multi_thread")]
async fn a_model_without_a_chat_template_is_reported_as_unable_to_find_one() {
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
    let issues = &snapshot
        .agents
        .first()
        .expect("the snapshot must hold the agent")
        .status
        .issues;

    assert!(issues.contains(&expected_issue), "{issues:?}");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
