#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use serde_json::Map;
use serde_json::Value;
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
async fn agent_reports_tool_call_validation_failure() {
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

    let mut location_properties = Map::new();
    location_properties.insert(
        "location".to_owned(),
        json!({"type": "integer", "description": "The city name"}),
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
                max_tokens: NonZeroU32::new(400).unwrap(),
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
        .await
        .expect("the inference request must be accepted");

    let validation_failures: Vec<&Vec<String>> = collected
        .token_results
        .iter()
        .filter_map(|event| match &event.token_result {
            GeneratedTokenResult::ToolCallValidationFailed(messages) => Some(messages),
            _ => None,
        })
        .collect();

    assert!(
        !validation_failures.is_empty(),
        "expected at least one ToolCallValidationFailed event when the model emits a string \
         location against an integer-typed schema; got tokens:\n{}",
        collected.text
    );

    let first_failure = validation_failures
        .iter()
        .flat_map(|messages| messages.iter())
        .next()
        .expect("no validation-failure messages in any event");

    assert!(
        first_failure.contains("get_weather"),
        "validation-failure message should name the offending tool; got: {first_failure}"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
