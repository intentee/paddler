#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
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
use serde_json::Map;
use serde_json::Value;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn qwen3_internal_endpoint_emits_tool_call_tokens() -> Result<()> {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)]).await?;

    let mut location_properties = Map::new();
    location_properties.insert(
        "location".to_owned(),
        serde_json::json!({"type": "string", "description": "The city name"}),
    );

    let collected = cluster
        .continue_from_conversation_history(&ContinueFromConversationHistoryParams {
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
        .ok_or_else(|| anyhow::anyhow!("no token results received"))?;
    let GeneratedTokenResult::Done(summary) = &last.token_result else {
        anyhow::bail!("last result was not Done: {last:?}");
    };

    assert!(summary.usage.prompt_tokens > 0);
    assert!(
        tool_call_count > 0,
        "expected ToolCallToken (got {tool_call_count}); content_count={content_count}; usage={:?}; generated text:\n{}",
        summary.usage,
        collected.text,
    );
    assert!(summary.usage.tool_call_tokens > 0);

    cluster.shutdown().await?;

    Ok(())
}
