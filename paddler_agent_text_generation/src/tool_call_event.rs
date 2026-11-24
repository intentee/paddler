use llama_cpp_chat::chat_error::ChatError;

use paddler_error_chain::error_chain::ErrorChain;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::generated_tool_call::GeneratedToolCall;
use paddler_messaging::raw_tool_call_tokens::RawToolCallTokens;
use paddler_tool_call_validator::tool_call_validation_error::ToolCallValidationError;

#[derive(Debug)]
pub enum ToolCallEvent<'reading> {
    Resolved(Vec<GeneratedToolCall>),
    ParseFailed(&'reading ChatError),
    ValidationFailed(Vec<ToolCallValidationError>),
    UnrecognizedFormat(RawToolCallTokens),
}

impl ToolCallEvent<'_> {
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
                        .map(|validation_error| {
                            ErrorChain {
                                error: &validation_error,
                            }
                            .to_string()
                        })
                        .collect(),
                )
            }
            Self::UnrecognizedFormat(raw) => GeneratedTokenResult::UnrecognizedToolCallFormat(raw),
        }
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_chat::chat_error::ChatError;
    use serde_json::json;

    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::generated_tool_call::GeneratedToolCall;
    use paddler_messaging::raw_tool_call_tokens::RawToolCallTokens;
    use paddler_messaging::tool_call_arguments::ToolCallArguments;
    use paddler_tool_call_validator::tool_call_validation_error::ToolCallValidationError;

    use super::ToolCallEvent;

    #[test]
    fn resolved_converts_to_tool_call_parsed() {
        let generated = GeneratedToolCall {
            arguments: ToolCallArguments::ValidJson(json!({})),
            id: "id".to_owned(),
            index: 0,
            name: "tool".to_owned(),
        };

        assert_eq!(
            ToolCallEvent::Resolved(vec![generated.clone()]).into_generated_token_result(),
            GeneratedTokenResult::ToolCallParsed(vec![generated])
        );
    }

    #[test]
    fn parse_failed_converts_to_tool_call_parse_failed() {
        assert!(matches!(
            ToolCallEvent::ParseFailed(&ChatError::ParseNotEnoughMemory)
                .into_generated_token_result(),
            GeneratedTokenResult::ToolCallParseFailed(_)
        ));
    }

    #[test]
    fn validation_failed_converts_to_tool_call_validation_failed_for_every_violation() {
        assert!(matches!(
            ToolCallEvent::ValidationFailed(vec![
                ToolCallValidationError::UnknownToolName("missing".to_owned()),
                ToolCallValidationError::UnknownToolName("also_missing".to_owned()),
            ])
            .into_generated_token_result(),
            GeneratedTokenResult::ToolCallValidationFailed(violations) if violations.len() == 2
        ));
    }

    #[test]
    fn unrecognized_format_converts_to_unrecognized_tool_call_format_preserving_payload() {
        let raw = RawToolCallTokens {
            text: "raw output".to_owned(),
            expected_format: "peg-native".to_owned(),
        };

        assert_eq!(
            ToolCallEvent::UnrecognizedFormat(raw.clone()).into_generated_token_result(),
            GeneratedTokenResult::UnrecognizedToolCallFormat(raw)
        );
    }
}
