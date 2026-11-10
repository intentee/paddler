use llama_cpp_bindings_types::tool_call_arguments::ToolCallArguments;
use serde_json::json;

use paddler_messaging::generated_tool_call::GeneratedToolCall;

#[must_use]
pub fn weather_call() -> GeneratedToolCall {
    GeneratedToolCall {
        arguments: ToolCallArguments::ValidJson(json!({ "location": "Paris" })),
        id: "call_x".to_owned(),
        index: 0,
        name: "get_weather".to_owned(),
    }
}
