use serde_json::Map;
use serde_json::Value;
use serde_json::json;

use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;

#[must_use]
pub fn weather_tool_parameters_with_an_unsatisfiable_location() -> ValidatedParametersSchema {
    ValidatedParametersSchema {
        schema_type: "object".to_owned(),
        properties: Some(Map::from_iter([(
            "location".to_owned(),
            json!({
                "type": "integer",
                "minimum": 1,
                "maximum": 0,
                "description": "The city name"
            }),
        )])),
        required: Some(vec!["location".to_owned()]),
        additional_properties: Some(Value::Bool(false)),
    }
}
