#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_text_generation_tests::chat_template_rendering_every_message_after::chat_template_rendering_every_message_after;
use paddler_balancer_desired_state_fixture::desired_state_with_chat_template_override::desired_state_with_chat_template_override;
use paddler_cluster_harness::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_model_card_fixture::smolvlm2_256m::smolvlm2_256m;

#[tokio::test(flavor = "multi_thread")]
async fn a_template_llama_cpp_cannot_probe_leaves_the_agent_serving_nothing() {
    let mut cluster =
        start_single_agent_cluster_with_desired_state(desired_state_with_chat_template_override(
            smolvlm2_256m().into_desired_state(),
            chat_template_rendering_every_message_after(
                "{% if messages[0]['role'] != 'system' %}{{ raise_exception('the first message must be the system message') }}{% endif %}",
            ),
        ))
        .await
        .expect("a single-agent cluster must start");

    let agents = cluster
        .wait_for_first_agent_issue(|issue| {
            matches!(issue, AgentIssue::ChatTemplateCapabilitiesUnreadable(_))
        })
        .await
        .expect("the agent must report that llama.cpp cannot probe its chat template");

    assert!(
        agents
            .agents
            .iter()
            .all(|agent| !matches!(agent.status.runtime, AgentRuntimeStatus::Serving { .. }))
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
