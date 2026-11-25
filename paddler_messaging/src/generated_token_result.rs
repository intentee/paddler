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
use crate::undecodable_media::UndecodableMedia;
use crate::unsupported_conversation_feature::UnsupportedConversationFeature;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum GeneratedTokenResult {
    AnswerClassificationFailed(String),
    BatchAssemblyFailed(String),
    ChatTemplateError(String),
    ContentToken(String),
    ConversationFeatureNotSupported(UnsupportedConversationFeature),
    DecodeFailed(String),
    DetokenizationFailed(String),
    Done(GenerationSummary),
    GrammarIncompatibleWithThinking(String),
    GrammarInitializationFailed(String),
    GrammarRejectedModelOutput(String),
    GrammarSyntaxError(String),
    KvCacheClearFailed(String),
    MediaDecodingFailed(UndecodableMedia),
    MediaExceedsMicroBatch(OversizedMediaDetails),
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
    TextGenerationNotServed(String),
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
    pub const fn is_tool_call_failure(&self) -> bool {
        matches!(
            self,
            Self::ToolCallParseFailed(_) | Self::ToolCallValidationFailed(_)
        )
    }

    #[must_use]
    pub const fn is_tool_call_parsed(&self) -> bool {
        matches!(self, Self::ToolCallParsed(_))
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
}

impl From<GeneratedTokenFinalResult> for GeneratedTokenResult {
    fn from(final_result: GeneratedTokenFinalResult) -> Self {
        match final_result {
            GeneratedTokenFinalResult::BatchAssemblyFailed(detail) => {
                Self::BatchAssemblyFailed(detail)
            }
            GeneratedTokenFinalResult::AnswerClassificationFailed(detail) => {
                Self::AnswerClassificationFailed(detail)
            }
            GeneratedTokenFinalResult::ChatTemplateError(detail) => Self::ChatTemplateError(detail),
            GeneratedTokenFinalResult::ConversationFeatureNotSupported(unsupported_feature) => {
                Self::ConversationFeatureNotSupported(unsupported_feature)
            }
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
            GeneratedTokenFinalResult::KvCacheClearFailed(detail) => {
                Self::KvCacheClearFailed(detail)
            }
            GeneratedTokenFinalResult::MediaDecodingFailed(undecodable_media) => {
                Self::MediaDecodingFailed(undecodable_media)
            }
            GeneratedTokenFinalResult::MediaExceedsMicroBatch(details) => {
                Self::MediaExceedsMicroBatch(details)
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
            GeneratedTokenFinalResult::TextGenerationNotServed(detail) => {
                Self::TextGenerationNotServed(detail)
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
            Self::AnswerClassificationFailed(detail) => {
                GeneratedTokenFinalResult::AnswerClassificationFailed(detail).into()
            }
            Self::BatchAssemblyFailed(detail) => {
                GeneratedTokenFinalResult::BatchAssemblyFailed(detail).into()
            }
            Self::ChatTemplateError(detail) => {
                GeneratedTokenFinalResult::ChatTemplateError(detail).into()
            }
            Self::ContentToken(text) => GeneratedTokenPartialResult::ContentToken(text).into(),
            Self::ConversationFeatureNotSupported(unsupported_feature) => {
                GeneratedTokenFinalResult::ConversationFeatureNotSupported(unsupported_feature)
                    .into()
            }
            Self::DecodeFailed(detail) => GeneratedTokenFinalResult::DecodeFailed(detail).into(),
            Self::DetokenizationFailed(detail) => {
                GeneratedTokenFinalResult::DetokenizationFailed(detail).into()
            }
            Self::Done(summary) => GeneratedTokenFinalResult::Done(summary).into(),
            Self::GrammarIncompatibleWithThinking(detail) => {
                GeneratedTokenFinalResult::GrammarIncompatibleWithThinking(detail).into()
            }
            Self::GrammarInitializationFailed(detail) => {
                GeneratedTokenFinalResult::GrammarInitializationFailed(detail).into()
            }
            Self::GrammarRejectedModelOutput(detail) => {
                GeneratedTokenFinalResult::GrammarRejectedModelOutput(detail).into()
            }
            Self::GrammarSyntaxError(detail) => {
                GeneratedTokenFinalResult::GrammarSyntaxError(detail).into()
            }
            Self::KvCacheClearFailed(detail) => {
                GeneratedTokenFinalResult::KvCacheClearFailed(detail).into()
            }
            Self::MediaDecodingFailed(undecodable_media) => {
                GeneratedTokenFinalResult::MediaDecodingFailed(undecodable_media).into()
            }
            Self::MediaExceedsMicroBatch(details) => {
                GeneratedTokenFinalResult::MediaExceedsMicroBatch(details).into()
            }
            Self::ModelNotLoaded(detail) => {
                GeneratedTokenFinalResult::ModelNotLoaded(detail).into()
            }
            Self::MultimodalIngestionFailed(detail) => {
                GeneratedTokenFinalResult::MultimodalIngestionFailed(detail).into()
            }
            Self::MultimodalNotSupported(detail) => {
                GeneratedTokenFinalResult::MultimodalNotSupported(detail).into()
            }
            Self::MultimodalTokenizationFailed(detail) => {
                GeneratedTokenFinalResult::MultimodalTokenizationFailed(detail).into()
            }
            Self::NoSequenceSlotAvailable(detail) => {
                GeneratedTokenFinalResult::NoSequenceSlotAvailable(detail).into()
            }
            Self::PromptExceedsContextSize(details) => {
                GeneratedTokenFinalResult::PromptExceedsContextSize(details).into()
            }
            Self::PromptTokenizationFailed(detail) => {
                GeneratedTokenFinalResult::PromptTokenizationFailed(detail).into()
            }
            Self::PromptTokenizedToNothing(detail) => {
                GeneratedTokenFinalResult::PromptTokenizedToNothing(detail).into()
            }
            Self::ReasoningToken(text) => GeneratedTokenPartialResult::ReasoningToken(text).into(),
            Self::SamplerChainCreationFailed(detail) => {
                GeneratedTokenFinalResult::SamplerChainCreationFailed(detail).into()
            }
            Self::SamplerError(detail) => GeneratedTokenFinalResult::SamplerError(detail).into(),
            Self::SamplingCandidatesExhausted(detail) => {
                GeneratedTokenFinalResult::SamplingCandidatesExhausted(detail).into()
            }
            Self::SchedulerUnavailable(detail) => {
                GeneratedTokenFinalResult::SchedulerUnavailable(detail).into()
            }
            Self::TextGenerationNotServed(detail) => {
                GeneratedTokenFinalResult::TextGenerationNotServed(detail).into()
            }
            Self::ToolCallParseFailed(detail) => {
                GeneratedTokenPartialResult::ToolCallParseFailed(detail).into()
            }
            Self::ToolCallParsed(parsed_calls) => {
                GeneratedTokenPartialResult::ToolCallParsed(parsed_calls).into()
            }
            Self::ToolCallToken(text) => GeneratedTokenPartialResult::ToolCallToken(text).into(),
            Self::ToolCallValidationFailed(validation_errors) => {
                GeneratedTokenPartialResult::ToolCallValidationFailed(validation_errors).into()
            }
            Self::ToolSchemaInvalid(detail) => {
                GeneratedTokenFinalResult::ToolSchemaInvalid(detail).into()
            }
            Self::UndeterminableToken(text) => {
                GeneratedTokenPartialResult::UndeterminableToken(text).into()
            }
            Self::UnrecognizedToolCallFormat(raw_tokens) => {
                GeneratedTokenPartialResult::UnrecognizedToolCallFormat(raw_tokens).into()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;

    use super::GeneratedTokenResult;
    use crate::conversation_feature::ConversationFeature;
    use crate::generation_finish::GenerationFinish;
    use crate::generation_summary::GenerationSummary;
    use crate::media_kind::MediaKind;
    use crate::oversized_media_details::OversizedMediaDetails;
    use crate::oversized_prompt_details::OversizedPromptDetails;
    use crate::raw_tool_call_tokens::RawToolCallTokens;
    use crate::result_step::ResultStep;
    use crate::streamable_result::StreamableResult as _;
    use crate::token_usage::TokenUsage;
    use crate::undecodable_media::UndecodableMedia;
    use crate::unsupported_conversation_feature::UnsupportedConversationFeature;

    fn keeps_the_stream_open(token_result: &GeneratedTokenResult) -> bool {
        matches!(token_result.clone().into_step(), ResultStep::Partial(_))
    }

    fn step_round_trip(token_result: GeneratedTokenResult) -> GeneratedTokenResult {
        match token_result.into_step() {
            ResultStep::Final(final_result) => final_result.into(),
            ResultStep::Partial(partial_result) => partial_result.into(),
        }
    }

    fn detailed_terminal_results() -> [GeneratedTokenResult; 5] {
        [
            GeneratedTokenResult::MediaDecodingFailed(UndecodableMedia {
                description: "the audio data is not valid base64".to_owned(),
                media: MediaKind::Audio,
            }),
            GeneratedTokenResult::ConversationFeatureNotSupported(UnsupportedConversationFeature {
                description: "the chat template of the model does not support thinking".to_owned(),
                feature: ConversationFeature::Thinking,
            }),
            GeneratedTokenResult::Done(GenerationSummary {
                finish: GenerationFinish::EndOfGeneration,
                usage: TokenUsage::default(),
            }),
            GeneratedTokenResult::MediaExceedsMicroBatch(OversizedMediaDetails {
                media_tokens: 368,
                micro_batch_tokens: 100,
            }),
            GeneratedTokenResult::PromptExceedsContextSize(OversizedPromptDetails {
                prompt_tokens: 9895,
                sequence_context_size: 8192,
            }),
        ]
    }

    #[test]
    fn every_terminal_result_ends_the_stream_without_a_token() {
        for terminal_result in detailed_terminal_results().into_iter().chain([
            GeneratedTokenResult::BatchAssemblyFailed(String::new()),
            GeneratedTokenResult::ChatTemplateError(String::new()),
            GeneratedTokenResult::DecodeFailed(String::new()),
            GeneratedTokenResult::DetokenizationFailed(String::new()),
            GeneratedTokenResult::AnswerClassificationFailed(String::new()),
            GeneratedTokenResult::GrammarIncompatibleWithThinking(String::new()),
            GeneratedTokenResult::GrammarInitializationFailed(String::new()),
            GeneratedTokenResult::GrammarRejectedModelOutput(String::new()),
            GeneratedTokenResult::GrammarSyntaxError(String::new()),
            GeneratedTokenResult::KvCacheClearFailed(String::new()),
            GeneratedTokenResult::ModelNotLoaded(String::new()),
            GeneratedTokenResult::MultimodalIngestionFailed(String::new()),
            GeneratedTokenResult::MultimodalNotSupported(String::new()),
            GeneratedTokenResult::TextGenerationNotServed(String::new()),
            GeneratedTokenResult::MultimodalTokenizationFailed(String::new()),
            GeneratedTokenResult::NoSequenceSlotAvailable(String::new()),
            GeneratedTokenResult::PromptTokenizationFailed(String::new()),
            GeneratedTokenResult::PromptTokenizedToNothing(String::new()),
            GeneratedTokenResult::SamplerChainCreationFailed(String::new()),
            GeneratedTokenResult::SamplerError(String::new()),
            GeneratedTokenResult::SamplingCandidatesExhausted(String::new()),
            GeneratedTokenResult::SchedulerUnavailable(String::new()),
            GeneratedTokenResult::ToolSchemaInvalid(String::new()),
        ]) {
            assert_eq!(
                discriminant(&step_round_trip(terminal_result.clone())),
                discriminant(&terminal_result)
            );
            assert!(!keeps_the_stream_open(&terminal_result));
            assert!(!terminal_result.is_token());
            assert_eq!(terminal_result.token_text(), None);
        }
    }

    #[test]
    fn every_detailed_terminal_result_keeps_its_details_through_the_stream_step() {
        for detailed_terminal_result in detailed_terminal_results() {
            assert_eq!(
                step_round_trip(detailed_terminal_result.clone()),
                detailed_terminal_result
            );
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
            assert_eq!(step_round_trip(token_result.clone()), token_result);
            assert!(keeps_the_stream_open(&token_result));
            assert!(token_result.is_token());
            assert_eq!(token_result.token_text(), Some("piece"));
        }
    }

    #[test]
    fn tool_call_parsed_is_a_parsed_tool_call_that_keeps_the_stream_open() {
        let event = GeneratedTokenResult::ToolCallParsed(vec![]);

        assert_eq!(step_round_trip(event.clone()), event);
        assert!(keeps_the_stream_open(&event));
        assert!(event.is_tool_call_parsed());
        assert!(!event.is_tool_call_failure());
    }

    #[test]
    fn tool_call_parse_failed_is_failure_but_not_done() {
        let event = GeneratedTokenResult::ToolCallParseFailed(String::new());

        assert!(matches!(
            step_round_trip(event.clone()),
            GeneratedTokenResult::ToolCallParseFailed(_)
        ));
        assert!(keeps_the_stream_open(&event));
        assert!(!event.is_tool_call_parsed());
        assert!(event.is_tool_call_failure());
    }

    #[test]
    fn tool_call_validation_failed_is_failure_but_not_done() {
        let event = GeneratedTokenResult::ToolCallValidationFailed(vec![String::new()]);

        assert!(matches!(
            step_round_trip(event.clone()),
            GeneratedTokenResult::ToolCallValidationFailed(validation_failures)
                if validation_failures.len() == 1
        ));
        assert!(keeps_the_stream_open(&event));
        assert!(!event.is_tool_call_parsed());
        assert!(event.is_tool_call_failure());
    }

    #[test]
    fn unrecognized_tool_call_format_keeps_the_stream_open_as_neither_token_nor_tool_call() {
        let event = GeneratedTokenResult::UnrecognizedToolCallFormat(RawToolCallTokens {
            text: "raw output".to_owned(),
            expected_format: "peg-native".to_owned(),
        });

        assert_eq!(step_round_trip(event.clone()), event);
        assert!(keeps_the_stream_open(&event));
        assert!(!event.is_token());
        assert!(event.token_text().is_none());
        assert!(!event.is_tool_call_parsed());
        assert!(!event.is_tool_call_failure());
    }
}
