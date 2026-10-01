#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::get_weather_tool::get_weather_tool;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn qwen3_internal_endpoint_emits_tool_call_tokens() {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![ConversationMessage {
                    content: ConversationMessageContent::Text(
                        "What is the weather in Paris? Use the get_weather tool to find out."
                            .to_owned(),
                    ),
                    role: "user".to_owned(),
                }]),
                enable_thinking: false,
                grammar: None,
                max_tokens: NonZeroU32::new(400).unwrap(),
                parse_tool_calls: true,
                tools: vec![get_weather_tool()],
            },
        )
        .await
        .expect("the inference request must be accepted");

    let tool_call_count = collected
        .token_results
        .iter()
        .filter(|result| matches!(result.token_result, GeneratedTokenResult::ToolCallToken(_)))
        .count();
    let content_count = collected
        .token_results
        .iter()
        .filter(|result| matches!(result.token_result, GeneratedTokenResult::ContentToken(_)))
        .count();

    let last = collected
        .token_results
        .last()
        .expect("no token results received");
    let GeneratedTokenResult::Done(summary) = &last.token_result else {
        panic!("last result was not Done: {last:?}");
    };

    assert!(summary.usage.prompt_tokens > 0);
    assert!(
        tool_call_count > 0,
        "expected ToolCallToken (got {tool_call_count}); content_count={content_count}; usage={:?}; generated text:\n{}",
        summary.usage,
        collected.text,
    );
    assert!(summary.usage.tool_call_tokens > 0);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
