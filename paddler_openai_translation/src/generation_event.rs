use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::generation_summary::GenerationSummary;
use paddler_messaging::oversized_media_details::OversizedMediaDetails;
use paddler_messaging::oversized_prompt_details::OversizedPromptDetails;
use paddler_messaging::raw_tool_call_tokens::RawToolCallTokens;

use crate::generated_output_part::GeneratedOutputPart;
use crate::generation_failure::GenerationFailure;
use crate::generation_failure_cause::GenerationFailureCause;

fn media_exceeds_micro_batch_message(
    OversizedMediaDetails {
        media_tokens,
        micro_batch_tokens,
    }: &OversizedMediaDetails,
) -> String {
    format!(
        "media required {media_tokens} tokens but one agent micro batch holds {micro_batch_tokens} tokens"
    )
}

fn prompt_exceeds_context_size_message(
    OversizedPromptDetails {
        prompt_tokens,
        sequence_context_size,
    }: &OversizedPromptDetails,
) -> String {
    format!(
        "prompt has {prompt_tokens} tokens but each agent sequence holds {sequence_context_size} tokens; shorten the prompt or raise context_size"
    )
}

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
pub enum GenerationEvent {
    Failed(GenerationFailure),
    Finished(GenerationSummary),
    Produced(GeneratedOutputPart),
    ToolCallTokenProduced,
}

impl GenerationEvent {
    const fn failed(cause: GenerationFailureCause, message: String) -> Self {
        Self::Failed(GenerationFailure { cause, message })
    }
}

