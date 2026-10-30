#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use std::mem::discriminant;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_model_source::model_source::ModelSource;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::desired_state_with_chat_template_override::desired_state_with_chat_template_override;
use paddler_tests::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;
use paddler_tests::uncompilable_chat_template::uncompilable_chat_template;
use paddler_tests::uncompilable_chat_template_issue::uncompilable_chat_template_issue;

#[tokio::test(flavor = "multi_thread")]
async fn agent_clears_the_chat_template_issue_once_a_valid_template_replaces_it() {
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
                model: ModelSource::HuggingFace(model_card.reference.clone())
                    .into_agent_desired_model(),
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
        .client_management
        .put_balancer_desired_state(
            CancellationToken::new(),
            &desired_state_with_chat_template_override(
                BalancerDesiredState {
                    model: ModelSource::HuggingFace(model_card.reference)
                        .into_agent_desired_model(),
                    ..BalancerDesiredState::default()
                },
                ChatTemplate {
                    content: "{% for message in messages %}{{ message.content }}{% endfor %}"
                        .to_owned(),
                },
            ),
        )
        .await
        .expect("the balancer must accept the desired state with a valid template");

    let agent_id = cluster
        .agent_ids
        .first()
        .expect("the cluster must have a registered agent")
        .clone();

    cluster
        .agents_watcher
        .until_agent(&agent_id, |snapshot| {
            snapshot.agents.iter().any(|agent| {
                agent.id == agent_id
                    && !agent
                        .status
                        .issues
                        .iter()
                        .any(|issue| discriminant(issue) == discriminant(&expected_issue))
            })
        })
        .await
        .expect("the agent must stay registered while reconciliation clears its template issue");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
