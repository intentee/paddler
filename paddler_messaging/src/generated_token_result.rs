use serde::Deserialize;
use serde::Serialize;

use crate::generated_token_final_result::GeneratedTokenFinalResult;
use crate::generated_token_partial_result::GeneratedTokenPartialResult;
use crate::generated_tool_call::GeneratedToolCall;
use crate::generation_summary::GenerationSummary;
use crate::oversized_media_details::OversizedMediaDetails;
use crate::oversized_prompt_details::OversizedPromptDetails;
use crate::raw_tool_call_tokens::RawToolCallTokens;
use crate::result_step::ResultStep;
use crate::streamable_result::StreamableResult;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum GeneratedTokenResult {
    BatchAssemblyFailed(String),
    ChatTemplateError(String),
    ContentToken(String),
    DecodeFailed(String),
    DetokenizationFailed(String),
    Done(GenerationSummary),
    GrammarIncompatibleWithThinking(String),
    GrammarInitializationFailed(String),
    GrammarRejectedModelOutput(String),
    GrammarSyntaxError(String),
    ImageDecodingFailed(String),
    KvCacheClearFailed(String),
    MediaExceedsMicroBatch(OversizedMediaDetails),
    MediaMicroBatchCheckFailed(String),
    ModelNotLoaded(String),
    MultimodalIngestionFailed(String),
    MultimodalNotSupported(String),
    MultimodalTokenizationFailed(String),
    NoSequenceSlotAvailable(String),
    PromptExceedsContextSize(OversizedPromptDetails),
    PromptTokenizationFailed(String),
    PromptTokenizedToNothing(String),
    ReasoningToken(String),
    SamplerChainCreationFailed(String),
    SamplerError(String),
    SamplingCandidatesExhausted(String),
    SchedulerUnavailable(String),
    ToolCallParseFailed(String),
    ToolCallParsed(Vec<GeneratedToolCall>),
    ToolCallToken(String),
    ToolCallValidationFailed(Vec<String>),
    ToolSchemaInvalid(String),
    UndeterminableToken(String),
    UnrecognizedToolCallFormat(RawToolCallTokens),
}

impl GeneratedTokenResult {
    #[must_use]
    pub const fn is_token(&self) -> bool {
        matches!(
            self,
            Self::ContentToken(_)
                | Self::ReasoningToken(_)
                | Self::ToolCallToken(_)
                | Self::UndeterminableToken(_)
        )
    }

    #[must_use]
    pub fn token_text(&self) -> Option<&str> {
        match self {
            Self::ContentToken(text)
            | Self::ReasoningToken(text)
            | Self::ToolCallToken(text)
            | Self::UndeterminableToken(text) => Some(text),
            _ => None,
        }
    }

    #[must_use]
    pub const fn is_tool_call_parsed(&self) -> bool {
        matches!(self, Self::ToolCallParsed(_))
    }

    #[must_use]
    pub const fn is_tool_call_failure(&self) -> bool {
        matches!(
            self,
            Self::ToolCallParseFailed(_) | Self::ToolCallValidationFailed(_)
        )
    }
}

