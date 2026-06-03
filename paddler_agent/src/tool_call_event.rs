use llama_cpp_bindings::ParsedToolCall;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::raw_tool_call_tokens::RawToolCallTokens;

use crate::tool_call_pipeline_error::ToolCallPipelineError;
use paddler_messaging::tool_call_validation_error::ToolCallValidationError;

#[derive(Debug)]
pub enum ToolCallEvent {
    Pending,
    Resolved(Vec<ParsedToolCall>),
    ParseFailed(ToolCallPipelineError),
    ValidationFailed(Vec<ToolCallValidationError>),
    UnrecognizedFormat(RawToolCallTokens),
}

impl ToolCallEvent {
    #[must_use]
    pub const fn is_resolved(&self) -> bool {
        matches!(self, Self::Resolved(_))
    }

    #[must_use]
    pub const fn is_failure(&self) -> bool {
        matches!(self, Self::ParseFailed(_) | Self::ValidationFailed(_))
    }

    #[must_use]
    pub const fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }

    #[must_use]
    pub fn into_generated_token_result(self) -> Option<GeneratedTokenResult> {
        match self {
            Self::Resolved(parsed) => Some(GeneratedTokenResult::ToolCallParsed(parsed)),
            Self::ParseFailed(err) => {
                Some(GeneratedTokenResult::ToolCallParseFailed(err.to_string()))
            }
            Self::ValidationFailed(errors) => Some(GeneratedTokenResult::ToolCallValidationFailed(
                errors.into_iter().map(|err| err.to_string()).collect(),
            )),
            Self::UnrecognizedFormat(raw) => {
                Some(GeneratedTokenResult::UnrecognizedToolCallFormat(raw))
            }
            Self::Pending => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::ParsedToolCall;
    use llama_cpp_bindings::ToolCallArguments;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::raw_tool_call_tokens::RawToolCallTokens;
    use serde_json::json;

    use super::ToolCallEvent;
    use crate::tool_call_pipeline_error::ToolCallPipelineError;
    use paddler_messaging::tool_call_validation_error::ToolCallValidationError;

    #[test]
    fn pending_classifies_as_pending() {
        let event = ToolCallEvent::Pending;

        assert!(event.is_pending());
        assert!(!event.is_resolved());
        assert!(!event.is_failure());
    }

    #[test]
    fn resolved_classifies_as_resolved() {
        let event = ToolCallEvent::Resolved(vec![ParsedToolCall::default()]);

        assert!(event.is_resolved());
        assert!(!event.is_pending());
        assert!(!event.is_failure());
    }

    #[test]
    fn parse_failed_classifies_as_failure() {
        let event = ToolCallEvent::ParseFailed(ToolCallPipelineError::EmptyBuffer);

        assert!(event.is_failure());
        assert!(!event.is_resolved());
        assert!(!event.is_pending());
    }

    #[test]
    fn validation_failed_classifies_as_failure() {
        let event =
            ToolCallEvent::ValidationFailed(vec![ToolCallValidationError::UnknownToolName(
                "x".to_owned(),
            )]);

        assert!(event.is_failure());
        assert!(!event.is_resolved());
    }

    #[test]
    fn pending_converts_to_none() {
        assert!(
            ToolCallEvent::Pending
                .into_generated_token_result()
                .is_none()
        );
    }

    #[test]
    fn resolved_converts_to_tool_call_parsed() {
        let parsed = ParsedToolCall::new(
            "id".to_owned(),
            "tool".to_owned(),
            ToolCallArguments::ValidJson(json!({})),
        );
        let event = ToolCallEvent::Resolved(vec![parsed.clone()]);

        let result = event
            .into_generated_token_result()
            .expect("Resolved must convert to Some");

        assert!(
            matches!(result, GeneratedTokenResult::ToolCallParsed(calls) if calls == vec![parsed])
        );
    }

    #[test]
    fn parse_failed_converts_to_tool_call_parse_failed_with_message() {
        let event = ToolCallEvent::ParseFailed(ToolCallPipelineError::EmptyBuffer);

        let result = event
            .into_generated_token_result()
            .expect("ParseFailed must convert to Some");

        assert!(matches!(
            result,
            GeneratedTokenResult::ToolCallParseFailed(message)
                if message == ToolCallPipelineError::EmptyBuffer.to_string()
        ));
    }

    #[test]
    fn validation_failed_converts_to_tool_call_validation_failed_with_messages() {
        let event =
            ToolCallEvent::ValidationFailed(vec![ToolCallValidationError::UnknownToolName(
                "missing".to_owned(),
            )]);

        let result = event
            .into_generated_token_result()
            .expect("ValidationFailed must convert to Some");

        assert!(matches!(
            result,
            GeneratedTokenResult::ToolCallValidationFailed(messages)
                if messages.len() == 1 && messages[0].contains("missing")
        ));
    }

    #[test]
    fn unrecognized_format_classifies_as_neither_resolved_nor_failure_nor_pending() {
        let event = ToolCallEvent::UnrecognizedFormat(RawToolCallTokens {
            text: "raw".to_owned(),
            ffi_error_message: "bailed".to_owned(),
        });

        assert!(!event.is_pending());
        assert!(!event.is_resolved());
        assert!(!event.is_failure());
    }

    #[test]
    fn unrecognized_format_converts_to_unrecognized_tool_call_format_preserving_payload() {
        let event = ToolCallEvent::UnrecognizedFormat(RawToolCallTokens {
            text: "raw output".to_owned(),
            ffi_error_message: "parser bailed".to_owned(),
        });

        let result = event
            .into_generated_token_result()
            .expect("UnrecognizedFormat must convert to Some");

        assert!(matches!(
            result,
            GeneratedTokenResult::UnrecognizedToolCallFormat(raw)
                if raw.text == "raw output" && raw.ffi_error_message == "parser bailed"
        ));
    }
}
