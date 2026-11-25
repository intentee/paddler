#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings_types::conversation_capability::ConversationCapability;
use llama_cpp_bindings_types::conversation_unsupported_reason::ConversationUnsupportedReason;
use llama_cpp_bindings_types::model_capabilities::ModelCapabilities;

use paddler_agent_text_generation_tests::qwen3_with_a_patched_chat_template::Qwen3WithAPatchedChatTemplate;
use paddler_cluster_harness::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;

#[tokio::test(flavor = "multi_thread")]
async fn a_model_without_a_chat_template_is_reported_as_unable_to_find_one() {
    let qwen3_without_a_chat_template = Qwen3WithAPatchedChatTemplate::without_a_chat_template()
        .expect("the model without a chat template must be written");
    let mut cluster = start_single_agent_cluster_with_desired_state(
        qwen3_without_a_chat_template
            .desired_state()
            .expect("the model without a chat template must be at an absolute path"),
    )
    .await
    .expect("a single-agent cluster must start");

    let snapshot = cluster
        .wait_for_first_agent_to_serve()
        .await
        .expect("the agent must serve the model without a chat template");
    let runtime = snapshot
        .agents
        .first()
        .expect("the snapshot must hold the agent")
        .status
        .runtime;

    assert!(
        matches!(
            runtime,
            AgentRuntimeStatus::Serving {
                capabilities: ModelCapabilities {
                    conversation: ConversationCapability::Unsupported(
                        ConversationUnsupportedReason::ModelHasNoChatTemplate
                    ),
                    ..
                },
                ..
            }
        ),
        "{runtime:?}"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
