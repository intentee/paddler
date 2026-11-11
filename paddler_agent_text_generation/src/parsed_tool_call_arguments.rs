use llama_cpp_bindings_types::invalid_arguments_json::InvalidArgumentsJson as ParsedInvalidArgumentsJson;
use llama_cpp_bindings_types::tool_call_arguments::ToolCallArguments as ParsedToolCallArgumentsValue;

use paddler_messaging::invalid_arguments_json::InvalidArgumentsJson;
use paddler_messaging::tool_call_arguments::ToolCallArguments;

pub struct ParsedToolCallArguments(pub ParsedToolCallArgumentsValue);

impl ParsedToolCallArguments {
    #[must_use]
    pub fn into_tool_call_arguments(self) -> ToolCallArguments {
        match self.0 {
            ParsedToolCallArgumentsValue::ValidJson(value) => ToolCallArguments::ValidJson(value),
            ParsedToolCallArgumentsValue::InvalidJson {
                raw,
                error:
                    ParsedInvalidArgumentsJson {
                        message,
                        line,
                        column,
                    },
            } => ToolCallArguments::InvalidJson {
                raw,
                error: InvalidArgumentsJson {
                    message,
                    line,
                    column,
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings_types::tool_call_arguments::ToolCallArguments as ParsedToolCallArgumentsValue;

    use paddler_messaging::invalid_arguments_json::InvalidArgumentsJson;
    use paddler_messaging::tool_call_arguments::ToolCallArguments;

    use super::ParsedToolCallArguments;

    #[test]
    fn unparseable_arguments_keep_their_text_and_where_parsing_failed() {
        assert_eq!(
            ParsedToolCallArguments(ParsedToolCallArgumentsValue::from_string(
                "{not really json".to_owned()
            ))
            .into_tool_call_arguments(),
            ToolCallArguments::InvalidJson {
                raw: "{not really json".to_owned(),
                error: InvalidArgumentsJson {
                    message: "key must be a string at line 1 column 2".to_owned(),
                    line: 1,
                    column: 2,
                },
            }
        );
    }
}
