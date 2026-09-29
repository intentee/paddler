#![cfg(feature = "tests_that_use_llms")]

use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_test_cluster_harness::model_card::ModelCard;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_chat_template_does_not_compile_for_invalid_jinja() {
    let ModelCard { reference, .. } = qwen3_0_6b();
    let mut cluster = start_single_agent_cluster_with_desired_state(BalancerDesiredState {
        chat_template_override: Some(ChatTemplate {
            content: "{{invalid jinja template".to_owned(),
        }),
        model: AgentDesiredModel::HuggingFace(reference),
        use_chat_template_override: true,
        ..BalancerDesiredState::default()
    })
    .await
    .expect("a single-agent cluster must start");

    cluster
        .wait_for_first_agent_issue(|issue| {
            matches!(issue, AgentIssue::ChatTemplateDoesNotCompile(_))
        })
        .await
        .expect("the agent must report ChatTemplateDoesNotCompile for invalid Jinja syntax");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
