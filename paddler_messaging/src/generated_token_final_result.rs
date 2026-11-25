use crate::generation_summary::GenerationSummary;
use crate::oversized_media_details::OversizedMediaDetails;
use crate::oversized_prompt_details::OversizedPromptDetails;
use crate::undecodable_media::UndecodableMedia;
use crate::unsupported_conversation_feature::UnsupportedConversationFeature;

#[derive(Debug, Eq, PartialEq)]
pub enum GeneratedTokenFinalResult {
    BatchAssemblyFailed(String),
    AnswerClassificationFailed(String),
    ChatTemplateError(String),
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
    SamplerChainCreationFailed(String),
    SamplerError(String),
    SamplingCandidatesExhausted(String),
    SchedulerUnavailable(String),
    TextGenerationNotServed(String),
    ToolSchemaInvalid(String),
}
