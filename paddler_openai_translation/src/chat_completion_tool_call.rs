use llama_cpp_bindings_types::ParsedToolCall;
use serde::Serialize;
use serde::Serializer;

use crate::arguments_to_tool_call_string::arguments_to_tool_call_string;

#[derive(Serialize)]
struct SerializedFunction<'call> {
    name: &'call str,
    arguments: String,
}

#[derive(Serialize)]
struct SerializedToolCall<'call> {
    id: &'call str,
    #[serde(rename = "type")]
    call_type: &'static str,
    function: SerializedFunction<'call>,
}

pub struct ChatCompletionToolCall<'call>(pub &'call ParsedToolCall);

impl Serialize for ChatCompletionToolCall<'_> {
    fn serialize<TSerializer>(
        &self,
        serializer: TSerializer,
    ) -> Result<TSerializer::Ok, TSerializer::Error>
    where
        TSerializer: Serializer,
    {
        let Self(parsed_tool_call) = self;

        SerializedToolCall {
            id: &parsed_tool_call.id,
            call_type: "function",
            function: SerializedFunction {
                name: &parsed_tool_call.name,
                arguments: arguments_to_tool_call_string(&parsed_tool_call.arguments),
            },
        }
        .serialize(serializer)
    }
}
