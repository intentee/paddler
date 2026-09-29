#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use anyhow::Result;
use anyhow::anyhow;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::get_weather_tool::get_weather_tool;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn qwen3_internal_endpoint_emits_tool_call_parsed_event() -> Result<()> {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)]).await?;

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
        .await?;

    let parsed_events: Vec<&Vec<llama_cpp_bindings::ParsedToolCall>> = collected
        .token_results
        .iter()
        .filter_map(|event| match &event.token_result {
            GeneratedTokenResult::ToolCallParsed(parsed) => Some(parsed),
            _ => None,
        })
        .collect();

    assert!(
        !parsed_events.is_empty(),
        "expected at least one ToolCallParsed event; got tokens:\n{}",
        collected.text
    );

    let first_call = parsed_events
        .iter()
        .flat_map(|calls| calls.iter())
        .next()
        .ok_or_else(|| anyhow!("no parsed tool calls in any event"))?;

    assert_eq!(first_call.name, "get_weather");

    cluster.shutdown().await?;

    Ok(())
}
