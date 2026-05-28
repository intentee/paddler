#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::start_in_process_cluster_with_gemma_4::start_in_process_cluster_with_gemma_4;
use paddler::conversation_history::ConversationHistory;
use paddler::conversation_message::ConversationMessage;
use paddler::conversation_message_content::ConversationMessageContent;
use paddler::generated_token_result::GeneratedTokenResult;
use paddler::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::FunctionCall;
use paddler::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::function::Function;
use paddler::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters::Parameters;
use paddler::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use reqwest::Client;
use serde_json::Map;
use serde_json::Value;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn gemma4_internal_endpoint_emits_tool_call_parsed_event() -> Result<()> {
    let cluster = start_in_process_cluster_with_gemma_4(AgentConfig::single(1)).await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let mut location_properties = Map::new();
    location_properties.insert(
        "location".to_owned(),
        serde_json::json!({"type": "string", "description": "The city name"}),
    );

    let stream = inference_client
        .post_continue_from_conversation_history(&ContinueFromConversationHistoryParams {
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
            max_tokens: 400,
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
        })
        .await?;

    let collected = collect_generated_tokens(stream).await?;

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
        "Gemma 4: expected at least one ToolCallParsed event; got tokens:\n{}",
        collected.text
    );

    let first_call = parsed_events
        .iter()
        .flat_map(|calls| calls.iter())
        .next()
        .ok_or_else(|| anyhow::anyhow!("no parsed tool calls in any event"))?;

    assert_eq!(first_call.name, "get_weather");

    cluster.shutdown().await?;

    Ok(())
}