impl From<GeneratedTokenResult> for GenerationEvent {
    fn from(generated_token_result: GeneratedTokenResult) -> Self {
        match generated_token_result {
            GeneratedTokenResult::ContentToken(text)
            | GeneratedTokenResult::UndeterminableToken(text) => {
                Self::Produced(GeneratedOutputPart::Content(text))
            }
            GeneratedTokenResult::ReasoningToken(text) => {
                Self::Produced(GeneratedOutputPart::Reasoning(text))
            }
            GeneratedTokenResult::ToolCallParsed(parsed_calls) => {
                Self::Produced(GeneratedOutputPart::ToolCalls(parsed_calls))
            }
            GeneratedTokenResult::ToolCallToken(_) => Self::ToolCallTokenProduced,
            GeneratedTokenResult::Done(generation_summary) => Self::Finished(generation_summary),
            GeneratedTokenResult::MediaExceedsMicroBatch(oversized_media_details) => Self::failed(
                GenerationFailureCause::InvalidRequest,
                media_exceeds_micro_batch_message(&oversized_media_details),
            ),
            GeneratedTokenResult::PromptExceedsContextSize(oversized_prompt_details) => {
                Self::failed(
                    GenerationFailureCause::InvalidRequest,
                    prompt_exceeds_context_size_message(&oversized_prompt_details),
                )
            }
            GeneratedTokenResult::GrammarIncompatibleWithThinking(description)
            | GeneratedTokenResult::GrammarSyntaxError(description)
            | GeneratedTokenResult::ImageDecodingFailed(description)
            | GeneratedTokenResult::MultimodalNotSupported(description)
            | GeneratedTokenResult::MultimodalTokenizationFailed(description)
            | GeneratedTokenResult::PromptTokenizationFailed(description)
            | GeneratedTokenResult::ToolSchemaInvalid(description) => {
                Self::failed(GenerationFailureCause::InvalidRequest, description)
            }
            GeneratedTokenResult::InferenceModeMismatch(description)
            | GeneratedTokenResult::ModelNotLoaded(description)
            | GeneratedTokenResult::NoSequenceSlotAvailable(description)
            | GeneratedTokenResult::SchedulerUnavailable(description) => {
                Self::failed(GenerationFailureCause::Unavailable, description)
            }
            GeneratedTokenResult::ToolCallValidationFailed(validation_errors) => Self::failed(
                GenerationFailureCause::AgentFailed,
                validation_errors.join("; "),
            ),
            GeneratedTokenResult::UnrecognizedToolCallFormat(raw_tool_call_tokens) => Self::failed(
                GenerationFailureCause::AgentFailed,
                unrecognized_tool_call_format_message(&raw_tool_call_tokens),
            ),
            GeneratedTokenResult::BatchAssemblyFailed(description)
            | GeneratedTokenResult::ChatTemplateError(description)
            | GeneratedTokenResult::DecodeFailed(description)
            | GeneratedTokenResult::DetokenizationFailed(description)
            | GeneratedTokenResult::GrammarInitializationFailed(description)
            | GeneratedTokenResult::GrammarRejectedModelOutput(description)
            | GeneratedTokenResult::KvCacheClearFailed(description)
            | GeneratedTokenResult::MediaMicroBatchCheckFailed(description)
            | GeneratedTokenResult::MultimodalIngestionFailed(description)
            | GeneratedTokenResult::SamplerChainCreationFailed(description)
            | GeneratedTokenResult::SamplerError(description)
            | GeneratedTokenResult::SamplingCandidatesExhausted(description)
            | GeneratedTokenResult::SequenceIdOutOfRange(description)
            | GeneratedTokenResult::ToolCallParseFailed(description)
            | GeneratedTokenResult::ToolsSerializationFailed(description) => {
                Self::failed(GenerationFailureCause::AgentFailed, description)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings_types::ParsedToolCall;
    use llama_cpp_bindings_types::TokenUsage;
    use llama_cpp_bindings_types::ToolCallArguments;
    use serde_json::json;

    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::generation_finish::GenerationFinish;
    use paddler_messaging::generation_summary::GenerationSummary;
    use paddler_messaging::oversized_media_details::OversizedMediaDetails;
    use paddler_messaging::oversized_prompt_details::OversizedPromptDetails;
    use paddler_messaging::raw_tool_call_tokens::RawToolCallTokens;

    use super::GenerationEvent;
    use crate::generated_output_part::GeneratedOutputPart;
    use crate::generation_failure::GenerationFailure;
    use crate::generation_failure_cause::GenerationFailureCause;

    fn weather_call() -> ParsedToolCall {
        ParsedToolCall::new(
            "call_x".to_owned(),
            "get_weather".to_owned(),
            ToolCallArguments::ValidJson(json!({ "location": "Paris" })),
        )
    }

    fn assert_failures_classified_as(
        expected_cause: GenerationFailureCause,
        failures: Vec<(GeneratedTokenResult, &str)>,
    ) {
        for (generated_token_result, expected_message) in failures {
            assert_eq!(
                GenerationEvent::from(generated_token_result),
                GenerationEvent::Failed(GenerationFailure {
                    cause: expected_cause,
                    message: expected_message.to_owned(),
                })
            );
        }
    }

    #[test]
    fn classifies_tokens_and_the_summary_as_generation_progress() {
        let generation_summary = GenerationSummary {
            finish: GenerationFinish::EndOfGeneration,
            usage: TokenUsage::new(),
        };

        for (generated_token_result, expected_event) in [
            (
                GeneratedTokenResult::ContentToken("hello".to_owned()),
                GenerationEvent::Produced(GeneratedOutputPart::Content("hello".to_owned())),
            ),
            (
                GeneratedTokenResult::UndeterminableToken("ambig".to_owned()),
                GenerationEvent::Produced(GeneratedOutputPart::Content("ambig".to_owned())),
            ),
            (
                GeneratedTokenResult::ReasoningToken("thought".to_owned()),
                GenerationEvent::Produced(GeneratedOutputPart::Reasoning("thought".to_owned())),
            ),
            (
                GeneratedTokenResult::ToolCallParsed(vec![weather_call()]),
                GenerationEvent::Produced(GeneratedOutputPart::ToolCalls(vec![weather_call()])),
            ),
            (
                GeneratedTokenResult::ToolCallToken("{".to_owned()),
                GenerationEvent::ToolCallTokenProduced,
            ),
            (
                GeneratedTokenResult::Done(generation_summary),
                GenerationEvent::Finished(generation_summary),
            ),
        ] {
            assert_eq!(
                GenerationEvent::from(generated_token_result),
                expected_event
            );
        }
    }

    #[test]
    fn classifies_client_mistakes_as_invalid_requests() {
        assert_failures_classified_as(
            GenerationFailureCause::InvalidRequest,
            vec![
                (
                    GeneratedTokenResult::GrammarIncompatibleWithThinking(
                        "client mistake".to_owned(),
                    ),
                    "client mistake",
                ),
                (
                    GeneratedTokenResult::GrammarSyntaxError("client mistake".to_owned()),
                    "client mistake",
                ),
                (
                    GeneratedTokenResult::ImageDecodingFailed("client mistake".to_owned()),
                    "client mistake",
                ),
                (
                    GeneratedTokenResult::MediaExceedsMicroBatch(OversizedMediaDetails {
                        media_tokens: 256,
                        micro_batch_tokens: 128,
                    }),
                    "media required 256 tokens but one agent micro batch holds 128 tokens",
                ),
                (
                    GeneratedTokenResult::MultimodalNotSupported("client mistake".to_owned()),
                    "client mistake",
                ),
                (
                    GeneratedTokenResult::MultimodalTokenizationFailed("client mistake".to_owned()),
                    "client mistake",
                ),
                (
                    GeneratedTokenResult::PromptExceedsContextSize(OversizedPromptDetails {
                        prompt_tokens: 9895,
                        sequence_context_size: 8192,
                    }),
                    "prompt has 9895 tokens but each agent sequence holds 8192 tokens; shorten the prompt or raise context_size",
                ),
                (
                    GeneratedTokenResult::PromptTokenizationFailed("client mistake".to_owned()),
                    "client mistake",
                ),
                (
                    GeneratedTokenResult::ToolSchemaInvalid("client mistake".to_owned()),
                    "client mistake",
                ),
            ],
        );
    }

    #[test]
    fn classifies_agents_that_cannot_serve_as_unavailable() {
        assert_failures_classified_as(
            GenerationFailureCause::Unavailable,
            vec![
                (
                    GeneratedTokenResult::InferenceModeMismatch("unavailable".to_owned()),
                    "unavailable",
                ),
                (
                    GeneratedTokenResult::ModelNotLoaded("unavailable".to_owned()),
                    "unavailable",
                ),
                (
                    GeneratedTokenResult::NoSequenceSlotAvailable("unavailable".to_owned()),
                    "unavailable",
                ),
                (
                    GeneratedTokenResult::SchedulerUnavailable("unavailable".to_owned()),
                    "unavailable",
                ),
            ],
        );
    }

    #[test]
    fn classifies_agent_failures_as_agent_failed() {
        assert_failures_classified_as(
            GenerationFailureCause::AgentFailed,
            vec![
                (
                    GeneratedTokenResult::BatchAssemblyFailed("agent failure".to_owned()),
                    "agent failure",
                ),
                (
                    GeneratedTokenResult::ChatTemplateError("agent failure".to_owned()),
                    "agent failure",
                ),
                (
                    GeneratedTokenResult::DecodeFailed("agent failure".to_owned()),
                    "agent failure",
                ),
                (
                    GeneratedTokenResult::DetokenizationFailed("agent failure".to_owned()),
                    "agent failure",
                ),
                (
                    GeneratedTokenResult::GrammarInitializationFailed("agent failure".to_owned()),
                    "agent failure",
                ),
                (
                    GeneratedTokenResult::GrammarRejectedModelOutput("agent failure".to_owned()),
                    "agent failure",
                ),
                (
                    GeneratedTokenResult::KvCacheClearFailed("agent failure".to_owned()),
                    "agent failure",
                ),
                (
                    GeneratedTokenResult::MediaMicroBatchCheckFailed("agent failure".to_owned()),
                    "agent failure",
                ),
                (
                    GeneratedTokenResult::MultimodalIngestionFailed("agent failure".to_owned()),
                    "agent failure",
                ),
                (
                    GeneratedTokenResult::SamplerChainCreationFailed("agent failure".to_owned()),
                    "agent failure",
                ),
                (
                    GeneratedTokenResult::SamplerError("agent failure".to_owned()),
                    "agent failure",
                ),
                (
                    GeneratedTokenResult::SamplingCandidatesExhausted("agent failure".to_owned()),
                    "agent failure",
                ),
                (
                    GeneratedTokenResult::SequenceIdOutOfRange("agent failure".to_owned()),
                    "agent failure",
                ),
                (
                    GeneratedTokenResult::ToolCallParseFailed("agent failure".to_owned()),
                    "agent failure",
                ),
                (
                    GeneratedTokenResult::ToolCallValidationFailed(vec![
                        "first issue".to_owned(),
                        "second issue".to_owned(),
                    ]),
                    "first issue; second issue",
                ),
                (
                    GeneratedTokenResult::ToolsSerializationFailed("agent failure".to_owned()),
                    "agent failure",
                ),
                (
                    GeneratedTokenResult::UnrecognizedToolCallFormat(RawToolCallTokens {
                        text: "<unknown_marker>blah</unknown_marker>".to_owned(),
                        ffi_error_message: "common_chat_parse failed: no parser".to_owned(),
                    }),
                    "model produced output the parser did not recognise as any registered tool-call format; FFI error: common_chat_parse failed: no parser; raw text: <unknown_marker>blah</unknown_marker>",
                ),
            ],
        );
    }
}
