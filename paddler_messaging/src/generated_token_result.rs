use llama_cpp_bindings_types::ParsedToolCall;
use serde::Deserialize;
use serde::Serialize;

use crate::generation_summary::GenerationSummary;
use crate::oversized_media_details::OversizedMediaDetails;
use crate::oversized_prompt_details::OversizedPromptDetails;
use crate::raw_tool_call_tokens::RawToolCallTokens;
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
    InferenceModeMismatch(String),
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
    ReasoningToken(String),
    SamplerChainCreationFailed(String),
    SamplerError(String),
    SamplingCandidatesExhausted(String),
    SchedulerUnavailable(String),
    SequenceIdOutOfRange(String),
    ToolCallParseFailed(String),
    ToolCallParsed(Vec<ParsedToolCall>),
    ToolCallToken(String),
    ToolCallValidationFailed(Vec<String>),
    ToolSchemaInvalid(String),
    ToolsSerializationFailed(String),
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

impl StreamableResult for GeneratedTokenResult {
    fn is_done(&self) -> bool {
        matches!(
            self,
            Self::BatchAssemblyFailed(_)
                | Self::ChatTemplateError(_)
                | Self::DecodeFailed(_)
                | Self::DetokenizationFailed(_)
                | Self::Done(_)
                | Self::GrammarIncompatibleWithThinking(_)
                | Self::GrammarInitializationFailed(_)
                | Self::GrammarRejectedModelOutput(_)
                | Self::GrammarSyntaxError(_)
                | Self::ImageDecodingFailed(_)
                | Self::InferenceModeMismatch(_)
                | Self::KvCacheClearFailed(_)
                | Self::MediaExceedsMicroBatch(_)
                | Self::MediaMicroBatchCheckFailed(_)
                | Self::ModelNotLoaded(_)
                | Self::MultimodalIngestionFailed(_)
                | Self::MultimodalNotSupported(_)
                | Self::MultimodalTokenizationFailed(_)
                | Self::NoSequenceSlotAvailable(_)
                | Self::PromptExceedsContextSize(_)
                | Self::PromptTokenizationFailed(_)
                | Self::SamplerChainCreationFailed(_)
                | Self::SamplerError(_)
                | Self::SamplingCandidatesExhausted(_)
                | Self::SchedulerUnavailable(_)
                | Self::SequenceIdOutOfRange(_)
                | Self::ToolSchemaInvalid(_)
                | Self::ToolsSerializationFailed(_)
        )
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
    use crate::streamable_result::StreamableResult;

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
            GeneratedTokenResult::InferenceModeMismatch("failure".to_owned()),
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
            GeneratedTokenResult::SamplerChainCreationFailed("failure".to_owned()),
            GeneratedTokenResult::SamplerError("failure".to_owned()),
            GeneratedTokenResult::SamplingCandidatesExhausted("failure".to_owned()),
            GeneratedTokenResult::SchedulerUnavailable("failure".to_owned()),
            GeneratedTokenResult::SequenceIdOutOfRange("failure".to_owned()),
            GeneratedTokenResult::ToolSchemaInvalid("failure".to_owned()),
            GeneratedTokenResult::ToolsSerializationFailed("failure".to_owned()),
        ] {
            assert!(terminal_result.is_done());
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
            assert!(!token_result.is_done());
            assert!(token_result.is_token());
            assert_eq!(token_result.token_text(), Some("piece"));
        }
    }

    #[test]
    fn tool_call_parsed_is_not_done() {
        let event = GeneratedTokenResult::ToolCallParsed(vec![]);

        assert!(!event.is_done());
        assert!(event.is_tool_call_parsed());
        assert!(!event.is_tool_call_failure());
    }

    #[test]
    fn tool_call_parse_failed_is_failure_but_not_done() {
        let event = GeneratedTokenResult::ToolCallParseFailed("oops".to_owned());

        assert!(!event.is_done());
        assert!(!event.is_tool_call_parsed());
        assert!(event.is_tool_call_failure());
    }

    #[test]
    fn tool_call_validation_failed_is_failure_but_not_done() {
        let event = GeneratedTokenResult::ToolCallValidationFailed(vec!["missing".to_owned()]);

        assert!(!event.is_done());
        assert!(!event.is_tool_call_parsed());
        assert!(event.is_tool_call_failure());
    }

    #[test]
    fn unrecognized_tool_call_format_is_not_done_and_not_classified_as_token() {
        let event = GeneratedTokenResult::UnrecognizedToolCallFormat(RawToolCallTokens {
            text: "raw output".to_owned(),
            ffi_error_message: "parser bailed".to_owned(),
        });

        assert!(!event.is_done());
        assert!(!event.is_token());
        assert!(event.token_text().is_none());
        assert!(!event.is_tool_call_parsed());
        assert!(!event.is_tool_call_failure());
    }
}
