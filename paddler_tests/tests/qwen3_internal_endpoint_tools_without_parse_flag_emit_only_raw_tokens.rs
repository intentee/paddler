#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::start_in_process_cluster_with_qwen3::start_in_process_cluster_with_qwen3;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_types::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::FunctionCall;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::function::Function;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters::Parameters;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use reqwest::Client;
use serde_json::Map;
use serde_json::Value;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn qwen3_internal_endpoint_tools_without_parse_flag_emit_only_raw_tokens() -> Result<()> {
    let cluster = start_in_process_cluster_with_qwen3(1).await?;

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
            parse_tool_calls: false,
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

    for event in &collected.token_results {
        match event {
            GeneratedTokenResult::ToolCallParsed(_)
            | GeneratedTokenResult::ToolCallParseFailed(_)
            | GeneratedTokenResult::ToolSchemaInvalid(_)
            | GeneratedTokenResult::ToolCallValidationFailed(_) => {
                anyhow::bail!(
                    "expected no parsed/parse-failed/schema-invalid/validation-failed events when parse_tool_calls=false, got: {event:?}"
                );
            }
            _ => {}
        }
    }

    let last = collected
        .token_results
        .last()
        .ok_or_else(|| anyhow::anyhow!("no token results received"))?;
    let GeneratedTokenResult::Done(_) = last else {
        anyhow::bail!("last result was not Done: {last:?}");
    };

    cluster.shutdown().await?;

    Ok(())
}
