#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use llama_cpp_bindings::ToolCallArguments;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::grammar_constraint::GrammarConstraint;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::FunctionCall;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::function::Function;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters::Parameters;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use serde_json::Map;
use serde_json::Value;
use serde_json::json;
use tokio_util::sync::CancellationToken;

const TOOL_CALL_FOLLOWED_BY_CONTENT: &str = "<tool_call>\n{\"name\": \"get_weather\", \"arguments\": {\"location\": \"Paris\"}}\n</tool_call>\nI asked for the weather in Paris.";

#[tokio::test(flavor = "multi_thread")]
async fn agent_emits_tool_call_parsed_event_before_trailing_content() -> Result<()> {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 1)).await?;

    let mut location_properties = Map::new();
    location_properties.insert(
        "location".to_owned(),
        json!({"type": "string", "description": "The city name"}),
    );

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![ConversationMessage {
                    content: ConversationMessageContent::Text(
                        "What is the weather in Paris?".to_owned(),
                    ),
                    role: "user".to_owned(),
                }]),
                enable_thinking: false,
                grammar: Some(GrammarConstraint::Gbnf {
                    grammar: format!(
                        "root ::= {}",
                        serde_json::to_string(TOOL_CALL_FOLLOWED_BY_CONTENT)?
                    ),
                    root: "root".to_owned(),
                }),
                max_tokens: i32::try_from(TOOL_CALL_FOLLOWED_BY_CONTENT.len())?,
                parse_tool_calls: true,
                tools: vec![Tool::Function(FunctionCall {
                    function: Function {
                        name: "get_weather".to_owned(),
                        description: "Get the current weather for a location".to_owned(),
                        parameters: Parameters::Schema(ValidatedParametersSchema {
                            schema_type: "object".to_owned(),
                            properties: Some(location_properties),
                            required: Some(vec!["location".to_owned()]),
                            additional_properties: Some(Value::Bool(false)),
                        }),
                    },
                })],
            },
        )
        .await?;

    let token_results: Vec<GeneratedTokenResult> = collected
        .token_results
        .into_iter()
        .map(|token_result_with_producer| token_result_with_producer.token_result)
        .collect();

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

    let Some(GeneratedTokenResult::ToolCallParsed(parsed_tool_calls)) =
        tool_call_position.map(|position| &token_results[position])
    else {
        panic!("expected a parsed tool call, got {token_results:?}");
    };

    assert_eq!(parsed_tool_calls.len(), 1);
    assert_eq!(parsed_tool_calls[0].name, "get_weather");
    assert_eq!(
        parsed_tool_calls[0].arguments,
        ToolCallArguments::ValidJson(json!({"location": "Paris"}))
    );

    cluster.shutdown().await?;

    Ok(())
}
