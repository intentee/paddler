#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_cli_tests::agent_config::AgentConfig;
use paddler_cli_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_cli_tests::inference_http_client::InferenceHttpClient;
use paddler_cli_tests::start_subprocess_cluster_with_qwen3::start_subprocess_cluster_with_qwen3;
use paddler::conversation_history::ConversationHistory;
use paddler::conversation_message::ConversationMessage;
use paddler::conversation_message_content::ConversationMessageContent;
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
async fn agent_conversation_with_function_tool_succeeds() -> Result<()> {
    let cluster = start_subprocess_cluster_with_qwen3(AgentConfig::uniform(1, 2)).await?;

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
                content: ConversationMessageContent::Text("Say hello".to_owned()),
                role: "user".to_owned(),
            }]),
            enable_thinking: true,
            grammar: None,
            max_tokens: 50,
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

    assert!(
        !collected.token_results.is_empty(),
        "should receive a response when a function tool is provided"
    );

    cluster.shutdown().await?;

    Ok(())
}
