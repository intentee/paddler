#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

const MAX_TOKENS: u32 = 16;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_serves_a_conversation_over_websocket() {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 1))
        .await
        .expect("the cluster must start");

    let collected = collect_generated_tokens(
        cluster
            .client_inference
            .continue_from_conversation_history(
                CancellationToken::new(),
                ContinueFromConversationHistoryParams {
                    add_generation_prompt: true,
                    conversation_history: ConversationHistory::new(vec![
                        ConversationMessage::Message {
                            content: ConversationMessageContent::Text(
                                "What is the capital of France?".to_owned(),
                            ),
                            role: "user".to_owned(),
                        },
                    ]),
                    enable_thinking: false,
                    grammar: None,
                    max_tokens: NonZeroU32::new(MAX_TOKENS).expect("the token budget is non-zero"),
                    parse_tool_calls: false,
                    tools: Vec::new(),
                },
            )
            .await
            .expect("the conversation must be accepted over the inference socket"),
    )
    .await
    .expect("the conversation tokens must be collected");

    assert!(
        collected
            .summary()
            .expect("the conversation must finish with a summary")
            .usage
            .completion_tokens()
            > 0
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
