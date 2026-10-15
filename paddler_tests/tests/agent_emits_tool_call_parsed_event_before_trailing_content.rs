#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings_types::ParsedToolCall;
use llama_cpp_bindings_types::ToolCallArguments;
use serde_json::json;
use tokio_util::sync::CancellationToken;

use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::gbnf_literal::gbnf_literal;
use paddler_tests::get_weather_tool::get_weather_tool;
use paddler_tests::max_tokens_for_forced_text::max_tokens_for_forced_text;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use paddler_tests::weather_question_conversation::weather_question_conversation;

const TOOL_CALL_FOLLOWED_BY_CONTENT: &str = "<tool_call>\n{\"name\": \"get_weather\", \"arguments\": {\"location\": \"Paris\"}}\n</tool_call>\nI asked for the weather in Paris.";

#[tokio::test(flavor = "multi_thread")]
async fn agent_emits_tool_call_parsed_event_before_trailing_content() {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 1))
        .await
        .expect("the cluster must start");

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &weather_question_conversation(
                gbnf_literal(TOOL_CALL_FOLLOWED_BY_CONTENT),
                max_tokens_for_forced_text(TOOL_CALL_FOLLOWED_BY_CONTENT),
                vec![get_weather_tool()],
            ),
        )
        .await
        .expect("the inference request must be accepted");

    let token_results = collected.into_token_results();

    let tool_call_position = token_results
        .iter()
        .position(|token_result| matches!(token_result, GeneratedTokenResult::ToolCallParsed(_)));
    let last_content_position = token_results
        .iter()
        .rposition(|token_result| matches!(token_result, GeneratedTokenResult::ContentToken(_)));

    assert!(
        tool_call_position < last_content_position,
        "expected content after the parsed tool call, got {token_results:?}"
    );

    let parsed_tool_calls: Vec<&ParsedToolCall> = token_results
        .iter()
        .filter_map(|token_result| match token_result {
            GeneratedTokenResult::ToolCallParsed(parsed_tool_calls) => Some(parsed_tool_calls),
            _ => None,
        })
        .flatten()
        .collect();

    assert_eq!(parsed_tool_calls.len(), 1);
    assert_eq!(parsed_tool_calls[0].name, "get_weather");
    assert_eq!(
        parsed_tool_calls[0].arguments,
        ToolCallArguments::ValidJson(json!({"location": "Paris"}))
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
