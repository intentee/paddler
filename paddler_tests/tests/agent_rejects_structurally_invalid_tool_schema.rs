#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use serde_json::Map;
use serde_json::json;
use tokio_util::sync::CancellationToken;

use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
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
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn agent_rejects_structurally_invalid_tool_schema() {
    let base_desired_state = qwen3_0_6b().into_desired_state();

    let cluster = start_cluster(ClusterParams {
        agents: vec![AgentConfig {
            name: "test-agent".to_owned(),
            slot_count: 1,
        }],
        desired_state: Some(BalancerDesiredState {
            inference_parameters: InferenceParameters {
                temperature: 0.0,
                ..base_desired_state.inference_parameters
            },
            ..base_desired_state
        }),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let mut invalid_properties = Map::new();
    invalid_properties.insert("location".to_owned(), json!({ "type": 123 }));

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![ConversationMessage {
                    content: ConversationMessageContent::Text(
                        "What is the weather in Paris?".to_owned(),
                    ),
                    role: "user".to_owned(),
                }]),
                enable_thinking: false,
                grammar: None,
                max_tokens: NonZeroU32::new(64).unwrap(),
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
        .await
        .expect("the inference request must be accepted");

    let schema_invalid_message = collected
        .token_results
        .iter()
        .find_map(|event| match &event.token_result {
            GeneratedTokenResult::ToolSchemaInvalid(message) => Some(message.clone()),
            _ => None,
        })
        .unwrap_or_else(|| {
            panic!(
                "expected a ToolSchemaInvalid event when a tool's JSON Schema is invalid; got:\n{}",
                collected.text
            )
        });

    assert!(
        schema_invalid_message.contains("get_weather"),
        "the schema-invalid message should name the offending tool; got: {schema_invalid_message}"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
