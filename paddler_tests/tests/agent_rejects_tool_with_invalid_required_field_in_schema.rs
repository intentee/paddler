#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use anyhow::Result;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::start_subprocess_cluster_with_qwen3::start_subprocess_cluster_with_qwen3;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_types::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::FunctionCall;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::function::Function;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters::Parameters;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use reqwest::Client;
use serde_json::Map;

#[tokio::test(flavor = "multi_thread")]
async fn agent_rejects_tool_with_invalid_required_field_in_schema() -> Result<()> {
    let cluster = start_subprocess_cluster_with_qwen3(2, 1).await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let mut name_properties = Map::new();

    name_properties.insert("name".to_owned(), serde_json::json!({"type": "string"}));

    let outcome = inference_client
        .post_continue_from_conversation_history(&ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Text("Say hello".to_owned()),
                role: "user".to_owned(),
            }]),
            enable_thinking: true,
            grammar: None,
            max_tokens: 10,
            tools: vec![Tool::Function(FunctionCall {
                function: Function {
                    name: "test_fn".to_owned(),
                    description: "test".to_owned(),
                    parameters: Parameters::Schema(ValidatedParametersSchema {
                        schema_type: "object".to_owned(),
                        properties: Some(name_properties),
                        required: Some(vec!["nonexistent_field".to_owned()]),
                        additional_properties: None,
                    }),
                },
            })],
        })
        .await;

    assert!(
        outcome.is_err(),
        "request with invalid schema (required field not in properties) must be rejected"
    );

    cluster.shutdown().await?;

    Ok(())
}