impl From<GeneratedTokenFinalResult> for GeneratedTokenResult {
    fn from(final_result: GeneratedTokenFinalResult) -> Self {
        match final_result {
            GeneratedTokenFinalResult::BatchAssemblyFailed(detail) => {
                Self::BatchAssemblyFailed(detail)
            }
            GeneratedTokenFinalResult::ChatTemplateError(detail) => Self::ChatTemplateError(detail),
            GeneratedTokenFinalResult::DecodeFailed(detail) => Self::DecodeFailed(detail),
            GeneratedTokenFinalResult::DetokenizationFailed(detail) => {
                Self::DetokenizationFailed(detail)
            }
            GeneratedTokenFinalResult::Done(summary) => Self::Done(summary),
            GeneratedTokenFinalResult::GrammarIncompatibleWithThinking(detail) => {
                Self::GrammarIncompatibleWithThinking(detail)
            }
            GeneratedTokenFinalResult::GrammarInitializationFailed(detail) => {
                Self::GrammarInitializationFailed(detail)
            }
            GeneratedTokenFinalResult::GrammarRejectedModelOutput(detail) => {
                Self::GrammarRejectedModelOutput(detail)
            }
            GeneratedTokenFinalResult::GrammarSyntaxError(detail) => {
                Self::GrammarSyntaxError(detail)
            }
            GeneratedTokenFinalResult::ImageDecodingFailed(detail) => {
                Self::ImageDecodingFailed(detail)
            }
            GeneratedTokenFinalResult::KvCacheClearFailed(detail) => {
                Self::KvCacheClearFailed(detail)
            }
            GeneratedTokenFinalResult::MediaExceedsMicroBatch(details) => {
                Self::MediaExceedsMicroBatch(details)
            }
            GeneratedTokenFinalResult::MediaMicroBatchCheckFailed(detail) => {
                Self::MediaMicroBatchCheckFailed(detail)
            }
            GeneratedTokenFinalResult::ModelNotLoaded(detail) => Self::ModelNotLoaded(detail),
            GeneratedTokenFinalResult::MultimodalIngestionFailed(detail) => {
                Self::MultimodalIngestionFailed(detail)
            }
            GeneratedTokenFinalResult::MultimodalNotSupported(detail) => {
                Self::MultimodalNotSupported(detail)
            }
            GeneratedTokenFinalResult::MultimodalTokenizationFailed(detail) => {
                Self::MultimodalTokenizationFailed(detail)
            }
            GeneratedTokenFinalResult::NoSequenceSlotAvailable(detail) => {
                Self::NoSequenceSlotAvailable(detail)
            }
            GeneratedTokenFinalResult::PromptExceedsContextSize(details) => {
                Self::PromptExceedsContextSize(details)
            }
            GeneratedTokenFinalResult::PromptTokenizationFailed(detail) => {
                Self::PromptTokenizationFailed(detail)
            }
            GeneratedTokenFinalResult::PromptTokenizedToNothing(detail) => {
                Self::PromptTokenizedToNothing(detail)
            }
            GeneratedTokenFinalResult::SamplerChainCreationFailed(detail) => {
                Self::SamplerChainCreationFailed(detail)
            }
            GeneratedTokenFinalResult::SamplerError(detail) => Self::SamplerError(detail),
            GeneratedTokenFinalResult::SamplingCandidatesExhausted(detail) => {
                Self::SamplingCandidatesExhausted(detail)
            }
            GeneratedTokenFinalResult::SchedulerUnavailable(detail) => {
                Self::SchedulerUnavailable(detail)
            }
            GeneratedTokenFinalResult::ToolSchemaInvalid(detail) => Self::ToolSchemaInvalid(detail),
        }
    }
}

impl From<GeneratedTokenPartialResult> for GeneratedTokenResult {
    fn from(partial_result: GeneratedTokenPartialResult) -> Self {
        match partial_result {
            GeneratedTokenPartialResult::ContentToken(text) => Self::ContentToken(text),
            GeneratedTokenPartialResult::ReasoningToken(text) => Self::ReasoningToken(text),
            GeneratedTokenPartialResult::ToolCallParseFailed(detail) => {
                Self::ToolCallParseFailed(detail)
            }
            GeneratedTokenPartialResult::ToolCallParsed(parsed_calls) => {
                Self::ToolCallParsed(parsed_calls)
            }
            GeneratedTokenPartialResult::ToolCallToken(text) => Self::ToolCallToken(text),
            GeneratedTokenPartialResult::ToolCallValidationFailed(validation_errors) => {
                Self::ToolCallValidationFailed(validation_errors)
            }
            GeneratedTokenPartialResult::UndeterminableToken(text) => {
                Self::UndeterminableToken(text)
            }
            GeneratedTokenPartialResult::UnrecognizedToolCallFormat(raw_tokens) => {
                Self::UnrecognizedToolCallFormat(raw_tokens)
            }
        }
    }
}

