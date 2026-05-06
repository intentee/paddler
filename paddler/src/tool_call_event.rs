use paddler_types::parsed_tool_call::ParsedToolCall;

use crate::tool_call_parse_error::ToolCallParseError;
use crate::tool_call_validation_error::ToolCallValidationError;

#[derive(Debug)]
pub enum ToolCallEvent {
    Pending,
    Resolved(Vec<ParsedToolCall>),
    ParseFailed(ToolCallParseError),
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
}

#[cfg(test)]
mod tests {
    use paddler_types::parsed_tool_call::ParsedToolCall;

    use super::ToolCallEvent;
    use crate::tool_call_parse_error::ToolCallParseError;
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
        let event = ToolCallEvent::ParseFailed(ToolCallParseError::EmptyInput);

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
}
