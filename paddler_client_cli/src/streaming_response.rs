use llama_cpp_bindings_types::ParsedToolCall;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::generation_summary::GenerationSummary;
use paddler_types::inference_client::Message;
use paddler_types::inference_client::Response;

use crate::stop_reason::StopReason;

#[derive(Debug, Default)]
pub struct StreamingResponse {
    pub thinking: String,
    pub response: String,
    pub tool_calls: Vec<ParsedToolCall>,
    pub pending_tool_call_buffer: String,
    pub undetermined: String,
    pub summary: Option<GenerationSummary>,
    pub stop_reason: Option<StopReason>,
}

impl StreamingResponse {
    pub fn apply_message(&mut self, message: Message) {
        match message {
            Message::Error(envelope) => {
                self.stop_reason = Some(StopReason::InferenceError {
                    code: envelope.error.code,
                    description: envelope.error.description,
                });
            }
            Message::Response(envelope) => self.apply_response(envelope.response),
        }
    }

    pub fn record_wire_error(&mut self, error: &anyhow::Error) {
        self.stop_reason = Some(StopReason::WireStreamError(error.to_string()));
    }

    pub const fn is_finished(&self) -> bool {
        self.stop_reason.is_some()
    }

    fn apply_response(&mut self, response: Response) {
        match response {
            Response::GeneratedToken(token_result) => self.apply_token_result(token_result),
            Response::Timeout => {
                self.stop_reason = Some(StopReason::Timeout);
            }
            Response::TooManyBufferedRequests => {
                self.stop_reason = Some(StopReason::TooManyBufferedRequests);
            }
            Response::Embedding(_) => {
                unreachable!("server sent an embedding response on a token-generation stream")
            }
        }
    }

    fn apply_token_result(&mut self, token_result: GeneratedTokenResult) {
        match token_result {
            GeneratedTokenResult::ContentToken(piece) => self.response.push_str(&piece),
            GeneratedTokenResult::ReasoningToken(piece) => self.thinking.push_str(&piece),
            GeneratedTokenResult::UndeterminableToken(piece) => self.undetermined.push_str(&piece),
            GeneratedTokenResult::ToolCallToken(piece) => {
                self.pending_tool_call_buffer.push_str(&piece);
            }
            GeneratedTokenResult::ToolCallParsed(calls) => {
                self.pending_tool_call_buffer.clear();
                self.tool_calls.extend(calls);
            }
            GeneratedTokenResult::Done(summary) => {
                self.summary = Some(summary);
                self.stop_reason = Some(StopReason::Completed);
            }
            GeneratedTokenResult::ChatTemplateError(detail) => {
                self.stop_reason = Some(StopReason::ChatTemplateError(detail));
            }
            GeneratedTokenResult::GrammarIncompatibleWithThinking(detail) => {
                self.stop_reason = Some(StopReason::GrammarIncompatibleWithThinking(detail));
            }
            GeneratedTokenResult::GrammarInitializationFailed(detail) => {
                self.stop_reason = Some(StopReason::GrammarInitializationFailed(detail));
            }
            GeneratedTokenResult::GrammarRejectedModelOutput(detail) => {
                self.stop_reason = Some(StopReason::GrammarRejectedModelOutput(detail));
            }
            GeneratedTokenResult::GrammarSyntaxError(detail) => {
                self.stop_reason = Some(StopReason::GrammarSyntaxError(detail));
            }
            GeneratedTokenResult::ImageDecodingFailed(detail) => {
                self.stop_reason = Some(StopReason::ImageDecodingFailed(detail));
            }
            GeneratedTokenResult::MultimodalNotSupported(detail) => {
                self.stop_reason = Some(StopReason::MultimodalNotSupported(detail));
            }
            GeneratedTokenResult::SamplerError(detail) => {
                self.stop_reason = Some(StopReason::SamplerError(detail));
            }
            GeneratedTokenResult::ToolCallParseFailed(detail) => {
                self.stop_reason = Some(StopReason::ToolCallParseFailed(detail));
            }
            GeneratedTokenResult::ToolCallValidationFailed(field_errors) => {
                self.stop_reason = Some(StopReason::ToolCallValidationFailed(field_errors));
            }
            GeneratedTokenResult::ToolSchemaInvalid(detail) => {
                self.stop_reason = Some(StopReason::ToolSchemaInvalid(detail));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;
    use paddler_types::jsonrpc::Error;
    use paddler_types::jsonrpc::ErrorEnvelope;
    use paddler_types::jsonrpc::ResponseEnvelope;

    use super::*;

    fn token_message(token_result: GeneratedTokenResult) -> Message {
        Message::Response(ResponseEnvelope {
            request_id: "test-request".to_owned(),
            response: Response::GeneratedToken(token_result),
        })
    }

    #[test]
    fn content_token_appends_to_response_buffer() {
        let mut state = StreamingResponse::default();

        state.apply_message(token_message(GeneratedTokenResult::ContentToken(
            "hello ".to_owned(),
        )));
        state.apply_message(token_message(GeneratedTokenResult::ContentToken(
            "world".to_owned(),
        )));

        assert_eq!(state.response, "hello world");
        assert!(state.thinking.is_empty());
        assert!(state.undetermined.is_empty());
        assert!(!state.is_finished());
    }

    #[test]
    fn raw_tool_call_token_appends_to_pending_buffer() {
        let mut state = StreamingResponse::default();

        state.apply_message(token_message(GeneratedTokenResult::ToolCallToken(
            "{\"name\":".to_owned(),
        )));
        state.apply_message(token_message(GeneratedTokenResult::ToolCallToken(
            "\"calc\"}".to_owned(),
        )));

        assert_eq!(state.pending_tool_call_buffer, "{\"name\":\"calc\"}");
        assert!(state.tool_calls.is_empty());
    }

    #[test]
    fn tool_call_parsed_replaces_pending_buffer_with_structured_calls() {
        let mut state = StreamingResponse::default();
        state.apply_message(token_message(GeneratedTokenResult::ToolCallToken(
            "{\"name\":\"calc\"}".to_owned(),
        )));
        let parsed = vec![ParsedToolCall::default()];

        state.apply_message(token_message(GeneratedTokenResult::ToolCallParsed(
            parsed.clone(),
        )));

        assert_eq!(state.tool_calls, parsed);
        assert!(state.pending_tool_call_buffer.is_empty());
    }

    #[test]
    fn done_records_summary_and_completed_stop_reason() {
        let mut state = StreamingResponse::default();
        let summary = GenerationSummary::default();

        state.apply_message(token_message(GeneratedTokenResult::Done(summary)));

        assert!(state.summary.is_some());
        assert!(matches!(state.stop_reason, Some(StopReason::Completed)));
        assert!(state.is_finished());
    }

    #[test]
    fn message_error_sets_inference_error_stop_reason() {
        let mut state = StreamingResponse::default();

        state.apply_message(Message::Error(ErrorEnvelope {
            request_id: "test-request".to_owned(),
            error: Error {
                code: 503,
                description: "agent unavailable".to_owned(),
            },
        }));

        assert!(matches!(
            state.stop_reason,
            Some(StopReason::InferenceError { code: 503, .. })
        ));
    }

    #[test]
    fn wire_error_sets_wire_stream_error_stop_reason() {
        let mut state = StreamingResponse::default();

        state.record_wire_error(&anyhow!("connection reset"));

        assert!(matches!(
            state.stop_reason,
            Some(StopReason::WireStreamError(ref message)) if message.contains("connection reset")
        ));
    }
}
