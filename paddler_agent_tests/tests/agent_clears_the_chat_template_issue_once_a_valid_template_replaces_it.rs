#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_balancer_desired_state_fixture::desired_state_with_chat_template_override::desired_state_with_chat_template_override;
use paddler_balancer_desired_state_fixture::uncompilable_chat_template::uncompilable_chat_template;
use paddler_balancer_desired_state_fixture::uncompilable_chat_template_issue::UncompilableChatTemplateIssue;
use paddler_cluster_harness::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;
use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler_messaging::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_model_card_fixture::qwen3_0_6b::qwen3_0_6b;
use paddler_model_source::model_source::ModelSource;

#[tokio::test(flavor = "multi_thread")]
async fn agent_clears_the_chat_template_issue_once_a_valid_template_replaces_it() {
    let model_card = qwen3_0_6b();
    let uncompilable_chat_template_issue = UncompilableChatTemplateIssue {
        model_path: ModelPath::from(
            model_card
                .cached_path()
                .expect("the model must be in the Hugging Face cache")
                .as_path(),
        ),
    };
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

    cluster
        .wait_for_first_agent_issue(|issue| uncompilable_chat_template_issue.is_reported_by(issue))
        .await
        .expect("the agent must report ChatTemplateDoesNotCompile for invalid Jinja syntax");

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
        .until_agent(&agent_id, |AgentControllerPoolSnapshot { agents }| {
            agents
                .iter()
                .any(|AgentControllerSnapshot { id, status, .. }| {
                    *id == agent_id
                        && !status
                            .issues
                            .iter()
                            .any(|issue| uncompilable_chat_template_issue.is_reported_by(issue))
                })
        })
        .await
        .expect("the agent must stay registered while reconciliation clears its template issue");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
