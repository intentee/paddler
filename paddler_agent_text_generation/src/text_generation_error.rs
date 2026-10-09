use std::num::TryFromIntError;
use std::path::PathBuf;
use std::str::Utf8Error;

use llama_cpp_bindings::batch_add_error::BatchAddError;
use llama_cpp_bindings::error::ChatTemplateError;
use llama_cpp_bindings::error::ClearKvCacheSeqError;
use llama_cpp_bindings::error::EvalMultimodalChunksError;
use llama_cpp_bindings::error::GrammarError;
use llama_cpp_bindings::error::JsonSchemaToGrammarError;
use llama_cpp_bindings::error::LogitsError;
use llama_cpp_bindings::error::MarkerDetectionError;
use llama_cpp_bindings::error::ParseChatMessageError;
use llama_cpp_bindings::error::SamplerAcceptError;
use llama_cpp_bindings::error::SamplerApplyError;
use llama_cpp_bindings::error::SamplingError;
use llama_cpp_bindings::error::StringToTokenError;
use llama_cpp_bindings::error::TokenToStringError;
use llama_cpp_bindings::mtmd::MtmdBitmapError;
use llama_cpp_bindings::mtmd::MtmdDefaultMarkerError;
use llama_cpp_bindings::mtmd::MtmdEvalError;
use llama_cpp_bindings::mtmd::MtmdInitError;
use llama_cpp_bindings::mtmd::MtmdTokenizeError;
use log::error;
use minijinja::Error as MinijinjaError;
use serde_json::Error;
use tokio::sync::mpsc;

use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_runtime::rejection_description::rejection_description;
use paddler_agent_runtime::send_result_or_warn::send_result_or_warn;
use paddler_image_decoder::decoded_image_error::DecodedImageError;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_messaging::oversized_media_details::OversizedMediaDetails;
use paddler_messaging::oversized_prompt_details::OversizedPromptDetails;
use paddler_tool_call_validator::tool_call_validation_error::ToolCallValidationError;

