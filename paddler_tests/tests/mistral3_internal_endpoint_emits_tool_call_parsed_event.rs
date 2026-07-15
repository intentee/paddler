#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use anyhow::anyhow;
use paddler_tests::ministral_3_cluster_params::Ministral3ClusterParams;
use paddler_tests::start_cluster_with_ministral_3::start_cluster_with_ministral_3;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::FunctionCall;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::function::Function;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters::Parameters;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use serde_json::Map;
use serde_json::json;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn mistral3_internal_endpoint_emits_tool_call_parsed_event() -> Result<()> {
    let cluster = start_cluster_with_ministral_3(Ministral3ClusterParams {
        deterministic_sampling: true,
        ..Ministral3ClusterParams::default()
    })
    .await?;

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
        "Mistral 3: expected at least one ToolCallParsed event; got tokens:\n{}",
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
