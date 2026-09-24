use std::num::TryFromIntError;

use anyhow::Error as AnyhowError;
use llama_cpp_bindings::error::ChatToolsError;
use llama_cpp_bindings::error::EvalMultimodalChunksError;
use llama_cpp_bindings::error::GrammarError;
use llama_cpp_bindings::error::JsonSchemaToGrammarError;
use llama_cpp_bindings::error::MarkerDetectionError;
use llama_cpp_bindings::error::SamplingError;
use llama_cpp_bindings::error::StringToTokenError;
use llama_cpp_bindings::mtmd::MtmdBitmapError;
use llama_cpp_bindings::mtmd::MtmdDefaultMarkerError;
use llama_cpp_bindings::mtmd::MtmdEvalError;
use llama_cpp_bindings::mtmd::MtmdTokenizeError;
use log::error;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::oversized_image_details::OversizedImageDetails;
use paddler_messaging::oversized_prompt_details::OversizedPromptDetails;
use tokio::sync::mpsc;

use crate::decoded_image_error::DecodedImageError;
use crate::send_generated_token_result_or_warn::send_generated_token_result_or_warn;
use crate::validator_build_error::ValidatorBuildError;

#[derive(Debug, thiserror::Error)]
pub enum GenerationRequestRejection {
    #[error("Grammar constraints are incompatible with thinking mode")]
    GrammarIncompatibleWithThinking,

