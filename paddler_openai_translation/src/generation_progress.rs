use paddler_messaging::generated_token_partial_result::GeneratedTokenPartialResult;
use paddler_messaging::raw_tool_call_tokens::RawToolCallTokens;

use crate::generated_output_part::GeneratedOutputPart;
use crate::generation_failure::GenerationFailure;
use crate::generation_failure_cause::GenerationFailureCause;

fn unrecognized_tool_call_format_message(
    RawToolCallTokens {
        ffi_error_message,
        text,
    }: &RawToolCallTokens,
) -> String {
    format!(
        "model produced output the parser did not recognise as any registered tool-call format; \
         FFI error: {ffi_error_message}; raw text: {text}"
    )
}

#[derive(Debug, Eq, PartialEq)]
pub enum GenerationProgress {
    Failed(GenerationFailure),
    Produced(GeneratedOutputPart),
    ToolCallTokenProduced,
}

impl GenerationProgress {
    const fn agent_failed(message: String) -> Self {
        Self::Failed(GenerationFailure {
            cause: GenerationFailureCause::AgentFailed,
            message,
        })
    }
}

impl From<GeneratedTokenPartialResult> for GenerationProgress {
    fn from(generated_token_partial_result: GeneratedTokenPartialResult) -> Self {
        match generated_token_partial_result {
            GeneratedTokenPartialResult::ContentToken(text)
            | GeneratedTokenPartialResult::UndeterminableToken(text) => {
                Self::Produced(GeneratedOutputPart::Content(text))
            }
            GeneratedTokenPartialResult::ReasoningToken(text) => {
                Self::Produced(GeneratedOutputPart::Reasoning(text))
            }
            GeneratedTokenPartialResult::ToolCallParsed(parsed_calls) => {
                Self::Produced(GeneratedOutputPart::ToolCalls(parsed_calls))
            }
            GeneratedTokenPartialResult::ToolCallToken(_) => Self::ToolCallTokenProduced,
            GeneratedTokenPartialResult::ToolCallParseFailed(description) => {
                Self::agent_failed(description)
            }
            GeneratedTokenPartialResult::ToolCallValidationFailed(validation_errors) => {
                Self::agent_failed(validation_errors.join("; "))
            }
            GeneratedTokenPartialResult::UnrecognizedToolCallFormat(raw_tool_call_tokens) => {
                Self::agent_failed(unrecognized_tool_call_format_message(&raw_tool_call_tokens))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings_types::ToolCallArguments;
    use serde_json::json;

    use paddler_messaging::generated_token_partial_result::GeneratedTokenPartialResult;
    use paddler_messaging::generated_tool_call::GeneratedToolCall;
    use paddler_messaging::raw_tool_call_tokens::RawToolCallTokens;

    use super::GenerationProgress;
    use crate::generated_output_part::GeneratedOutputPart;
    use crate::generation_failure::GenerationFailure;
    use crate::generation_failure_cause::GenerationFailureCause;

    fn weather_call() -> GeneratedToolCall {
        GeneratedToolCall {
            arguments: ToolCallArguments::ValidJson(json!({ "location": "Paris" })),
            id: "call_x".to_owned(),
            index: 0,
            name: "get_weather".to_owned(),
        }
    }

    fn agent_failed(message: &str) -> GenerationProgress {
        GenerationProgress::Failed(GenerationFailure {
            cause: GenerationFailureCause::AgentFailed,
            message: message.to_owned(),
        })
    }

    #[test]
    fn classifies_tokens_and_parsed_tool_calls_as_produced_output() {
        for (generated_token_partial_result, expected_progress) in [
            (
                GeneratedTokenPartialResult::ContentToken("hello".to_owned()),
                GenerationProgress::Produced(GeneratedOutputPart::Content("hello".to_owned())),
            ),
            (
                GeneratedTokenPartialResult::UndeterminableToken("ambig".to_owned()),
                GenerationProgress::Produced(GeneratedOutputPart::Content("ambig".to_owned())),
            ),
            (
                GeneratedTokenPartialResult::ReasoningToken("thought".to_owned()),
                GenerationProgress::Produced(GeneratedOutputPart::Reasoning("thought".to_owned())),
            ),
            (
                GeneratedTokenPartialResult::ToolCallParsed(vec![weather_call()]),
                GenerationProgress::Produced(GeneratedOutputPart::ToolCalls(vec![weather_call()])),
            ),
            (
                GeneratedTokenPartialResult::ToolCallToken("{".to_owned()),
                GenerationProgress::ToolCallTokenProduced,
            ),
        ] {
            assert_eq!(
                GenerationProgress::from(generated_token_partial_result),
                expected_progress
            );
        }
    }

    #[test]
    fn classifies_tool_call_failures_as_agent_failures() {
        for (generated_token_partial_result, expected_progress) in [
            (
                GeneratedTokenPartialResult::ToolCallParseFailed("agent failure".to_owned()),
                agent_failed("agent failure"),
            ),
            (
                GeneratedTokenPartialResult::ToolCallValidationFailed(vec![
                    "first issue".to_owned(),
                    "second issue".to_owned(),
                ]),
                agent_failed("first issue; second issue"),
            ),
            (
                GeneratedTokenPartialResult::UnrecognizedToolCallFormat(RawToolCallTokens {
                    text: "<unknown_marker>blah</unknown_marker>".to_owned(),
                    ffi_error_message: "common_chat_parse failed: no parser".to_owned(),
                }),
                agent_failed(
                    "model produced output the parser did not recognise as any registered tool-call format; FFI error: common_chat_parse failed: no parser; raw text: <unknown_marker>blah</unknown_marker>",
                ),
            ),
        ] {
            assert_eq!(
                GenerationProgress::from(generated_token_partial_result),
                expected_progress
            );
        }
    }
}
