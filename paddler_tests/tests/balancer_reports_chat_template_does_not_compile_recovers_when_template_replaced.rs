#![cfg(feature = "tests_that_use_llms")]

use std::time::Duration;

use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_test_cluster_harness::observation_window::ObservationWindow;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_chat_template_does_not_compile_recovers_when_template_replaced() {
    let ModelCard { reference, .. } = qwen3_0_6b();
    let mut cluster = start_single_agent_cluster_with_desired_state(BalancerDesiredState {
        chat_template_override: Some(ChatTemplate {
            content: "{{invalid jinja template".to_owned(),
        }),
        model: AgentDesiredModel::HuggingFace(reference.clone()),
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
        .client_management
        .put_balancer_desired_state(
            CancellationToken::new(),
            &BalancerDesiredState {
                chat_template_override: Some(ChatTemplate {
                    content: "{% for message in messages %}{{ message.content }}{% endfor %}"
                        .to_owned(),
                }),
                model: AgentDesiredModel::HuggingFace(reference),
                use_chat_template_override: true,
                ..BalancerDesiredState::default()
            },
        )
        .await
        .expect("the balancer must accept the desired state with a valid template");

    let agent_id = cluster
        .agent_ids
        .first()
        .expect("the cluster must have a registered agent")
        .clone();

    timeout(
        Duration::from_secs(3),
        cluster.agents_watcher.until_agent(
            &agent_id,
            ObservationWindow::model_load(),
            |snapshot| {
                snapshot.agents.iter().any(|agent| {
                    agent.id == agent_id
                        && !agent
                            .issues
                            .iter()
                            .any(|issue| matches!(issue, AgentIssue::ChatTemplateDoesNotCompile(_)))
                })
            },
        ),
    )
    .await
    .expect("reconciliation must clear ChatTemplateDoesNotCompile within 3 seconds")
    .expect("the agent must stay registered while its template is replaced");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
