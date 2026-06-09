#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::FunctionCall;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::function::Function;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters::Parameters;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_cluster::start_cluster;
use serde_json::Map;

#[tokio::test(flavor = "multi_thread")]
async fn agent_rejects_structurally_invalid_tool_schema() -> Result<()> {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let cluster = start_cluster(ClusterParams {
        agents: vec![AgentConfig {
            name: "test-agent".to_owned(),
            slot_count: 1,
        }],
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters {
                n_gpu_layers: gpu_layer_count,
                temperature: 0.0,
                ..InferenceParameters::default()
            },
            model: AgentDesiredModel::HuggingFace(reference),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        }),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await?;

    // `{"type": 123}` is a structurally well-formed JSON object (so it survives
    // request-parameter validation) but is not a valid JSON Schema: the `type`
    // keyword must be a string or an array of strings. `jsonschema::validator_for`
    // rejects it, so the agent's tool-call pipeline build reports the tool's schema
    // as invalid and the scheduler emits `ToolSchemaInvalid` before any generation.
    let mut invalid_properties = Map::new();
    invalid_properties.insert("location".to_owned(), serde_json::json!({ "type": 123 }));

    let collected = cluster
        .continue_from_conversation_history(&ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Text(
                    "What is the weather in Paris?".to_owned(),
                ),
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

    let schema_invalid_message = collected
        .token_results
        .iter()
        .find_map(|event| match &event.token_result {
            GeneratedTokenResult::ToolSchemaInvalid(message) => Some(message.clone()),
            _ => None,
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "expected a ToolSchemaInvalid event when a tool's JSON Schema is invalid; got:\n{}",
                collected.text
            )
        })?;

    assert!(
        schema_invalid_message.contains("get_weather"),
        "the schema-invalid message should name the offending tool; got: {schema_invalid_message}"
    );

    cluster.shutdown().await?;

    Ok(())
}