impl StreamableResult for GeneratedTokenResult {
    type Final = GeneratedTokenFinalResult;
    type Partial = GeneratedTokenPartialResult;

    fn into_step(self) -> ResultStep<GeneratedTokenPartialResult, GeneratedTokenFinalResult> {
        match self {
            Self::BatchAssemblyFailed(detail) => {
                ResultStep::Final(GeneratedTokenFinalResult::BatchAssemblyFailed(detail))
            }
            Self::ChatTemplateError(detail) => {
                ResultStep::Final(GeneratedTokenFinalResult::ChatTemplateError(detail))
            }
            Self::ContentToken(text) => {
                ResultStep::Partial(GeneratedTokenPartialResult::ContentToken(text))
            }
            Self::DecodeFailed(detail) => {
                ResultStep::Final(GeneratedTokenFinalResult::DecodeFailed(detail))
            }
            Self::DetokenizationFailed(detail) => {
                ResultStep::Final(GeneratedTokenFinalResult::DetokenizationFailed(detail))
            }
            Self::Done(summary) => ResultStep::Final(GeneratedTokenFinalResult::Done(summary)),
            Self::GrammarIncompatibleWithThinking(detail) => ResultStep::Final(
                GeneratedTokenFinalResult::GrammarIncompatibleWithThinking(detail),
            ),
            Self::GrammarInitializationFailed(detail) => ResultStep::Final(
                GeneratedTokenFinalResult::GrammarInitializationFailed(detail),
            ),
            Self::GrammarRejectedModelOutput(detail) => ResultStep::Final(
                GeneratedTokenFinalResult::GrammarRejectedModelOutput(detail),
            ),
            Self::GrammarSyntaxError(detail) => {
                ResultStep::Final(GeneratedTokenFinalResult::GrammarSyntaxError(detail))
            }
            Self::ImageDecodingFailed(detail) => {
                ResultStep::Final(GeneratedTokenFinalResult::ImageDecodingFailed(detail))
            }
            Self::KvCacheClearFailed(detail) => {
                ResultStep::Final(GeneratedTokenFinalResult::KvCacheClearFailed(detail))
            }
            Self::MediaExceedsMicroBatch(details) => {
                ResultStep::Final(GeneratedTokenFinalResult::MediaExceedsMicroBatch(details))
            }
            Self::MediaMicroBatchCheckFailed(detail) => ResultStep::Final(
                GeneratedTokenFinalResult::MediaMicroBatchCheckFailed(detail),
            ),
            Self::ModelNotLoaded(detail) => {
                ResultStep::Final(GeneratedTokenFinalResult::ModelNotLoaded(detail))
            }
            Self::MultimodalIngestionFailed(detail) => {
                ResultStep::Final(GeneratedTokenFinalResult::MultimodalIngestionFailed(detail))
            }
            Self::MultimodalNotSupported(detail) => {
                ResultStep::Final(GeneratedTokenFinalResult::MultimodalNotSupported(detail))
            }
            Self::MultimodalTokenizationFailed(detail) => ResultStep::Final(
                GeneratedTokenFinalResult::MultimodalTokenizationFailed(detail),
            ),
            Self::NoSequenceSlotAvailable(detail) => {
                ResultStep::Final(GeneratedTokenFinalResult::NoSequenceSlotAvailable(detail))
            }
            Self::PromptExceedsContextSize(details) => {
                ResultStep::Final(GeneratedTokenFinalResult::PromptExceedsContextSize(details))
            }
            Self::PromptTokenizationFailed(detail) => {
                ResultStep::Final(GeneratedTokenFinalResult::PromptTokenizationFailed(detail))
            }
            Self::PromptTokenizedToNothing(detail) => {
                ResultStep::Final(GeneratedTokenFinalResult::PromptTokenizedToNothing(detail))
            }
            Self::ReasoningToken(text) => {
                ResultStep::Partial(GeneratedTokenPartialResult::ReasoningToken(text))
            }
            Self::SamplerChainCreationFailed(detail) => ResultStep::Final(
                GeneratedTokenFinalResult::SamplerChainCreationFailed(detail),
            ),
            Self::SamplerError(detail) => {
                ResultStep::Final(GeneratedTokenFinalResult::SamplerError(detail))
            }
            Self::SamplingCandidatesExhausted(detail) => ResultStep::Final(
                GeneratedTokenFinalResult::SamplingCandidatesExhausted(detail),
            ),
            Self::SchedulerUnavailable(detail) => {
                ResultStep::Final(GeneratedTokenFinalResult::SchedulerUnavailable(detail))
            }
            Self::ToolCallParseFailed(detail) => {
                ResultStep::Partial(GeneratedTokenPartialResult::ToolCallParseFailed(detail))
            }
            Self::ToolCallParsed(parsed_calls) => {
                ResultStep::Partial(GeneratedTokenPartialResult::ToolCallParsed(parsed_calls))
            }
            Self::ToolCallToken(text) => {
                ResultStep::Partial(GeneratedTokenPartialResult::ToolCallToken(text))
            }
            Self::ToolCallValidationFailed(validation_errors) => ResultStep::Partial(
                GeneratedTokenPartialResult::ToolCallValidationFailed(validation_errors),
            ),
            Self::ToolSchemaInvalid(detail) => {
                ResultStep::Final(GeneratedTokenFinalResult::ToolSchemaInvalid(detail))
            }
            Self::UndeterminableToken(text) => {
                ResultStep::Partial(GeneratedTokenPartialResult::UndeterminableToken(text))
            }
            Self::UnrecognizedToolCallFormat(raw_tokens) => ResultStep::Partial(
                GeneratedTokenPartialResult::UnrecognizedToolCallFormat(raw_tokens),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings_types::TokenUsage;

    use super::GeneratedTokenResult;
    use crate::generation_finish::GenerationFinish;
    use crate::generation_summary::GenerationSummary;
    use crate::oversized_media_details::OversizedMediaDetails;
    use crate::oversized_prompt_details::OversizedPromptDetails;
    use crate::raw_tool_call_tokens::RawToolCallTokens;
    use crate::result_step::ResultStep;
    use crate::streamable_result::StreamableResult as _;

    fn keeps_the_stream_open(token_result: &GeneratedTokenResult) -> bool {
        match token_result.clone().into_step() {
            ResultStep::Final(final_result) => {
                assert_eq!(&GeneratedTokenResult::from(final_result), token_result);

                false
            }
            ResultStep::Partial(partial_result) => {
                assert_eq!(&GeneratedTokenResult::from(partial_result), token_result);

                true
            }
        }
    }

    #[test]
    fn every_terminal_result_ends_the_stream_without_a_token() {
        for terminal_result in [
            GeneratedTokenResult::BatchAssemblyFailed("failure".to_owned()),
            GeneratedTokenResult::ChatTemplateError("failure".to_owned()),
            GeneratedTokenResult::DecodeFailed("failure".to_owned()),
            GeneratedTokenResult::DetokenizationFailed("failure".to_owned()),
            GeneratedTokenResult::Done(GenerationSummary {
                finish: GenerationFinish::EndOfGeneration,
                usage: TokenUsage::new(),
            }),
            GeneratedTokenResult::GrammarIncompatibleWithThinking("failure".to_owned()),
            GeneratedTokenResult::GrammarInitializationFailed("failure".to_owned()),
            GeneratedTokenResult::GrammarRejectedModelOutput("failure".to_owned()),
            GeneratedTokenResult::GrammarSyntaxError("failure".to_owned()),
            GeneratedTokenResult::ImageDecodingFailed("failure".to_owned()),
            GeneratedTokenResult::KvCacheClearFailed("failure".to_owned()),
            GeneratedTokenResult::MediaExceedsMicroBatch(OversizedMediaDetails {
                media_tokens: 368,
                micro_batch_tokens: 100,
            }),
            GeneratedTokenResult::MediaMicroBatchCheckFailed("failure".to_owned()),
            GeneratedTokenResult::ModelNotLoaded("failure".to_owned()),
            GeneratedTokenResult::MultimodalIngestionFailed("failure".to_owned()),
            GeneratedTokenResult::MultimodalNotSupported("failure".to_owned()),
            GeneratedTokenResult::MultimodalTokenizationFailed("failure".to_owned()),
            GeneratedTokenResult::NoSequenceSlotAvailable("failure".to_owned()),
            GeneratedTokenResult::PromptExceedsContextSize(OversizedPromptDetails {
                prompt_tokens: 9895,
                sequence_context_size: 8192,
            }),
            GeneratedTokenResult::PromptTokenizationFailed("failure".to_owned()),
            GeneratedTokenResult::PromptTokenizedToNothing("failure".to_owned()),
            GeneratedTokenResult::SamplerChainCreationFailed("failure".to_owned()),
            GeneratedTokenResult::SamplerError("failure".to_owned()),
            GeneratedTokenResult::SamplingCandidatesExhausted("failure".to_owned()),
            GeneratedTokenResult::SchedulerUnavailable("failure".to_owned()),
            GeneratedTokenResult::ToolSchemaInvalid("failure".to_owned()),
        ] {
            assert!(!keeps_the_stream_open(&terminal_result));
            assert!(!terminal_result.is_token());
            assert_eq!(terminal_result.token_text(), None);
        }
    }

    #[test]
    fn every_token_carries_its_text_without_ending_the_stream() {
        for token_result in [
            GeneratedTokenResult::ContentToken("piece".to_owned()),
            GeneratedTokenResult::ReasoningToken("piece".to_owned()),
            GeneratedTokenResult::ToolCallToken("piece".to_owned()),
            GeneratedTokenResult::UndeterminableToken("piece".to_owned()),
        ] {
            assert!(keeps_the_stream_open(&token_result));
            assert!(token_result.is_token());
            assert_eq!(token_result.token_text(), Some("piece"));
        }
    }

    #[test]
    fn tool_call_parsed_is_not_done() {
        let event = GeneratedTokenResult::ToolCallParsed(vec![]);

        assert!(keeps_the_stream_open(&event));
        assert!(event.is_tool_call_parsed());
        assert!(!event.is_tool_call_failure());
    }

    #[test]
    fn tool_call_parse_failed_is_failure_but_not_done() {
        let event = GeneratedTokenResult::ToolCallParseFailed("oops".to_owned());

        assert!(keeps_the_stream_open(&event));
        assert!(!event.is_tool_call_parsed());
        assert!(event.is_tool_call_failure());
    }

    #[test]
    fn tool_call_validation_failed_is_failure_but_not_done() {
        let event = GeneratedTokenResult::ToolCallValidationFailed(vec!["missing".to_owned()]);

        assert!(keeps_the_stream_open(&event));
        assert!(!event.is_tool_call_parsed());
        assert!(event.is_tool_call_failure());
    }

    #[test]
    fn unrecognized_tool_call_format_is_not_done_and_not_classified_as_token() {
        let event = GeneratedTokenResult::UnrecognizedToolCallFormat(RawToolCallTokens {
            text: "raw output".to_owned(),
            ffi_error_message: "parser bailed".to_owned(),
        });

        assert!(keeps_the_stream_open(&event));
        assert!(!event.is_token());
        assert!(event.token_text().is_none());
        assert!(!event.is_tool_call_parsed());
        assert!(!event.is_tool_call_failure());
    }
}
