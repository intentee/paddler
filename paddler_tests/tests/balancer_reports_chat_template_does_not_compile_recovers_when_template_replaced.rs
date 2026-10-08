#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_test_cluster_harness::model_card::ModelCard;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::desired_state_with_chat_template_override::desired_state_with_chat_template_override;
use paddler_tests::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_chat_template_does_not_compile_recovers_when_template_replaced() {
    let ModelCard { reference } = qwen3_0_6b();
    let mut cluster =
        start_single_agent_cluster_with_desired_state(desired_state_with_chat_template_override(
            BalancerDesiredState {
                model: AgentDesiredModel::HuggingFace(reference.clone()),
                ..BalancerDesiredState::default()
            },
            ChatTemplate {
                content: "{{invalid jinja template".to_owned(),
            },
        ))
        .await
        .expect("a single-agent cluster must start");

    cluster
        .wait_for_first_agent_issue(|issue| {
            matches!(issue, AgentIssue::ChatTemplateDoesNotCompile(_))
        })
        .await
        .expect("the agent must report ChatTemplateDoesNotCompile for invalid Jinja syntax");

    cluster
        .client_management
        .put_balancer_desired_state(
            CancellationToken::new(),
            &desired_state_with_chat_template_override(
                BalancerDesiredState {
                    model: AgentDesiredModel::HuggingFace(reference),
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
                        .any(|issue| matches!(issue, AgentIssue::ChatTemplateDoesNotCompile(_)))
            })
        })
        .await
        .expect("the agent must stay registered while reconciliation clears its template issue");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
