#![cfg(feature = "tests_that_use_llms")]

use std::sync::Arc;

use anyhow::Result;
use llama_cpp_bindings::llama_backend::LlamaBackend;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::model::params::LlamaModelParams;
use paddler_agent::tool_call_event::ToolCallEvent;
use paddler_agent::tool_call_pipeline::ToolCallPipeline;
use paddler_agent::tool_call_validator::ToolCallValidator;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::FunctionCall;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::function::Function;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters::Parameters;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_model_card::model_card::ModelCard;
use paddler_model_card::qwen3_0_6b::qwen3_0_6b;
use serde_json::Map;
use serde_json::json;

#[test]
fn agent_tool_call_pipeline_resolves_empty_when_no_tokens_fed() -> Result<()> {
    let backend = LlamaBackend::init()?;

    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let path = hf_hub::api::sync::ApiBuilder::from_env()
        .build()?
        .model(reference.repo_id.clone())
        .get(&reference.filename)?;

    let model_params = LlamaModelParams::default().with_n_gpu_layers(gpu_layer_count);
    let model = Arc::new(LlamaModel::load_from_file(&backend, &path, &model_params)?);

    let mut location_properties = Map::new();
    location_properties.insert("location".to_owned(), json!({ "type": "string" }));
    let tools = vec![Tool::Function(FunctionCall {
        function: Function {
            name: "get_weather".to_owned(),
            description: "Get the current weather for a location".to_owned(),
            parameters: Parameters::Schema(ValidatedParametersSchema {
                schema_type: "object".to_owned(),
                properties: Some(location_properties),
                required: None,
                additional_properties: None,
            }),
        },
    })];

    let validator = ToolCallValidator::from_tools(&tools)?;
    let tools_json: Vec<serde_json::Value> = tools
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<_, _>>()?;
    let mut pipeline = ToolCallPipeline::new(model, &tools_json, validator);

    let event = pipeline.finalize();

    assert!(matches!(event, ToolCallEvent::Resolved(parsed) if parsed.is_empty()));

    Ok(())
}
