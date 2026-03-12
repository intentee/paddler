use std::time::Duration;

use futures_util::StreamExt;
use integration_tests::AGENT_DESIRED_MODEL;
use integration_tests::BALANCER_INFERENCE_ADDR;
use integration_tests::BALANCER_MANAGEMENT_ADDR;
use integration_tests::balancer_params;
use integration_tests::managed_agent::ManagedAgent;
use integration_tests::managed_agent::ManagedAgentParams;
use integration_tests::managed_balancer::ManagedBalancer;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::inference_client::Message;
use paddler_types::inference_client::Response;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::ContinueFromConversationHistoryParams;
use paddler_types::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::FunctionCall;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::function::Function;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters::Parameters;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use serial_test::file_serial;
use serde_json::Map;
use serde_json::Value;
use tempfile::NamedTempFile;

struct ToolsTestCluster {
    balancer: ManagedBalancer,
    _agent: ManagedAgent,
    _state_db: NamedTempFile,
}

async fn spawn_tools_cluster() -> ToolsTestCluster {
    let state_db = NamedTempFile::new().expect("failed to create temp file");

    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: AGENT_DESIRED_MODEL.clone(),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    let balancer = ManagedBalancer::spawn(balancer_params(
        BALANCER_MANAGEMENT_ADDR,
        BALANCER_INFERENCE_ADDR,
        state_db.path().to_str().unwrap(),
        10,
        Duration::from_secs(10),
    ))
    .await
    .expect("failed to spawn balancer");

    balancer
        .client()
        .management()
        .put_balancer_desired_state(&desired_state)
        .await
        .expect("failed to set balancer desired state");

    balancer.wait_for_desired_state(&desired_state).await;

    let agent = ManagedAgent::spawn(ManagedAgentParams {
        management_addr: BALANCER_MANAGEMENT_ADDR.to_string(),
        name: Some("tools-agent".to_string()),
        slots: 4,
    })
    .await
    .expect("failed to spawn agent");

    balancer.wait_for_agent_count(1).await;
    balancer.wait_for_total_slots(4).await;

    ToolsTestCluster {
        balancer,
        _agent: agent,
        _state_db: state_db,
    }
}

fn assert_stream_received_tokens(messages: Vec<Message>) {
    let mut received_tokens = false;

    for message in messages {
        match message {
            Message::Response(envelope) => match envelope.response {
                Response::GeneratedToken(token_result) => match token_result {
                    GeneratedTokenResult::Token(_) => {
                        received_tokens = true;
                    }
                    GeneratedTokenResult::Done => break,
                    other => panic!("unexpected token result: {other:?}"),
                },
                other => panic!("unexpected response: {other:?}"),
            },
            Message::Error(envelope) => {
                panic!(
                    "unexpected error: {} - {}",
                    envelope.error.code, envelope.error.description
                );
            }
        }
    }

    assert!(received_tokens, "should have received at least one token");
}

fn assert_stream_received_valid_response(messages: Vec<Message>) {
    let mut received_response = false;

    for message in messages {
        match message {
            Message::Response(envelope) => match envelope.response {
                Response::GeneratedToken(_) => {
                    received_response = true;
                }
                other => panic!("unexpected response: {other:?}"),
            },
            Message::Error(envelope) => {
                panic!(
                    "unexpected error: {} - {}",
                    envelope.error.code, envelope.error.description
                );
            }
        }
    }

    assert!(
        received_response,
        "should have received at least one valid response"
    );
}

async fn collect_stream_messages(
    mut stream: std::pin::Pin<
        Box<dyn futures_util::Stream<Item = paddler_client::Result<Message>> + Send>,
    >,
) -> Vec<Message> {
    let mut messages = Vec::new();

    while let Some(message) = stream.next().await {
        messages.push(message.expect("message should deserialize"));
    }

    messages
}

#[tokio::test]
#[file_serial]
async fn test_tools_parameter_is_optional() {
    let cluster = spawn_tools_cluster().await;

    let stream = cluster
        .balancer
        .client()
        .inference()
        .continue_from_conversation_history(ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Text("Say hello".to_string()),
                role: "user".to_string(),
            }]),
            enable_thinking: true,
            max_tokens: 10,
            tools: vec![],
        })
        .await
        .expect("request without tools should succeed");

    let messages = collect_stream_messages(stream).await;

    assert_stream_received_tokens(messages);
}

#[tokio::test]
#[file_serial]
async fn test_tools_with_function() {
    let cluster = spawn_tools_cluster().await;

    let mut location_properties = Map::new();
    location_properties.insert(
        "location".to_string(),
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
                content: ConversationMessageContent::Text("Say hello".to_string()),
                role: "user".to_string(),
            }]),
            enable_thinking: true,
            max_tokens: 50,
            tools: vec![Tool::Function(FunctionCall {
                function: Function {
                    name: "get_weather".to_string(),
                    description: "Get the current weather for a location".to_string(),
                    parameters: Parameters::Schema(ValidatedParametersSchema {
                        schema_type: "object".to_string(),
                        properties: Some(location_properties),
                        required: Some(vec!["location".to_string()]),
                        additional_properties: Some(Value::Bool(false)),
                    }),
                },
            })],
        })
        .await
        .expect("request with function tool should succeed");

    let messages = collect_stream_messages(stream).await;

    assert_stream_received_valid_response(messages);
}

#[tokio::test]
#[file_serial]
async fn test_tools_schema_validation() {
    let cluster = spawn_tools_cluster().await;

    let mut name_properties = Map::new();
    name_properties.insert("name".to_string(), serde_json::json!({ "type": "string" }));

    let result = cluster
        .balancer
        .client()
        .inference()
        .post_continue_from_conversation_history(&ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Text("Say hello".to_string()),
                role: "user".to_string(),
            }]),
            enable_thinking: true,
            max_tokens: 10,
            tools: vec![Tool::Function(FunctionCall {
                function: Function {
                    name: "test_fn".to_string(),
                    description: "test".to_string(),
                    parameters: Parameters::Schema(ValidatedParametersSchema {
                        schema_type: "object".to_string(),
                        properties: Some(name_properties),
                        required: Some(vec!["nonexistent_field".to_string()]),
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
}
