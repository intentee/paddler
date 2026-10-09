use llama_cpp_bindings::ParsedToolCall;
use llama_cpp_bindings::error::ParseChatMessageError;

use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::raw_tool_call_tokens::RawToolCallTokens;
use paddler_tool_call_validator::tool_call_validation_error::ToolCallValidationError;

#[derive(Debug)]
pub enum ToolCallEvent {
    Resolved(Vec<ParsedToolCall>),
    ParseFailed(ParseChatMessageError),
    ValidationFailed(Vec<ToolCallValidationError>),
    UnrecognizedFormat(RawToolCallTokens),
}

impl ToolCallEvent {
    #[must_use]
    pub fn into_generated_token_result(self) -> GeneratedTokenResult {
        match self {
            Self::Resolved(parsed) => GeneratedTokenResult::ToolCallParsed(parsed),
            Self::ParseFailed(parse_error) => {
                GeneratedTokenResult::ToolCallParseFailed(parse_error.to_string())
            }
            Self::ValidationFailed(validation_errors) => {
                GeneratedTokenResult::ToolCallValidationFailed(
                    validation_errors
                        .into_iter()
                        .map(|validation_error| validation_error.to_string())
                        .collect(),
                )
            }
            Self::UnrecognizedFormat(raw) => GeneratedTokenResult::UnrecognizedToolCallFormat(raw),
        }
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::ParsedToolCall;
    use llama_cpp_bindings::ToolCallArguments;
    use llama_cpp_bindings::error::ParseChatMessageError;
    use serde_json::json;

    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::raw_tool_call_tokens::RawToolCallTokens;
    use paddler_tool_call_validator::tool_call_validation_error::ToolCallValidationError;

    use super::ToolCallEvent;

    #[test]
    fn resolved_converts_to_tool_call_parsed() {
        let parsed = ParsedToolCall::new(
            "id".to_owned(),
            "tool".to_owned(),
            ToolCallArguments::ValidJson(json!({})),
        );

        assert_eq!(
            ToolCallEvent::Resolved(vec![parsed.clone()]).into_generated_token_result(),
            GeneratedTokenResult::ToolCallParsed(vec![parsed])
        );
    }

    #[test]
    fn parse_failed_converts_to_tool_call_parse_failed_with_the_parser_error() {
        assert_eq!(
            ToolCallEvent::ParseFailed(ParseChatMessageError::ToolsNotAnArray)
                .into_generated_token_result(),
            GeneratedTokenResult::ToolCallParseFailed(
                ParseChatMessageError::ToolsNotAnArray.to_string()
            )
        );
    }

    #[test]
    fn validation_failed_converts_to_tool_call_validation_failed_with_every_message() {
        let validation_error = ToolCallValidationError::UnknownToolName("missing".to_owned());
        let expected_message = validation_error.to_string();

        assert_eq!(
            ToolCallEvent::ValidationFailed(vec![validation_error]).into_generated_token_result(),
            GeneratedTokenResult::ToolCallValidationFailed(vec![expected_message])
        );
    }

    #[test]
    fn unrecognized_format_converts_to_unrecognized_tool_call_format_preserving_payload() {
        let raw = RawToolCallTokens {
            text: "raw output".to_owned(),
            ffi_error_message: "parser bailed".to_owned(),
        };

        assert_eq!(
            ToolCallEvent::UnrecognizedFormat(raw.clone()).into_generated_token_result(),
            GeneratedTokenResult::UnrecognizedToolCallFormat(raw)
        );
    }
}
