#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use serde_json::Map;
use serde_json::json;
use tokio_util::sync::CancellationToken;

use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;
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
use paddler_model_card_fixture::qwen3_0_6b::qwen3_0_6b;

#[tokio::test(flavor = "multi_thread")]
async fn agent_rejects_structurally_invalid_tool_schema() {
    let cluster = Cluster::start_in_process(ClusterParams {
        agents: vec![AgentConfig {
            name: "test-agent".to_owned(),
            slot_count: 1,
        }],
        desired_state: ClusterDesiredState::Apply(Box::new(qwen3_0_6b().into_desired_state())),
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
                conversation_history: ConversationHistory::new(vec![
                    ConversationMessage::Message {
                        content: ConversationMessageContent::Text(
                            "What is the weather in Paris?".to_owned(),
                        ),
                        role: "user".to_owned(),
                    },
                ]),
                enable_thinking: ModelDefaultOr::Explicit(false),
                grammar: None,
                max_tokens: NonZeroU32::new(64).unwrap(),
                parallel_tool_calls: ModelDefaultOr::ModelDefault,
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

    assert!(matches!(
        collected.into_token_results().as_slice(),
        [GeneratedTokenResult::ToolSchemaInvalid(_)]
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
