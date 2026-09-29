use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::FunctionCall;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::function::Function;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters::Parameters;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use serde_json::Map;
use serde_json::Value;
use serde_json::json;

#[must_use]
pub fn get_weather_tool() -> Tool<ValidatedParametersSchema> {
    let mut location_properties = Map::new();

    location_properties.insert(
        "location".to_owned(),
        json!({"type": "string", "description": "The city name"}),
    );

    Tool::Function(FunctionCall {
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
    })
}
