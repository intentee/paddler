#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
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
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3_and_context_size::start_cluster_with_qwen3_and_context_size;
use serde_json::Map;
use serde_json::json;
use tokio_util::sync::CancellationToken;

const SEQUENCE_CONTEXT_SIZE: u32 = 256;

#[tokio::test(flavor = "multi_thread")]
async fn agent_rejects_oversized_conversation_before_compiling_its_tools() -> Result<()> {
    let cluster = start_cluster_with_qwen3_and_context_size(
        vec![AgentConfig::single(1)],
        SEQUENCE_CONTEXT_SIZE,
    )
    .await?;

    let mut invalid_properties = Map::new();
    invalid_properties.insert("location".to_owned(), json!({ "type": 123 }));

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![ConversationMessage {
                    content: ConversationMessageContent::Text(
                        "The quick brown fox jumps over the lazy dog. ".repeat(40),
                    ),
                    role: "user".to_owned(),
                }]),
                enable_thinking: false,
                grammar: None,
                max_tokens: 20,
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
            },
        )
        .await?;

    assert!(matches!(
        collected
            .token_results
            .iter()
            .map(|result| &result.token_result)
            .collect::<Vec<_>>()
            .as_slice(),
        [GeneratedTokenResult::PromptExceedsContextSize(details)]
            if details.sequence_context_size == SEQUENCE_CONTEXT_SIZE
    ));

    cluster.shutdown().await?;

    Ok(())
}
