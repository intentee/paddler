#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_client::token_result_with_producer::TokenResultWithProducer;
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
async fn agent_multimodal_request_with_valid_tool_generates_response() -> Result<()> {
    let cluster = start_cluster_with_smolvlm2(vec![AgentConfig::single(1)]).await?;
    let image_data_uri = load_test_image_data_uri()?;

    let mut properties = Map::new();
    properties.insert("location".to_owned(), json!({ "type": "string" }));

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
                        text: "Describe this image.".to_owned(),
                    },
                ]),
                role: "user".to_owned(),
            }]),
            enable_thinking: false,
            grammar: None,
            max_tokens: 16,
            parse_tool_calls: true,
            tools: vec![Tool::Function(FunctionCall {
                function: Function {
                    name: "get_weather".to_owned(),
                    description: "Get the current weather for a location".to_owned(),
                    parameters: Parameters::Schema(ValidatedParametersSchema {
                        schema_type: "object".to_owned(),
                        properties: Some(properties),
                        required: None,
                        additional_properties: None,
                    }),
                },
            })],
        })
        .await?;

    assert!(
        !collected.token_results.iter().any(|event| matches!(
            event.token_result,
            GeneratedTokenResult::ToolSchemaInvalid(_)
        )),
        "a valid tool schema must not be rejected; got:\n{}",
        collected.text
    );
    assert!(matches!(
        collected.token_results.last(),
        Some(TokenResultWithProducer {
            token_result: GeneratedTokenResult::Done(_),
            ..
        })
    ));

    cluster.shutdown().await?;

    Ok(())
}
