#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings_types::ToolCallArguments;
use serde_json::json;
use tokio_util::sync::CancellationToken;

use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::generated_tool_call::GeneratedToolCall;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::gbnf_literal::gbnf_literal;
use paddler_tests::get_weather_tool::get_weather_tool;
use paddler_tests::max_tokens_for_forced_text::max_tokens_for_forced_text;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use paddler_tests::weather_question_conversation::weather_question_conversation;

const TWO_TOOL_CALL_SECTIONS: &str = "<tool_call>\n{\"name\": \"get_weather\", \"arguments\": {\"location\": \"Paris\"}}\n</tool_call>\n<tool_call>\n{\"name\": \"get_weather\", \"arguments\": {\"location\": \"Berlin\"}}\n</tool_call>";

#[tokio::test(flavor = "multi_thread")]
async fn agent_numbers_tool_calls_across_separate_tool_call_sections() {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 1))
        .await
        .expect("the cluster must start");

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &weather_question_conversation(
                gbnf_literal(TWO_TOOL_CALL_SECTIONS),
                max_tokens_for_forced_text(TWO_TOOL_CALL_SECTIONS),
                vec![get_weather_tool()],
            ),
        )
        .await
        .expect("the inference request must be accepted");

    let tool_call_sections: Vec<Vec<GeneratedToolCall>> = collected
        .into_token_results()
        .into_iter()
        .filter_map(|token_result| match token_result {
            GeneratedTokenResult::ToolCallParsed(generated_tool_calls) => {
                Some(generated_tool_calls)
            }
            _ => None,
        })
        .collect();

    assert_eq!(
        tool_call_sections,
        vec![
            vec![GeneratedToolCall {
                arguments: ToolCallArguments::ValidJson(json!({"location": "Paris"})),
                id: "call_0".to_owned(),
                index: 0,
                name: "get_weather".to_owned(),
            }],
            vec![GeneratedToolCall {
                arguments: ToolCallArguments::ValidJson(json!({"location": "Berlin"})),
                id: "call_1".to_owned(),
                index: 1,
                name: "get_weather".to_owned(),
            }],
        ]
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
