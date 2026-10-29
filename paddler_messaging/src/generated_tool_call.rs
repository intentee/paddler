use llama_cpp_bindings_types::ParsedToolCall;
use llama_cpp_bindings_types::ParsedToolCallId;
use llama_cpp_bindings_types::ToolCallArguments;
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratedToolCall {
    pub arguments: ToolCallArguments,
    pub id: String,
    pub index: usize,
    pub name: String,
}

impl GeneratedToolCall {
    #[must_use]
    pub fn at_index(
        ParsedToolCall {
            arguments,
            id,
            name,
        }: ParsedToolCall,
        index: usize,
    ) -> Self {
        Self {
            arguments,
            id: match id {
                ParsedToolCallId::ModelProvided(model_provided_id) => {
                    model_provided_id.as_str().to_owned()
                }
                ParsedToolCallId::NotProvidedByModel => format!("call_{index}"),
            },
            index,
            name,
        }
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings_types::ModelProvidedToolCallId;
    use llama_cpp_bindings_types::ParsedToolCall;
    use llama_cpp_bindings_types::ParsedToolCallId;
    use llama_cpp_bindings_types::ToolCallArguments;
    use serde_json::json;

    use super::GeneratedToolCall;

    fn weather_call(id: ParsedToolCallId) -> ParsedToolCall {
        ParsedToolCall::new(
            id,
            "get_weather".to_owned(),
            ToolCallArguments::ValidJson(json!({"location": "Paris"})),
        )
    }

    #[test]
    fn a_tool_call_the_model_gave_an_id_keeps_that_id() {
        assert_eq!(
            GeneratedToolCall::at_index(
                weather_call(ParsedToolCallId::ModelProvided(
                    ModelProvidedToolCallId::try_from("abc123def".to_owned()).unwrap(),
                )),
                3,
            )
            .id,
            "abc123def"
        );
    }

    #[test]
    fn a_tool_call_the_model_gave_no_id_is_named_after_its_index_in_the_generation() {
        assert_eq!(
            GeneratedToolCall::at_index(weather_call(ParsedToolCallId::NotProvidedByModel), 3),
            GeneratedToolCall {
                arguments: ToolCallArguments::ValidJson(json!({"location": "Paris"})),
                id: "call_3".to_owned(),
                index: 3,
                name: "get_weather".to_owned(),
            }
        );
    }
}
