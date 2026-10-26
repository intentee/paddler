#![cfg(feature = "tests_that_use_llms")]

use std::mem::discriminant;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_model_source::model_source::ModelSource;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::desired_state_with_chat_template_override::desired_state_with_chat_template_override;
use paddler_tests::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;
use paddler_tests::uncompilable_chat_template::uncompilable_chat_template;
use paddler_tests::uncompilable_chat_template_issue::uncompilable_chat_template_issue;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_chat_template_does_not_compile_for_invalid_jinja() {
    let model_card = qwen3_0_6b();
    let expected_issue = uncompilable_chat_template_issue(
        &model_card
            .cached_path()
            .expect("the model must be in the Hugging Face cache"),
    )
    .expect("the uncompilable chat template must not compile");
    let mut cluster =
        start_single_agent_cluster_with_desired_state(desired_state_with_chat_template_override(
            BalancerDesiredState {
                model: ModelSource::HuggingFace(model_card.reference).into_agent_desired_model(),
                ..BalancerDesiredState::default()
            },
            uncompilable_chat_template(),
        ))
        .await
        .expect("a single-agent cluster must start");

    let snapshot = cluster
        .wait_for_first_agent_issue(|issue| discriminant(issue) == discriminant(&expected_issue))
        .await
        .expect("the agent must report ChatTemplateDoesNotCompile for invalid Jinja syntax");
    let issues = &snapshot.agents[0].status.issues;

    assert!(issues.contains(&expected_issue), "{issues:?}");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
