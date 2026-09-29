use anyhow::Error as AnyhowError;
use llama_cpp_bindings::error::EvalMultimodalChunksError;
use llama_cpp_bindings::error::GrammarError;
use llama_cpp_bindings::error::JsonSchemaToGrammarError;
use llama_cpp_bindings::error::ParseChatMessageError;
use llama_cpp_bindings::error::SamplingError;
use llama_cpp_bindings::error::StringToTokenError;
use llama_cpp_bindings::mtmd::MtmdBitmapError;
use llama_cpp_bindings::mtmd::MtmdEvalError;
use llama_cpp_bindings::mtmd::MtmdTokenizeError;
use log::error;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::oversized_media_details::OversizedMediaDetails;
use paddler_messaging::oversized_prompt_details::OversizedPromptDetails;
use tokio::sync::mpsc;

use paddler_image_decoder::decoded_image_error::DecodedImageError;
use paddler_tool_call_validator::validator_build_error::ValidatorBuildError;

use crate::send_generated_token_result_or_warn::send_generated_token_result_or_warn;

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

    #[error("failed to render chat template: {0:?}")]
    ChatTemplateRenderingFailed(AnyhowError),

    #[error("{0}")]
    ToolSchemaInvalid(#[source] ValidatorBuildError),

    #[error("failed to serialize tools: {0}")]
    ToolsSerializationFailed(#[source] serde_json::Error),

    #[error("failed to build this model's tool call parser: {0}")]
    ToolCallParserCreationFailed(#[source] ParseChatMessageError),

    #[error("failed to tokenize prompt: {0}")]
    PromptTokenizationFailed(#[source] StringToTokenError),

    #[error(
        "prompt has {} tokens but each sequence holds {} tokens",
        details.prompt_tokens,
        details.sequence_context_size
    )]
    PromptExceedsContextSize { details: OversizedPromptDetails },

    #[error("the scheduler is no longer accepting requests")]
    SchedulerUnavailable,

    #[error("no available sequence slots, all slots are busy")]
    NoSequenceSlotAvailable,

    #[error("invalid grammar: {0}")]
    GrammarInvalid(#[source] GrammarError),
    #[error("failed to initialize grammar sampler: {0}")]
    GrammarSamplerInitializationFailed(#[source] GrammarError),

    #[error("failed to create sampler chain: {0}")]
    SamplerChainCreationFailed(#[source] SamplingError),

    #[error("failed to tokenize multimodal input: {0}")]
    MultimodalTokenizationFailed(#[source] MtmdTokenizeError),

    #[error(
        "media chunk has {} tokens but one micro batch holds {} tokens",
        details.media_tokens,
        details.micro_batch_tokens
    )]
    MediaExceedsMicroBatch { details: OversizedMediaDetails },
    #[error("failed to check media chunks against the micro batch: {0}")]
    MediaMicroBatchCheckFailed(#[source] MtmdEvalError),

    #[error("failed to ingest multimodal prompt: {0}")]
    MultimodalIngestionFailed(#[source] EvalMultimodalChunksError),
}

impl GenerationRequestRejection {
    #[must_use]
    pub const fn for_gbnf_grammar_error(grammar_error: GrammarError) -> Self {
        match grammar_error {
            GrammarError::GrammarContainsNul(_)
            | GrammarError::GrammarRejected(_)
            | GrammarError::RootContainsNul(_)
            | GrammarError::RootNotFound => Self::GrammarInvalid(grammar_error),
            GrammarError::FfiContract(_)
            | GrammarError::FfiStatus(_)
            | GrammarError::GrammarMalformed
            | GrammarError::InvalidTriggerPattern { .. }
            | GrammarError::LazyGrammarMalformed
            | GrammarError::LlamaCppOutOfMemory
            | GrammarError::LlguidanceFactoryUnavailable { .. }
            | GrammarError::LlguidanceGrammarInvalid { .. }
            | GrammarError::LlguidanceParserUnavailable { .. }
            | GrammarError::NotEnoughMemory
            | GrammarError::Reported { .. }
            | GrammarError::SamplerInitialization(_)
            | GrammarError::SequenceBreakerContainsNul(_)
            | GrammarError::TokEnvUnavailable(_)
            | GrammarError::TriggerPatternContainsNul(_) => {
                Self::GrammarSamplerInitializationFailed(grammar_error)
            }
        }
    }

    #[must_use]
    pub fn for_micro_batch_fit_error(fit_error: MtmdEvalError) -> Self {
        match fit_error {
            MtmdEvalError::NonCausalChunkExceedsMicroBatch(mismatch) => {
                Self::MediaExceedsMicroBatch {
                    details: OversizedMediaDetails {
                        media_tokens: mismatch.chunk_tokens,
                        micro_batch_tokens: mismatch.micro_batch_tokens,
                    },
                }
            }
            other_fit_error => Self::MediaMicroBatchCheckFailed(other_fit_error),
        }
    }

    pub fn report(
        self,
        agent_name: Option<&str>,
        generated_tokens_tx: &mpsc::UnboundedSender<GeneratedTokenResult>,
    ) {
        send_generated_token_result_or_warn(
            agent_name,
            generated_tokens_tx,
            self.into_generated_token_result(agent_name),
        );
    }

    #[must_use]
    pub fn into_generated_token_result(self, agent_name: Option<&str>) -> GeneratedTokenResult {
        let message = format!("{agent_name:?}: {self}");

        error!("{message}");

        match self {
            Self::GrammarIncompatibleWithThinking => {
                GeneratedTokenResult::GrammarIncompatibleWithThinking(message)
            }
            Self::GrammarConversionFailed(_) | Self::GrammarInvalid(_) => {
                GeneratedTokenResult::GrammarSyntaxError(message)
            }
            Self::TokenGenerationDisabled => GeneratedTokenResult::TokenGenerationDisabled(message),
            Self::ImageDecodingFailed(_) | Self::ImageBitmapCreationFailed(_) => {
                GeneratedTokenResult::ImageDecodingFailed(message)
            }
            Self::MultimodalNotSupported => GeneratedTokenResult::MultimodalNotSupported(message),
            Self::ChatTemplateRenderingFailed(_) | Self::ToolCallParserCreationFailed(_) => {
                GeneratedTokenResult::ChatTemplateError(message)
            }
            Self::ToolSchemaInvalid(_) => GeneratedTokenResult::ToolSchemaInvalid(message),
            Self::GrammarSamplerInitializationFailed(_) => {
                GeneratedTokenResult::GrammarInitializationFailed(message)
            }
            Self::MediaExceedsMicroBatch { details } => {
                GeneratedTokenResult::MediaExceedsMicroBatch(details)
            }
            Self::PromptExceedsContextSize { details } => {
                GeneratedTokenResult::PromptExceedsContextSize(details)
            }
            Self::ToolsSerializationFailed(_)
            | Self::MediaMicroBatchCheckFailed(_)
            | Self::PromptTokenizationFailed(_)
            | Self::SchedulerUnavailable
            | Self::NoSequenceSlotAvailable
            | Self::SamplerChainCreationFailed(_)
            | Self::MultimodalTokenizationFailed(_)
            | Self::MultimodalIngestionFailed(_) => GeneratedTokenResult::SamplerError(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;
    use llama_cpp_bindings::error::EvalMultimodalChunksError;
    use llama_cpp_bindings::error::FfiContractError;
    use llama_cpp_bindings::error::FfiStatusError;
    use llama_cpp_bindings::error::GrammarError;
    use llama_cpp_bindings::error::JsonSchemaToGrammarError;
    use llama_cpp_bindings::error::ParseChatMessageError;
    use llama_cpp_bindings::mtmd::MtmdEvalError;
    use llama_cpp_bindings::mtmd::NonCausalChunkMicroBatchMismatch;
    use paddler_image_decoder::decoded_image_error::DecodedImageError;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::oversized_media_details::OversizedMediaDetails;
    use paddler_messaging::oversized_prompt_details::OversizedPromptDetails;
    use paddler_tool_call_validator::validator_build_error::ValidatorBuildError;
    use tokio::sync::mpsc;

    use super::GenerationRequestRejection;

    fn reported(rejection: GenerationRequestRejection) -> GeneratedTokenResult {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();

        rejection.report(Some("agent"), &generated_tokens_tx);

        generated_tokens_rx.try_recv().unwrap()
    }

    fn agent_message(description: &str) -> String {
        format!("Some(\"agent\"): {description}")
    }

    #[test]
    fn reports_thinking_incompatibility_with_the_agent_name() {
        assert_eq!(
            reported(GenerationRequestRejection::GrammarIncompatibleWithThinking),
            GeneratedTokenResult::GrammarIncompatibleWithThinking(agent_message(
                "Grammar constraints are incompatible with thinking mode"
            ))
        );
    }

    #[test]
    fn reports_grammar_conversion_failure_as_grammar_syntax_error() {
        let conversion_error = JsonSchemaToGrammarError::NotEnoughMemory;
        let expected_message = agent_message(&format!(
            "Failed to convert JSON schema to grammar: {conversion_error}"
        ));

        assert_eq!(
            reported(GenerationRequestRejection::GrammarConversionFailed(
                conversion_error
            )),
            GeneratedTokenResult::GrammarSyntaxError(expected_message)
        );
    }

    #[test]
    fn reports_disabled_token_generation() {
        assert_eq!(
            reported(GenerationRequestRejection::TokenGenerationDisabled),
            GeneratedTokenResult::TokenGenerationDisabled(agent_message(
                "token generation is disabled because this agent is running in embeddings-only mode"
            ))
        );
    }

    #[test]
    fn reports_image_failures_as_image_decoding_failure() {
        assert_eq!(
            reported(GenerationRequestRejection::ImageDecodingFailed(
                DecodedImageError::MissingCommaSeparator,
            )),
            GeneratedTokenResult::ImageDecodingFailed(agent_message(
                "failed to decode images: Invalid data URI: missing comma separator"
            ))
        );
    }

    #[test]
    fn reports_missing_multimodal_support() {
        assert_eq!(
            reported(GenerationRequestRejection::MultimodalNotSupported),
            GeneratedTokenResult::MultimodalNotSupported(agent_message(
                "received images but model does not support multimodal input"
            ))
        );
    }

    #[test]
    fn reports_prompt_rendering_failures_as_chat_template_error() {
        assert_eq!(
            reported(GenerationRequestRejection::ChatTemplateRenderingFailed(
                anyhow!("missing variable"),
            )),
            GeneratedTokenResult::ChatTemplateError(agent_message(
                "failed to render chat template: missing variable"
            ))
        );
    }

    #[test]
    fn reports_invalid_tool_schema() {
        assert_eq!(
            reported(GenerationRequestRejection::ToolSchemaInvalid(
                ValidatorBuildError::InvalidSchema {
                    tool_name: "get_weather".to_owned(),
                    message: "not a schema".to_owned(),
                },
            )),
            GeneratedTokenResult::ToolSchemaInvalid(agent_message(
                "tool \"get_weather\" parameters are not a valid JSON Schema: not a schema"
            ))
        );
    }

    #[test]
    fn reports_grammar_sampler_initialization_failure() {
        let grammar_error = GrammarError::FfiStatus(FfiStatusError {
            operation: "llama_sampler_init_grammar",
            code: 1,
        });
        let expected_message = agent_message(&format!(
            "failed to initialize grammar sampler: {grammar_error}"
        ));

        assert_eq!(
            reported(GenerationRequestRejection::GrammarSamplerInitializationFailed(grammar_error)),
            GeneratedTokenResult::GrammarInitializationFailed(expected_message)
        );
    }

    #[test]
    fn reports_a_grammar_the_client_malformed_as_grammar_syntax_error() {
        let expected_message =
            agent_message(&format!("invalid grammar: {}", GrammarError::RootNotFound));

        assert_eq!(
            reported(GenerationRequestRejection::for_gbnf_grammar_error(
                GrammarError::RootNotFound
            )),
            GeneratedTokenResult::GrammarSyntaxError(expected_message)
        );
    }

    #[test]
    fn reports_a_grammar_the_agent_failed_to_initialize_as_grammar_initialization_failure() {
        let expected_message = agent_message(&format!(
            "failed to initialize grammar sampler: {}",
            GrammarError::NotEnoughMemory
        ));

        assert_eq!(
            reported(GenerationRequestRejection::for_gbnf_grammar_error(
                GrammarError::NotEnoughMemory
            )),
            GeneratedTokenResult::GrammarInitializationFailed(expected_message)
        );
    }

    #[test]
    fn reports_tool_call_parser_creation_failure_as_chat_template_error() {
        let parser_error = ParseChatMessageError::ToolsNotAnArray;
        let expected_message = agent_message(&format!(
            "failed to build this model's tool call parser: {parser_error}"
        ));

        assert_eq!(
            reported(GenerationRequestRejection::ToolCallParserCreationFailed(
                parser_error
            )),
            GeneratedTokenResult::ChatTemplateError(expected_message)
        );
    }

    #[test]
    fn reports_media_exceeding_the_micro_batch_with_its_token_counts() {
        let rejection = GenerationRequestRejection::for_micro_batch_fit_error(
            MtmdEvalError::NonCausalChunkExceedsMicroBatch(NonCausalChunkMicroBatchMismatch {
                chunk_tokens: 256,
                micro_batch_tokens: 128,
            }),
        );

        assert_eq!(
            reported(rejection),
            GeneratedTokenResult::MediaExceedsMicroBatch(OversizedMediaDetails {
                media_tokens: 256,
                micro_batch_tokens: 128,
            })
        );
    }

    #[test]
    fn reports_other_micro_batch_check_failures_as_sampler_error() {
        let fit_error = MtmdEvalError::FfiContract(FfiContractError {
            operation: "mtmd_input_chunks_get",
            detail: "returned a null chunk within the chunk count",
        });
        let expected_message = agent_message(&format!(
            "failed to check media chunks against the micro batch: {fit_error}"
        ));

        assert_eq!(
            reported(GenerationRequestRejection::for_micro_batch_fit_error(
                fit_error
            )),
            GeneratedTokenResult::SamplerError(expected_message)
        );
    }

    #[test]
    fn reports_oversized_prompt_with_its_token_counts() {
        assert_eq!(
            reported(GenerationRequestRejection::PromptExceedsContextSize {
                details: OversizedPromptDetails {
                    prompt_tokens: 9895,
                    sequence_context_size: 8192,
                },
            }),
            GeneratedTokenResult::PromptExceedsContextSize(OversizedPromptDetails {
                prompt_tokens: 9895,
                sequence_context_size: 8192,
            })
        );
    }

    #[test]
    fn reports_other_multimodal_ingestion_failures_as_sampler_error() {
        let rejection = GenerationRequestRejection::MultimodalIngestionFailed(
            EvalMultimodalChunksError::ChunkOutOfBounds(3),
        );

        assert_eq!(
            reported(rejection),
            GeneratedTokenResult::SamplerError(agent_message(
                "failed to ingest multimodal prompt: chunk index 3 out of bounds during post-eval walk"
            ))
        );
    }
}
