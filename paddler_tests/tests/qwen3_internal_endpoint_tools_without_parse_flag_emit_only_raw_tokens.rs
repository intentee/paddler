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
async fn qwen3_internal_endpoint_tools_without_parse_flag_emit_only_raw_tokens() {
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
                parse_tool_calls: false,
                tools: vec![get_weather_tool()],
            },
        )
        .await
        .expect("the inference request must be accepted");

    for event in &collected.token_results {
        match &event.token_result {
            GeneratedTokenResult::ToolCallParsed(_)
            | GeneratedTokenResult::ToolCallParseFailed(_)
            | GeneratedTokenResult::ToolSchemaInvalid(_)
            | GeneratedTokenResult::ToolCallValidationFailed(_) => {
                panic!(
                    "expected no parsed/parse-failed/schema-invalid/validation-failed events when parse_tool_calls=false, got: {event:?}"
                );
            }
            _ => {}
        }
    }

    let last = collected
        .token_results
        .last()
        .expect("no token results received");
    let GeneratedTokenResult::Done(_) = &last.token_result else {
        panic!("last result was not Done: {last:?}");
    };

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
