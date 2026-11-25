use llama_cpp_bindings_types::parsed_tool_call::ParsedToolCall;
use llama_cpp_bindings_types::parsed_tool_call_id::ParsedToolCallId;

use paddler_messaging::generated_tool_call::GeneratedToolCall;

use crate::parsed_tool_call_arguments::ParsedToolCallArguments;

pub struct NumberedToolCall {
    pub number: usize,
    pub parsed_tool_call: ParsedToolCall,
}

impl NumberedToolCall {
    #[must_use]
    pub fn into_generated_tool_call(self) -> GeneratedToolCall {
        let Self {
            number,
            parsed_tool_call:
                ParsedToolCall {
                    arguments,
                    id,
                    name,
                },
        } = self;

        GeneratedToolCall {
            arguments: ParsedToolCallArguments(arguments).into_tool_call_arguments(),
            id: match id {
                ParsedToolCallId::ModelProvided(model_provided_id) => {
                    model_provided_id.as_str().to_owned()
                }
                ParsedToolCallId::NotProvidedByModel => format!("call_{number}"),
            },
            index: number,
            name,
        }
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings_types::model_provided_tool_call_id::ModelProvidedToolCallId;
    use llama_cpp_bindings_types::parsed_tool_call::ParsedToolCall;
    use llama_cpp_bindings_types::parsed_tool_call_id::ParsedToolCallId;
    use llama_cpp_bindings_types::tool_call_arguments::ToolCallArguments as ParsedToolCallArgumentsValue;
    use serde_json::json;

    use paddler_messaging::generated_tool_call::GeneratedToolCall;
    use paddler_messaging::tool_call_arguments::ToolCallArguments;

    use super::NumberedToolCall;

    fn weather_call(id: ParsedToolCallId) -> ParsedToolCall {
        ParsedToolCall::new(
            id,
            "get_weather".to_owned(),
            ParsedToolCallArgumentsValue::ValidJson(json!({"location": "Paris"})),
        )
    }

    #[test]
    fn a_tool_call_the_model_gave_an_id_keeps_that_id() {
        assert_eq!(
            NumberedToolCall {
                number: 3,
                parsed_tool_call: weather_call(ParsedToolCallId::ModelProvided(
                    ModelProvidedToolCallId::try_from("abc123def".to_owned()).unwrap(),
                )),
            }
            .into_generated_tool_call()
            .id,
            "abc123def"
        );
    }

    #[test]
    fn a_tool_call_the_model_gave_no_id_is_named_after_its_number_in_the_generation() {
        assert_eq!(
            NumberedToolCall {
                number: 3,
                parsed_tool_call: weather_call(ParsedToolCallId::NotProvidedByModel),
            }
            .into_generated_tool_call(),
            GeneratedToolCall {
                arguments: ToolCallArguments::ValidJson(json!({"location": "Paris"})),
                id: "call_3".to_owned(),
                index: 3,
                name: "get_weather".to_owned(),
            }
        );
    }
}