    #[error("Failed to convert JSON schema to grammar: {0}")]
    GrammarConversionFailed(#[source] JsonSchemaToGrammarError),

    #[error("token generation is disabled because this agent is running in embeddings-only mode")]
    TokenGenerationDisabled,

    #[error("failed to decode images: {0}")]
    ImageDecodingFailed(#[source] DecodedImageError),

    #[error("failed to create image bitmap: {0}")]
    ImageBitmapCreationFailed(#[source] MtmdBitmapError),

    #[error("received images but model does not support multimodal input")]
    MultimodalNotSupported,

    #[error("failed to resolve the media marker: {0}")]
    MediaMarkerUnavailable(#[source] MtmdDefaultMarkerError),

    #[error("failed to render chat template: {0:?}")]
    ChatTemplateRenderingFailed(AnyhowError),

    #[error("{0}")]
    ToolSchemaInvalid(#[source] ValidatorBuildError),

    #[error("failed to serialize tools: {0}")]
    ToolsSerializationFailed(#[source] serde_json::Error),

    #[error("serialized tools are not a valid tool list: {0}")]
    ChatToolsInvalid(#[source] ChatToolsError),

    #[error("failed to tokenize prompt: {0}")]
    PromptTokenizationFailed(#[source] StringToTokenError),

    #[error(
        "prompt has {} tokens but each sequence holds {} tokens",
        details.prompt_tokens,
        details.sequence_context_size
    )]
    PromptExceedsContextSize { details: OversizedPromptDetails },

    #[error("n_batch does not fit in i32: {0}")]
    BatchSizeOutOfRange(#[source] TryFromIntError),

    #[error("the scheduler is no longer accepting requests")]
    SchedulerUnavailable,

    #[error("no available sequence slots, all slots are busy")]
    NoSequenceSlotAvailable,

    #[error("failed to initialize grammar sampler: {0}")]
    GrammarSamplerInitializationFailed(#[source] GrammarError),

    #[error("failed to create sampler chain: {0}")]
    SamplerChainCreationFailed(#[source] SamplingError),

    #[error("failed to build the sampled token classifier: {0}")]
    TokenClassifierUnavailable(#[source] MarkerDetectionError),

    #[error("failed to tokenize multimodal input: {0}")]
    MultimodalTokenizationFailed(#[source] MtmdTokenizeError),

    #[error(
        "image chunk has {} tokens but n_batch is {}",
        details.image_tokens,
        details.n_batch
    )]
    ImageExceedsBatchSize { details: OversizedImageDetails },

    #[error("failed to ingest multimodal prompt: {0}")]
    MultimodalIngestionFailed(#[source] EvalMultimodalChunksError),
}

impl From<EvalMultimodalChunksError> for GenerationRequestRejection {
    fn from(error: EvalMultimodalChunksError) -> Self {
        match error {
            EvalMultimodalChunksError::EvalFailed(MtmdEvalError::ImageChunkExceedsBatchSize(
                mismatch,
            )) => Self::ImageExceedsBatchSize {
                details: OversizedImageDetails {
                    image_tokens: mismatch.image_tokens,
                    n_batch: mismatch.n_batch,
                },
            },
            other_error => Self::MultimodalIngestionFailed(other_error),
        }
    }
}

impl GenerationRequestRejection {
    pub fn report(
        self,
        agent_name: Option<&str>,
        generated_tokens_tx: &mpsc::UnboundedSender<GeneratedTokenResult>,
    ) {
        let message = format!("{agent_name:?}: {self}");

        error!("{message}");

        send_generated_token_result_or_warn(
            agent_name,
            generated_tokens_tx,
            self.into_generated_token_result(message),
        );
    }

    fn into_generated_token_result(self, message: String) -> GeneratedTokenResult {
        match self {
            Self::GrammarIncompatibleWithThinking => {
                GeneratedTokenResult::GrammarIncompatibleWithThinking(message)
            }
            Self::GrammarConversionFailed(_) => GeneratedTokenResult::GrammarSyntaxError(message),
            Self::TokenGenerationDisabled => GeneratedTokenResult::TokenGenerationDisabled(message),
            Self::ImageDecodingFailed(_) | Self::ImageBitmapCreationFailed(_) => {
                GeneratedTokenResult::ImageDecodingFailed(message)
            }
            Self::MultimodalNotSupported => GeneratedTokenResult::MultimodalNotSupported(message),
            Self::MediaMarkerUnavailable(_) | Self::ChatTemplateRenderingFailed(_) => {
                GeneratedTokenResult::ChatTemplateError(message)
            }
            Self::ToolSchemaInvalid(_) => GeneratedTokenResult::ToolSchemaInvalid(message),
            Self::GrammarSamplerInitializationFailed(_) => {
                GeneratedTokenResult::GrammarInitializationFailed(message)
            }
            Self::ImageExceedsBatchSize { details } => {
                GeneratedTokenResult::ImageExceedsBatchSize(details)
            }
            Self::PromptExceedsContextSize { details } => {
                GeneratedTokenResult::PromptExceedsContextSize(details)
            }
            Self::ToolsSerializationFailed(_)
            | Self::ChatToolsInvalid(_)
            | Self::PromptTokenizationFailed(_)
            | Self::BatchSizeOutOfRange(_)
            | Self::SchedulerUnavailable
            | Self::NoSequenceSlotAvailable
            | Self::SamplerChainCreationFailed(_)
            | Self::TokenClassifierUnavailable(_)
            | Self::MultimodalTokenizationFailed(_)
            | Self::MultimodalIngestionFailed(_) => GeneratedTokenResult::SamplerError(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;
    use llama_cpp_bindings::error::EvalMultimodalChunksError;
    use llama_cpp_bindings::error::FfiStatusError;
    use llama_cpp_bindings::error::GrammarError;
    use llama_cpp_bindings::mtmd::ImageChunkBatchSizeMismatch;
    use llama_cpp_bindings::mtmd::MtmdEvalError;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::oversized_prompt_details::OversizedPromptDetails;
    use tokio::sync::mpsc;

    use super::GenerationRequestRejection;
    use crate::decoded_image_error::DecodedImageError;
    use crate::validator_build_error::ValidatorBuildError;

    fn reported(rejection: GenerationRequestRejection) -> GeneratedTokenResult {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();

        rejection.report(Some("agent"), &generated_tokens_tx);

        generated_tokens_rx.try_recv().unwrap()
    }

    #[test]
    fn reports_thinking_incompatibility_with_the_agent_name() {
        let result = reported(GenerationRequestRejection::GrammarIncompatibleWithThinking);

        assert!(matches!(
            result,
            GeneratedTokenResult::GrammarIncompatibleWithThinking(message)
                if message == "Some(\"agent\"): Grammar constraints are incompatible with thinking mode"
        ));
    }

    #[test]
    fn reports_grammar_conversion_failure_as_grammar_syntax_error() {
        let result = reported(GenerationRequestRejection::GrammarConversionFailed(
            FfiStatusError {
                operation: "json_schema_to_grammar",
                code: 1,
            }
            .into(),
        ));

        assert!(matches!(
            result,
            GeneratedTokenResult::GrammarSyntaxError(_)
        ));
    }

    #[test]
    fn reports_disabled_token_generation() {
        let result = reported(GenerationRequestRejection::TokenGenerationDisabled);

        assert!(matches!(
            result,
            GeneratedTokenResult::TokenGenerationDisabled(_)
        ));
    }

    #[test]
    fn reports_image_failures_as_image_decoding_failure() {
        let result = reported(GenerationRequestRejection::ImageDecodingFailed(
            DecodedImageError::MissingCommaSeparator,
        ));

        assert!(matches!(
            result,
            GeneratedTokenResult::ImageDecodingFailed(_)
        ));
    }

    #[test]
    fn reports_missing_multimodal_support() {
        let result = reported(GenerationRequestRejection::MultimodalNotSupported);

        assert!(matches!(
            result,
            GeneratedTokenResult::MultimodalNotSupported(_)
        ));
    }

    #[test]
    fn reports_prompt_rendering_failures_as_chat_template_error() {
        let result = reported(GenerationRequestRejection::ChatTemplateRenderingFailed(
            anyhow!("missing variable"),
        ));

        assert!(matches!(result, GeneratedTokenResult::ChatTemplateError(_)));
    }

    #[test]
    fn reports_invalid_tool_schema() {
        let result = reported(GenerationRequestRejection::ToolSchemaInvalid(
            ValidatorBuildError::InvalidSchema {
                tool_name: "get_weather".to_owned(),
                message: "not a schema".to_owned(),
            },
        ));

        assert!(matches!(
            result,
            GeneratedTokenResult::ToolSchemaInvalid(message)
                if message == "Some(\"agent\"): tool \"get_weather\" parameters are not a valid JSON Schema: not a schema"
        ));
    }

    #[test]
    fn reports_grammar_sampler_initialization_failure() {
        let result = reported(
            GenerationRequestRejection::GrammarSamplerInitializationFailed(
                GrammarError::FfiStatus(FfiStatusError {
                    operation: "llama_sampler_init_grammar",
                    code: 1,
                }),
            ),
        );

        assert!(matches!(
            result,
            GeneratedTokenResult::GrammarInitializationFailed(_)
        ));
    }

    #[test]
    fn reports_oversized_image_chunk_with_its_token_counts() {
        let rejection = GenerationRequestRejection::from(EvalMultimodalChunksError::EvalFailed(
            MtmdEvalError::ImageChunkExceedsBatchSize(ImageChunkBatchSizeMismatch {
                image_tokens: 9,
                n_batch: 4,
            }),
        ));

        let result = reported(rejection);

        assert!(matches!(
            result,
            GeneratedTokenResult::ImageExceedsBatchSize(details)
                if details.image_tokens == 9 && details.n_batch == 4
        ));
    }

    #[test]
    fn reports_oversized_prompt_with_its_token_counts() {
        let result = reported(GenerationRequestRejection::PromptExceedsContextSize {
            details: OversizedPromptDetails {
                prompt_tokens: 9895,
                sequence_context_size: 8192,
            },
        });

        assert!(matches!(
            result,
            GeneratedTokenResult::PromptExceedsContextSize(details)
                if details.prompt_tokens == 9895 && details.sequence_context_size == 8192
        ));
    }

    #[test]
    fn reports_other_multimodal_ingestion_failures_as_sampler_error() {
        let rejection =
            GenerationRequestRejection::from(EvalMultimodalChunksError::ChunkOutOfBounds(3));

        let result = reported(rejection);

        assert!(matches!(result, GeneratedTokenResult::SamplerError(_)));
    }
}
