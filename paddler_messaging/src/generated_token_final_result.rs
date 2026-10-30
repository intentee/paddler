use crate::generation_summary::GenerationSummary;
use crate::oversized_media_details::OversizedMediaDetails;
use crate::oversized_prompt_details::OversizedPromptDetails;

#[derive(Debug, Eq, PartialEq)]
pub enum GeneratedTokenFinalResult {
    BatchAssemblyFailed(String),
    ChatTemplateError(String),
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
    SamplerChainCreationFailed(String),
    SamplerError(String),
    SamplingCandidatesExhausted(String),
    SchedulerUnavailable(String),
    ToolSchemaInvalid(String),
}
