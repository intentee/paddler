#![cfg(feature = "tests_that_use_llms")]

use paddler_balancer_desired_state_fixture::desired_state_with_chat_template_override::desired_state_with_chat_template_override;
use paddler_balancer_desired_state_fixture::uncompilable_chat_template::uncompilable_chat_template;
use paddler_balancer_desired_state_fixture::uncompilable_chat_template_issue::UncompilableChatTemplateIssue;
use paddler_cluster_harness::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_model_card_fixture::qwen3_0_6b::qwen3_0_6b;
use paddler_model_source::model_source::ModelSource;

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_reports_an_override_chat_template_that_does_not_compile() {
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
                model: ModelSource::HuggingFace(model_card.reference).into_agent_desired_model(),
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
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