#[derive(Debug, thiserror::Error)]
pub enum TextGenerationError {
    #[error(transparent)]
    AgentRuntime(#[from] AgentRuntimeError),

    #[error("the model has no usable chat template: {0}")]
    ChatTemplateUnavailable(#[source] ChatTemplateError),

    #[error("the model's chat template is not valid UTF-8: {0}")]
    ChatTemplateNotUtf8(#[source] Utf8Error),

    #[error("the chat template does not compile: {0}")]
    ChatTemplateDoesNotCompile(#[source] MinijinjaError),

    #[error("unable to find the media marker of the multimodal projection: {0}")]
    MediaMarkerUnavailable(#[source] MtmdDefaultMarkerError),

    #[error("unable to load the multimodal projection from {multimodal_projection_path}")]
    MultimodalProjectionLoadFailed {
        multimodal_projection_path: PathBuf,
        #[source]
        source: MtmdInitError,
    },

    #[error("failed to apply the grammar sampler to the token candidates: {0}")]
    GrammarConstraintFailed(#[source] SamplerApplyError),

    #[error("failed to apply the sampler chain to the token candidates: {0}")]
    SamplerChainApplicationFailed(#[source] SamplerApplyError),

    #[error("the sampler chain failed to accept the selected token: {0}")]
    SamplerChainRejectedToken(#[source] SamplerAcceptError),

    #[error("failed to read the token candidates for sampling: {0}")]
    TokenDataUnreadable(#[source] LogitsError),

    #[error("unable to render the model's special token: {0}")]
    SpecialTokenUnrenderable(#[source] TokenToStringError),

    #[error("unable to detect the model's streaming markers: {0}")]
    StreamingMarkersUnavailable(#[source] MarkerDetectionError),

    #[error("Grammar constraints are incompatible with thinking mode")]
    GrammarIncompatibleWithThinking,

    #[error("Failed to convert JSON schema to grammar: {0}")]
    GrammarConversionFailed(#[source] JsonSchemaToGrammarError),

    #[error("the agent serves {serving_inference_mode:?}, so it cannot generate tokens")]
    InferenceModeMismatch {
        serving_inference_mode: InferenceMode,
    },

    #[error("failed to decode images: {0}")]
    ImageDecodingFailed(#[source] DecodedImageError),

    #[error("failed to create image bitmap: {0}")]
    ImageBitmapCreationFailed(#[source] MtmdBitmapError),

    #[error("received images but model does not support multimodal input")]
    MultimodalNotSupported,

    #[error("no model is loaded")]
    ModelNotLoaded,

    #[error("failed to render chat template: {0}")]
    ChatTemplateRenderingFailed(#[source] MinijinjaError),

    #[error("{0}")]
    ToolSchemaInvalid(#[source] ToolCallValidationError),

    #[error("failed to serialize tools: {0}")]
    ToolsSerializationFailed(#[source] Error),

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

    #[error("failed to add a token to the batch: {0}")]
    BatchAssemblyFailed(#[source] BatchAddError),

    #[error("failed to clear the KV cache of the sequence: {0}")]
    KvCacheClearFailed(#[source] ClearKvCacheSeqError),

    #[error("sequence id {sequence_id} cannot address a KV cache sequence: {source}")]
    SequenceIdOutOfRange {
        sequence_id: i32,
        #[source]
        source: TryFromIntError,
    },
}

impl TextGenerationError {
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
        send_result_or_warn(
            agent_name,
            generated_tokens_tx,
            self.into_generated_token_result(agent_name),
        );
    }

    #[must_use]
    pub fn into_generated_token_result(self, agent_name: Option<&str>) -> GeneratedTokenResult {
        let message = rejection_description(agent_name, &self);

        error!("{message}");

        match self {
            Self::GrammarIncompatibleWithThinking => {
                GeneratedTokenResult::GrammarIncompatibleWithThinking(message)
            }
            Self::GrammarConversionFailed(_) | Self::GrammarInvalid(_) => {
                GeneratedTokenResult::GrammarSyntaxError(message)
            }
            Self::InferenceModeMismatch { .. } => {
                GeneratedTokenResult::InferenceModeMismatch(message)
            }
            Self::ImageDecodingFailed(_) | Self::ImageBitmapCreationFailed(_) => {
                GeneratedTokenResult::ImageDecodingFailed(message)
            }
            Self::AgentRuntime(_)
            | Self::MediaMarkerUnavailable(_)
            | Self::ModelNotLoaded
            | Self::MultimodalProjectionLoadFailed { .. }
            | Self::SpecialTokenUnrenderable(_)
            | Self::StreamingMarkersUnavailable(_) => GeneratedTokenResult::ModelNotLoaded(message),
            Self::MultimodalNotSupported => GeneratedTokenResult::MultimodalNotSupported(message),
            Self::ChatTemplateDoesNotCompile(_)
            | Self::ChatTemplateNotUtf8(_)
            | Self::ChatTemplateRenderingFailed(_)
            | Self::ChatTemplateUnavailable(_)
            | Self::ToolCallParserCreationFailed(_) => {
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
            Self::ToolsSerializationFailed(_) => {
                GeneratedTokenResult::ToolsSerializationFailed(message)
            }
            Self::MediaMicroBatchCheckFailed(_) => {
                GeneratedTokenResult::MediaMicroBatchCheckFailed(message)
            }
            Self::PromptTokenizationFailed(_) => {
                GeneratedTokenResult::PromptTokenizationFailed(message)
            }
            Self::SchedulerUnavailable => GeneratedTokenResult::SchedulerUnavailable(message),
            Self::NoSequenceSlotAvailable => GeneratedTokenResult::NoSequenceSlotAvailable(message),
            Self::SamplerChainCreationFailed(_) => {
                GeneratedTokenResult::SamplerChainCreationFailed(message)
            }
            Self::GrammarConstraintFailed(_)
            | Self::SamplerChainApplicationFailed(_)
            | Self::SamplerChainRejectedToken(_)
            | Self::TokenDataUnreadable(_) => GeneratedTokenResult::SamplerError(message),
            Self::MultimodalTokenizationFailed(_) => {
                GeneratedTokenResult::MultimodalTokenizationFailed(message)
            }
            Self::MultimodalIngestionFailed(_) => {
                GeneratedTokenResult::MultimodalIngestionFailed(message)
            }
            Self::BatchAssemblyFailed(_) => GeneratedTokenResult::BatchAssemblyFailed(message),
            Self::KvCacheClearFailed(_) => GeneratedTokenResult::KvCacheClearFailed(message),
            Self::SequenceIdOutOfRange { .. } => {
                GeneratedTokenResult::SequenceIdOutOfRange(message)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use llama_cpp_bindings::batch_add_error::BatchAddError;
    use llama_cpp_bindings::error::ClearKvCacheSeqError;
    use llama_cpp_bindings::error::EvalMultimodalChunksError;
    use llama_cpp_bindings::error::FfiContractError;
    use llama_cpp_bindings::error::FfiStatusError;
    use llama_cpp_bindings::error::GrammarError;
    use llama_cpp_bindings::error::JsonSchemaToGrammarError;
    use llama_cpp_bindings::error::LogitsError;
    use llama_cpp_bindings::error::ParseChatMessageError;
    use llama_cpp_bindings::error::SamplingError;
    use llama_cpp_bindings::error::StringToTokenError;
    use llama_cpp_bindings::mtmd::MtmdEvalError;
    use llama_cpp_bindings::mtmd::MtmdTokenizeError;
    use llama_cpp_bindings::mtmd::NonCausalChunkMicroBatchMismatch;
    use minijinja::Error as MinijinjaError;
    use minijinja::ErrorKind as MinijinjaErrorKind;
    use serde_json::Map;
    use serde_json::Value;
    use serde_json::from_str;
    use serde_json::json;
    use tokio::sync::mpsc;

    use paddler_image_decoder::decoded_image_error::DecodedImageError;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::inference_mode::InferenceMode;
    use paddler_messaging::oversized_media_details::OversizedMediaDetails;
    use paddler_messaging::oversized_prompt_details::OversizedPromptDetails;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::FunctionCall;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::function::Function;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters::Parameters;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
    use paddler_tool_call_validator::tool_call_validator::ToolCallValidator;

    use super::TextGenerationError;

    struct ExpectedReport {
        rejection: TextGenerationError,
        into_result: fn(String) -> GeneratedTokenResult,
    }

    fn reported(rejection: TextGenerationError) -> GeneratedTokenResult {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();

        rejection.report(Some("agent"), &generated_tokens_tx);

        generated_tokens_rx.try_recv().unwrap()
    }

    fn agent_message(description: &str) -> String {
        format!("agent: {description}")
    }

    #[test]
    fn reports_thinking_incompatibility_with_the_agent_name() {
        assert_eq!(
            reported(TextGenerationError::GrammarIncompatibleWithThinking),
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
            reported(TextGenerationError::GrammarConversionFailed(
                conversion_error
            )),
            GeneratedTokenResult::GrammarSyntaxError(expected_message)
        );
    }

    #[test]
    fn reports_a_request_for_another_mode_naming_the_served_mode() {
        assert_eq!(
            reported(TextGenerationError::InferenceModeMismatch {
                serving_inference_mode: InferenceMode::Embeddings,
            }),
            GeneratedTokenResult::InferenceModeMismatch(agent_message(
                "the agent serves Embeddings, so it cannot generate tokens"
            ))
        );
    }

    #[test]
    fn reports_image_failures_as_image_decoding_failure() {
        assert_eq!(
            reported(TextGenerationError::ImageDecodingFailed(
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
            reported(TextGenerationError::MultimodalNotSupported),
            GeneratedTokenResult::MultimodalNotSupported(agent_message(
                "received images but model does not support multimodal input"
            ))
        );
    }

    #[test]
    fn reports_prompt_rendering_failures_as_chat_template_error() {
        let rendering_error =
            MinijinjaError::new(MinijinjaErrorKind::UndefinedError, "missing variable");
        let expected_message = agent_message(&format!(
            "failed to render chat template: {rendering_error}"
        ));

        assert_eq!(
            reported(TextGenerationError::ChatTemplateRenderingFailed(
                rendering_error
            )),
            GeneratedTokenResult::ChatTemplateError(expected_message)
        );
    }

    #[test]
    fn reports_invalid_tool_schema() {
        let mut properties = Map::new();

        properties.insert("location".to_owned(), json!({"type": 123}));

        let schema_error =
            ToolCallValidator::from_tools(&[Tool::from(&Tool::Function(FunctionCall {
                function: Function {
                    name: "get_weather".to_owned(),
                    description: "fetch weather".to_owned(),
                    parameters: Parameters::Schema(ValidatedParametersSchema {
                        schema_type: "object".to_owned(),
                        properties: Some(properties),
                        ..ValidatedParametersSchema::default()
                    }),
                },
            }))])
            .err()
            .expect("a property typed as a number is not a valid JSON Schema");
        let expected_message = agent_message(&schema_error.to_string());

        assert_eq!(
            reported(TextGenerationError::ToolSchemaInvalid(schema_error)),
            GeneratedTokenResult::ToolSchemaInvalid(expected_message)
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
            reported(TextGenerationError::GrammarSamplerInitializationFailed(
                grammar_error
            )),
            GeneratedTokenResult::GrammarInitializationFailed(expected_message)
        );
    }

    #[test]
    fn reports_a_grammar_the_client_malformed_as_grammar_syntax_error() {
        let expected_message =
            agent_message(&format!("invalid grammar: {}", GrammarError::RootNotFound));

        assert_eq!(
            reported(TextGenerationError::for_gbnf_grammar_error(
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
            reported(TextGenerationError::for_gbnf_grammar_error(
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
            reported(TextGenerationError::ToolCallParserCreationFailed(
                parser_error
            )),
            GeneratedTokenResult::ChatTemplateError(expected_message)
        );
    }

    #[test]
    fn reports_media_exceeding_the_micro_batch_with_its_token_counts() {
        let rejection = TextGenerationError::for_micro_batch_fit_error(
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
    fn reports_other_micro_batch_check_failures_as_micro_batch_check_failures() {
        let fit_error = MtmdEvalError::FfiContract(FfiContractError {
            operation: "mtmd_input_chunks_get",
            detail: "returned a null chunk within the chunk count",
        });
        let expected_message = agent_message(&format!(
            "failed to check media chunks against the micro batch: {fit_error}"
        ));

        assert_eq!(
            reported(TextGenerationError::for_micro_batch_fit_error(fit_error)),
            GeneratedTokenResult::MediaMicroBatchCheckFailed(expected_message)
        );
    }

    #[test]
    fn reports_oversized_prompt_with_its_token_counts() {
        assert_eq!(
            reported(TextGenerationError::PromptExceedsContextSize {
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
    fn reports_multimodal_ingestion_failures_as_ingestion_failures() {
        let rejection = TextGenerationError::MultimodalIngestionFailed(
            EvalMultimodalChunksError::ChunkOutOfBounds(3),
        );

        assert_eq!(
            reported(rejection),
            GeneratedTokenResult::MultimodalIngestionFailed(agent_message(
                "failed to ingest multimodal prompt: chunk index 3 out of bounds during post-eval walk"
            ))
        );
    }

    #[test]
    fn reports_internal_scheduler_failures_under_their_own_results() {
        for ExpectedReport {
            rejection,
            into_result,
        } in [
            ExpectedReport {
                rejection: TextGenerationError::BatchAssemblyFailed(BatchAddError::EmptyBuffer),
                into_result: GeneratedTokenResult::BatchAssemblyFailed,
            },
            ExpectedReport {
                rejection: TextGenerationError::KvCacheClearFailed(
                    ClearKvCacheSeqError::PartialSequenceNotRemoved {
                        seq_id: 1,
                        p0: 0,
                        p1: 4,
                    },
                ),
                into_result: GeneratedTokenResult::KvCacheClearFailed,
            },
            ExpectedReport {
                rejection: TextGenerationError::ModelNotLoaded,
                into_result: GeneratedTokenResult::ModelNotLoaded,
            },
            ExpectedReport {
                rejection: TextGenerationError::MultimodalTokenizationFailed(
                    MtmdTokenizeError::BitmapCountDoesNotMatchMarkerCount,
                ),
                into_result: GeneratedTokenResult::MultimodalTokenizationFailed,
            },
            ExpectedReport {
                rejection: TextGenerationError::NoSequenceSlotAvailable,
                into_result: GeneratedTokenResult::NoSequenceSlotAvailable,
            },
            ExpectedReport {
                rejection: TextGenerationError::PromptTokenizationFailed(
                    StringToTokenError::NulError(CString::new("a \0 byte").unwrap_err()),
                ),
                into_result: GeneratedTokenResult::PromptTokenizationFailed,
            },
            ExpectedReport {
                rejection: TextGenerationError::SamplerChainCreationFailed(
                    SamplingError::SamplerUnavailable { sampler: "grammar" },
                ),
                into_result: GeneratedTokenResult::SamplerChainCreationFailed,
            },
            ExpectedReport {
                rejection: TextGenerationError::SequenceIdOutOfRange {
                    sequence_id: -1,
                    source: u32::try_from(-1_i32).unwrap_err(),
                },
                into_result: GeneratedTokenResult::SequenceIdOutOfRange,
            },
            ExpectedReport {
                rejection: TextGenerationError::TokenDataUnreadable(LogitsError::NullLogits),
                into_result: GeneratedTokenResult::SamplerError,
            },
            ExpectedReport {
                rejection: TextGenerationError::ToolsSerializationFailed(
                    from_str::<Value>("{").unwrap_err(),
                ),
                into_result: GeneratedTokenResult::ToolsSerializationFailed,
            },
        ] {
            let expected_message = agent_message(&rejection.to_string());

            assert_eq!(reported(rejection), into_result(expected_message));
        }
    }
}
