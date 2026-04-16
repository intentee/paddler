#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use futures_util::StreamExt;
use paddler_integration_tests::managed_cluster::ManagedCluster;
use paddler_integration_tests::managed_cluster_params::ManagedClusterParams;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::inference_client::Message;
use paddler_types::inference_client::Response;
use paddler_types::request_params::ContinueFromConversationHistoryParams;
use paddler_types::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::FunctionCall;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::function::Function;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters::Parameters;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use serial_test::file_serial;
use serde_json::Map;
use serde_json::Value;

fn assert_stream_received_tokens(messages: Vec<Message>) -> Result<()> {
    let mut received_tokens = false;

    for message in messages {
        match message {
            Message::Response(envelope) => match envelope.response {
                Response::GeneratedToken(token_result) => match token_result {
                    GeneratedTokenResult::Token(_) => {
                        received_tokens = true;
                    }
                    GeneratedTokenResult::Done => break,
                    other => return Err(anyhow!("unexpected token result: {other:?}")),
                },
                other => return Err(anyhow!("unexpected response: {other:?}")),
            },
            Message::Error(envelope) => {
                return Err(anyhow!(
                    "unexpected error: {} - {}",
                    envelope.error.code,
                    envelope.error.description
                ));
            }
        }
    }

    assert!(received_tokens, "should have received at least one token");

    Ok(())
}

fn assert_stream_received_valid_response(messages: Vec<Message>) -> Result<()> {
    let mut received_response = false;

    for message in messages {
        match message {
            Message::Response(envelope) => match envelope.response {
                Response::GeneratedToken(_) => {
                    received_response = true;
                }
                other => return Err(anyhow!("unexpected response: {other:?}")),
            },
            Message::Error(envelope) => {
                return Err(anyhow!(
                    "unexpected error: {} - {}",
                    envelope.error.code,
                    envelope.error.description
                ));
            }
        }
    }

    assert!(
        received_response,
        "should have received at least one valid response"
    );

    Ok(())
}

async fn collect_stream_messages(
    mut stream: std::pin::Pin<
        Box<dyn futures_util::Stream<Item = paddler_client::Result<Message>> + Send>,
    >,
) -> Result<Vec<Message>> {
    let mut messages = Vec::new();

    while let Some(message) = stream.next().await {
        messages.push(message.context("message should deserialize")?);
    }

    Ok(messages)
}

#[tokio::test]
#[file_serial]
async fn test_tools_parameter_is_optional() -> Result<()> {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "tools-agent".to_owned(),
        ..ManagedClusterParams::default()
    })
    .await
    .context("failed to spawn cluster")?;

    let stream = cluster
        .balancer
        .client()
        .inference()
        .continue_from_conversation_history(ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Text("Say hello".to_owned()),
                role: "user".to_owned(),
            }]),
            enable_thinking: true,
            grammar: None,
            max_tokens: 10,
            tools: vec![],
        })
        .await
        .context("request without tools should succeed")?;

    let messages = collect_stream_messages(stream).await?;

    assert_stream_received_tokens(messages)?;

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_tools_with_function() -> Result<()> {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "tools-agent".to_owned(),
        ..ManagedClusterParams::default()
    })
    .await
    .context("failed to spawn cluster")?;

    let mut location_properties = Map::new();
    location_properties.insert(
        "location".to_owned(),
        serde_json::json!({
            "type": "string",
            "description": "The city name"
        }),
    );

    let stream = cluster
        .balancer
        .client()
        .inference()
        .continue_from_conversation_history(ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Text("Say hello".to_owned()),
                role: "user".to_owned(),
            }]),
            enable_thinking: true,
            grammar: None,
            max_tokens: 50,
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
        .await
        .context("request with function tool should succeed")?;

    let messages = collect_stream_messages(stream).await?;

    assert_stream_received_valid_response(messages)?;

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_tools_schema_validation() -> Result<()> {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "tools-agent".to_owned(),
        ..ManagedClusterParams::default()
    })
    .await
    .context("failed to spawn cluster")?;

    let mut name_properties = Map::new();
    name_properties.insert("name".to_owned(), serde_json::json!({ "type": "string" }));

    let result = cluster
        .balancer
        .client()
        .inference()
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
        result.is_err(),
        "request with invalid schema should be rejected"
    );

    Ok(())
}
