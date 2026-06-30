#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_cluster::agent_config::AgentConfig;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::image_url::ImageUrl;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::FunctionCall;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::function::Function;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters::Parameters;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_tests::load_test_image_data_uri::load_test_image_data_uri;
use paddler_tests::start_cluster_with_smolvlm2::start_cluster_with_smolvlm2;
use serde_json::Map;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn agent_rejects_multimodal_request_with_invalid_tool_schema() -> Result<()> {
    let cluster = start_cluster_with_smolvlm2(vec![AgentConfig::single(1)]).await?;
    let image_data_uri = load_test_image_data_uri()?;

    let mut invalid_properties = Map::new();
    invalid_properties.insert("location".to_owned(), json!({ "type": 123 }));

    let collected = cluster
        .inference_client
        .http()
        .continue_from_conversation_history_collected(&ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Parts(vec![
                    ConversationMessageContentPart::ImageUrl {
                        image_url: ImageUrl {
                            url: image_data_uri,
                        },
                    },
                    ConversationMessageContentPart::Text {
                        text: "What is the weather in this image?".to_owned(),
                    },
                ]),
                role: "user".to_owned(),
            }]),
            enable_thinking: false,
            grammar: None,
            max_tokens: 64,
            parse_tool_calls: true,
            tools: vec![Tool::Function(FunctionCall {
                function: Function {
                    name: "get_weather".to_owned(),
                    description: "Get the current weather for a location".to_owned(),
                    parameters: Parameters::Schema(ValidatedParametersSchema {
                        schema_type: "object".to_owned(),
                        properties: Some(invalid_properties),
                        required: None,
                        additional_properties: None,
                    }),
                },
            })],
        })
        .await?;

    let rejected_with_invalid_tool_schema = collected.token_results.iter().any(|event| {
        matches!(
            event.token_result,
            GeneratedTokenResult::ToolSchemaInvalid(_)
        )
    });

    assert!(rejected_with_invalid_tool_schema);

    cluster.shutdown().await?;

    Ok(())
}
