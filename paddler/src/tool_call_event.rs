use llama_cpp_bindings::ParsedToolCall;
use paddler_types::generated_token_result::GeneratedTokenResult;

use crate::tool_call_pipeline_error::ToolCallPipelineError;
use crate::tool_call_validation_error::ToolCallValidationError;

#[derive(Debug)]
pub enum ToolCallEvent {
    Pending,
    Resolved(Vec<ParsedToolCall>),
    ParseFailed(ToolCallPipelineError),
    ValidationFailed(Vec<ToolCallValidationError>),
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
            Self::Pending => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use anyhow::bail;
    use llama_cpp_bindings::ParsedToolCall;
    use llama_cpp_bindings::ToolCallArguments;
    use paddler_types::generated_token_result::GeneratedTokenResult;
    use serde_json::json;

    use super::ToolCallEvent;
    use crate::tool_call_pipeline_error::ToolCallPipelineError;
    use crate::tool_call_validation_error::ToolCallValidationError;

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
    fn resolved_converts_to_tool_call_parsed() -> Result<()> {
        let parsed = ParsedToolCall::new(
            "id".to_owned(),
            "tool".to_owned(),
            ToolCallArguments::ValidJson(json!({})),
        );
        let event = ToolCallEvent::Resolved(vec![parsed.clone()]);

        match event.into_generated_token_result() {
            Some(GeneratedTokenResult::ToolCallParsed(calls)) if calls == vec![parsed] => Ok(()),
            other => bail!("expected ToolCallParsed with one call, got {other:?}"),
        }
    }

    #[test]
    fn parse_failed_converts_to_tool_call_parse_failed_with_message() -> Result<()> {
        let event = ToolCallEvent::ParseFailed(ToolCallPipelineError::EmptyBuffer);

        match event.into_generated_token_result() {
            Some(GeneratedTokenResult::ToolCallParseFailed(message)) if !message.is_empty() => {
                Ok(())
            }
            other => bail!("expected ToolCallParseFailed with non-empty message, got {other:?}"),
        }
    }

    #[test]
    fn validation_failed_converts_to_tool_call_validation_failed_with_messages() -> Result<()> {
        let event =
            ToolCallEvent::ValidationFailed(vec![ToolCallValidationError::UnknownToolName(
                "missing".to_owned(),
            )]);

        match event.into_generated_token_result() {
            Some(GeneratedTokenResult::ToolCallValidationFailed(messages))
                if messages.len() == 1 && messages[0].contains("missing") =>
            {
                Ok(())
            }
            other => bail!("expected ToolCallValidationFailed mentioning 'missing', got {other:?}"),
        }
    }
}
