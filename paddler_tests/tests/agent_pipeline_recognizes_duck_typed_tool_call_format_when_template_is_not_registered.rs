#![cfg(feature = "tests_that_use_llms")]

use std::sync::Arc;

use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use llama_cpp_bindings::ToolCallArguments;
use llama_cpp_bindings::llama_backend::LlamaBackend;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::model::params::LlamaModelParams;
use paddler_agent::tool_call_event::ToolCallEvent;
use paddler_agent::tool_call_pipeline::ToolCallPipeline;
use paddler_agent::tool_call_validator::ToolCallValidator;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::deepseek_r1_distill_llama_8b::deepseek_r1_distill_llama_8b;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::FunctionCall;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::function::Function;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters::Parameters;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use serde_json::Map;
use serde_json::json;

const QWEN_XML_PAYLOAD: &str = "<tool_call>\n\
<function=get_weather>\n\
<parameter=location>\n\
Paris\n\
</parameter>\n\
</function>\n\
</tool_call>";

#[test]
fn agent_pipeline_recognizes_duck_typed_tool_call_format_when_template_is_not_registered()
-> Result<()> {
    let backend = LlamaBackend::init()?;

    let ModelCard {
        gpu_layer_count,
        reference,
    } = deepseek_r1_distill_llama_8b();

    let path = hf_hub::api::sync::ApiBuilder::from_env()
        .build()?
        .model(reference.repo_id.clone())
        .get(&reference.filename)?;

    let model_params = LlamaModelParams::default().with_n_gpu_layers(gpu_layer_count);
    let model = Arc::new(LlamaModel::load_from_file(&backend, &path, &model_params)?);

    let mut location_properties = Map::new();
    location_properties.insert(
        "location".to_owned(),
        json!({"type": "string", "description": "The city name"}),
    );
    let tools = vec![Tool::Function(FunctionCall {
        function: Function {
            name: "get_weather".to_owned(),
            description: "Get the current weather for a location".to_owned(),
            parameters: Parameters::Schema(ValidatedParametersSchema {
                schema_type: "object".to_owned(),
                properties: Some(location_properties),
                required: Some(vec!["location".to_owned()]),
                additional_properties: Some(serde_json::Value::Bool(false)),
            }),
        },
    })];

    let validator = ToolCallValidator::from_tools(&tools)?;
    let tools_json: Vec<serde_json::Value> = tools
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<_, _>>()?;
    let mut pipeline = ToolCallPipeline::new(model, &tools_json, validator)?;

    pipeline.feed(QWEN_XML_PAYLOAD);
    let event = pipeline.finalize();

    let ToolCallEvent::Resolved(parsed_calls) = event else {
        bail!(
            "duck-type pass must recover Qwen XML on a model with no registered template; \
             expected ToolCallEvent::Resolved, got {event:?}"
        );
    };
    assert_eq!(
        parsed_calls.len(),
        1,
        "expected exactly one parsed tool call; got {parsed_calls:?}"
    );
    assert_eq!(parsed_calls[0].name, "get_weather");

    let mapped = ToolCallEvent::Resolved(parsed_calls)
        .into_generated_token_result()
        .ok_or_else(|| anyhow!("Resolved must produce a GeneratedTokenResult variant"))?;
    let GeneratedTokenResult::ToolCallParsed(wire_calls) = mapped else {
        bail!("expected GeneratedTokenResult::ToolCallParsed after mapping");
    };
    assert_eq!(wire_calls.len(), 1);
    assert_eq!(wire_calls[0].name, "get_weather");
    let location = match &wire_calls[0].arguments {
        ToolCallArguments::ValidJson(value) => value
            .get("location")
            .and_then(|v| v.as_str())
            .map(str::to_owned),
        ToolCallArguments::InvalidJson(raw) => {
            bail!("expected ValidJson, got InvalidJson: {raw}");
        }
    };
    assert_eq!(location.as_deref(), Some("Paris"));

    Ok(())
}
