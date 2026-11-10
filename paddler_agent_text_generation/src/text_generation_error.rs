use std::ffi::NulError;
use std::path::PathBuf;

use llama_cpp_bindings::batch_add_error::BatchAddError;
use llama_cpp_bindings::error::chat_session_error::ChatSessionError;
use llama_cpp_bindings::error::chat_templates_error::ChatTemplatesError;
use llama_cpp_bindings::error::eval_multimodal_chunks_error::EvalMultimodalChunksError;
use llama_cpp_bindings::error::grammar_error::GrammarError;
use llama_cpp_bindings::error::ingest_error::IngestError;
use llama_cpp_bindings::error::json_schema_to_grammar_error::JsonSchemaToGrammarError;
use llama_cpp_bindings::error::logits_error::LogitsError;
use llama_cpp_bindings::error::memory_seq_rm_error::MemorySeqRmError;
use llama_cpp_bindings::error::parse_chat_message_error::ParseChatMessageError;
use llama_cpp_bindings::error::sample_error::SampleError;
use llama_cpp_bindings::error::sampler_accept_error::SamplerAcceptError;
use llama_cpp_bindings::error::sampling_error::SamplingError;
use llama_cpp_bindings::error::string_to_token_error::StringToTokenError;
use llama_cpp_bindings::error::token_to_string_error::TokenToStringError;
use llama_cpp_bindings::mtmd::mtmd_bitmap_error::MtmdBitmapError;
use llama_cpp_bindings::mtmd::mtmd_eval_error::MtmdEvalError;
use llama_cpp_bindings::mtmd::mtmd_init_error::MtmdInitError;
use llama_cpp_bindings::mtmd::mtmd_tokenize_error::MtmdTokenizeError;
use log::error;
use tokio::sync::mpsc;

use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_runtime::rejection_description::rejection_description;
use paddler_agent_runtime::send_result_or_warn::send_result_or_warn;
use paddler_image_decoder::decoded_image_error::DecodedImageError;
use paddler_inference_parameters::invalid_inference_parameters::InvalidInferenceParameters;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::oversized_media_details::OversizedMediaDetails;
use paddler_messaging::oversized_prompt_details::OversizedPromptDetails;
use paddler_tool_call_validator::tool_call_validation_error::ToolCallValidationError;

#[derive(Debug, thiserror::Error)]
pub enum TextGenerationError {
    #[error(transparent)]
    AgentRuntime(#[from] AgentRuntimeError),

    #[error(
        "the model {} has no chat template, so it cannot continue a conversation",
        model_path.model_path
    )]
    ConversationRequiresChatTemplate { model_path: ModelPath },

    #[error("llama.cpp cannot prepare the chat template: {0}")]
    ChatTemplateDoesNotCompile(#[source] ChatTemplatesError),

    #[error("the chat template holds a NUL character: {0}")]
    ChatTemplateContainsNul(#[source] NulError),

    #[error("the chat template cannot render the conversation: {0}")]
    ChatSessionCreationFailed(#[source] ChatSessionError),

    #[error("unable to read the rendered prompt: {0}")]
    ChatPromptUnreadable(#[source] ChatSessionError),

    #[error("failed to tokenize the rendered prompt: {0}")]
    ChatPromptTokenizationFailed(#[source] ChatSessionError),

    #[error("failed to build the grammar that constrains the tool calls: {0}")]
    ChatSessionGrammarUnavailable(#[source] ChatSessionError),

    #[error("unable to start classifying the answer: {0}")]
    AnswerClassifierUnavailable(#[source] ParseChatMessageError),

    #[error("failed to classify the answer: {0}")]
    AnswerClassificationFailed(#[source] ParseChatMessageError),

    #[error("failed to detokenize a sampled token: {0}")]
    TokenDetokenizationFailed(#[source] TokenToStringError),

    #[error("unable to load the multimodal projection from {multimodal_projection_path}: {source}")]
    MultimodalProjectionLoadFailed {
        multimodal_projection_path: PathBuf,
        #[source]
        source: MtmdInitError,
    },

    #[error("failed to apply the grammar sampler to the token candidates: {0}")]
    GrammarConstraintFailed(#[source] SampleError),

    #[error("failed to apply the sampler chain to the token candidates: {0}")]
    SamplerChainApplicationFailed(#[source] SampleError),

    #[error("the sampler chain failed to accept the selected token: {0}")]
    SamplerChainRejectedToken(#[source] SamplerAcceptError),

    #[error("failed to read the token candidates for sampling: {0}")]
    TokenDataUnreadable(#[source] LogitsError),

    #[error("the sampling parameters do not fit the sequence context: {0}")]
    SamplingParametersUnfitForTheSequenceContext(#[source] InvalidInferenceParameters),

    #[error("Grammar constraints are incompatible with thinking mode")]
    GrammarIncompatibleWithThinking,

    #[error("Failed to convert JSON schema to grammar: {0}")]
    GrammarConversionFailed(#[source] JsonSchemaToGrammarError),

    #[error("failed to decode images: {0}")]
    ImageDecodingFailed(#[source] DecodedImageError),

    #[error("failed to create image bitmap: {0}")]
    ImageBitmapCreationFailed(#[source] MtmdBitmapError),

    #[error("received images but model does not support multimodal input")]
    MultimodalNotSupported,

    #[error("no model is loaded")]
    ModelNotLoaded,

    #[error("{0}")]
    ToolSchemaInvalid(#[source] ToolCallValidationError),

    #[error("failed to tokenize prompt: {0}")]
    PromptTokenizationFailed(#[source] StringToTokenError),

    #[error("the prompt tokenized to no tokens, so there is nothing to generate from")]
    PromptTokenizedToNothing,

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
        "the chat template rendered {rendered_media_markers} media markers for {images} images"
    )]
    RenderedMediaMarkersMismatchImages {
        images: usize,
        rendered_media_markers: usize,
    },

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
    KvCacheClearFailed(#[source] MemorySeqRmError),
}

impl TextGenerationError {
    #[must_use]
    pub const fn for_gbnf_grammar_error(grammar_error: GrammarError) -> Self {
        match grammar_error {
            GrammarError::GrammarContainsNul(_)
            | GrammarError::GrammarRejected(_)
            | GrammarError::RootContainsNul(_)
            | GrammarError::RootNotFound => Self::GrammarInvalid(grammar_error),
            GrammarError::CppString(_)
            | GrammarError::FfiContract(_)
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
            | GrammarError::TokEnvUnavailable(_)
            | GrammarError::TriggerPatternContainsNul(_) => {
                Self::GrammarSamplerInitializationFailed(grammar_error)
            }
        }
    }

    #[must_use]
    pub fn for_ingest_error(ingest_error: IngestError) -> Self {
        match ingest_error {
            IngestError::Detokenize(detokenization_error) => {
                Self::TokenDetokenizationFailed(detokenization_error)
            }
            IngestError::Classify(classification_error) => {
                Self::AnswerClassificationFailed(classification_error)
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
            Self::ImageDecodingFailed(_) | Self::ImageBitmapCreationFailed(_) => {
                GeneratedTokenResult::ImageDecodingFailed(message)
            }
            Self::AgentRuntime(_)
            | Self::ModelNotLoaded
            | Self::MultimodalProjectionLoadFailed { .. }
            | Self::SamplingParametersUnfitForTheSequenceContext(_) => {
                GeneratedTokenResult::ModelNotLoaded(message)
            }
            Self::MultimodalNotSupported => GeneratedTokenResult::MultimodalNotSupported(message),
            Self::ChatTemplateDoesNotCompile(_)
            | Self::ChatTemplateContainsNul(_)
            | Self::ChatSessionCreationFailed(_)
            | Self::ChatPromptUnreadable(_)
            | Self::ConversationRequiresChatTemplate { .. } => {
                GeneratedTokenResult::ChatTemplateError(message)
            }
            Self::AnswerClassifierUnavailable(_) | Self::AnswerClassificationFailed(_) => {
                GeneratedTokenResult::AnswerClassificationFailed(message)
            }
            Self::TokenDetokenizationFailed(_) => {
                GeneratedTokenResult::DetokenizationFailed(message)
            }
            Self::ToolSchemaInvalid(_) => GeneratedTokenResult::ToolSchemaInvalid(message),
            Self::GrammarSamplerInitializationFailed(_)
            | Self::ChatSessionGrammarUnavailable(_) => {
                GeneratedTokenResult::GrammarInitializationFailed(message)
            }
            Self::MediaExceedsMicroBatch { details } => {
                GeneratedTokenResult::MediaExceedsMicroBatch(details)
            }
            Self::PromptExceedsContextSize { details } => {
                GeneratedTokenResult::PromptExceedsContextSize(details)
            }
            Self::MediaMicroBatchCheckFailed(_) => {
                GeneratedTokenResult::MediaMicroBatchCheckFailed(message)
            }
            Self::PromptTokenizationFailed(_) | Self::ChatPromptTokenizationFailed(_) => {
                GeneratedTokenResult::PromptTokenizationFailed(message)
            }
            Self::PromptTokenizedToNothing => {
                GeneratedTokenResult::PromptTokenizedToNothing(message)
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
            Self::MultimodalTokenizationFailed(_)
            | Self::RenderedMediaMarkersMismatchImages { .. } => {
                GeneratedTokenResult::MultimodalTokenizationFailed(message)
            }
            Self::MultimodalIngestionFailed(_) => {
                GeneratedTokenResult::MultimodalIngestionFailed(message)
            }
            Self::BatchAssemblyFailed(_) => GeneratedTokenResult::BatchAssemblyFailed(message),
            Self::KvCacheClearFailed(_) => GeneratedTokenResult::KvCacheClearFailed(message),
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
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use llama_cpp_bindings::batch_add_error::BatchAddError;
    use llama_cpp_bindings::error::chat_session_error::ChatSessionError;
    use llama_cpp_bindings::error::chat_templates_error::ChatTemplatesError;
    use llama_cpp_bindings::error::eval_multimodal_chunks_error::EvalMultimodalChunksError;
    use llama_cpp_bindings::error::grammar_error::GrammarError;
    use llama_cpp_bindings::error::ingest_error::IngestError;
    use llama_cpp_bindings::error::json_schema_to_grammar_error::JsonSchemaToGrammarError;
    use llama_cpp_bindings::error::logits_error::LogitsError;
    use llama_cpp_bindings::error::memory_seq_rm_error::MemorySeqRmError;
    use llama_cpp_bindings::error::parse_chat_message_error::ParseChatMessageError;
    use llama_cpp_bindings::error::sampling_error::SamplingError;
    use llama_cpp_bindings::error::string_to_token_error::StringToTokenError;
    use llama_cpp_bindings::error::token_to_string_error::TokenToStringError;
    use llama_cpp_bindings::mtmd::mtmd_eval_error::MtmdEvalError;
    use llama_cpp_bindings::mtmd::mtmd_tokenize_error::MtmdTokenizeError;
    use llama_cpp_bindings::mtmd::non_causal_chunk_micro_batch_mismatch::NonCausalChunkMicroBatchMismatch;
    use llama_cpp_ffi_status::ffi_contract_error::FfiContractError;
    use llama_cpp_ffi_status::ffi_status_error::FfiStatusError;
    use serde_json::Map;
    use serde_json::json;
    use tokio::sync::mpsc;

    use paddler_image_decoder::decoded_image_error::DecodedImageError;
    use paddler_messaging::agent_issue_params::model_path::ModelPath;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::oversized_media_details::OversizedMediaDetails;
    use paddler_messaging::oversized_prompt_details::OversizedPromptDetails;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::FunctionCall;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::function::Function;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters::Parameters;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
    use paddler_tool_call_validator::tool_call_validator::ToolCallValidator;

    use super::TextGenerationError;

    fn reported(rejection: TextGenerationError) -> GeneratedTokenResult {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();

        rejection.report(Some("agent"), &generated_tokens_tx);

        generated_tokens_rx.try_recv().unwrap()
    }

    fn agent_message(description: &str) -> String {
        format!("agent: {description}")
    }

    fn assert_each_reported_as(
        into_result: fn(String) -> GeneratedTokenResult,
        rejections: impl IntoIterator<Item = TextGenerationError>,
    ) {
        for rejection in rejections {
            let expected_message = agent_message(&rejection.to_string());

            assert_eq!(reported(rejection), into_result(expected_message));
        }
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
    fn reports_a_conversation_on_a_model_without_a_chat_template_as_a_chat_template_error() {
        assert_eq!(
            reported(TextGenerationError::ConversationRequiresChatTemplate {
                model_path: ModelPath {
                    model_path: "nomic.gguf".to_owned(),
                },
            }),
            GeneratedTokenResult::ChatTemplateError(agent_message(
                "the model nomic.gguf has no chat template, so it cannot continue a conversation"
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
    fn reports_invalid_tool_schema() {
        let mut properties = Map::new();

        properties.insert("location".to_owned(), json!({"type": 123}));

        let schema_error =
            ToolCallValidator::from_tools(&[Tool::from(Tool::Function(FunctionCall {
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
        assert_each_reported_as(
            GeneratedTokenResult::BatchAssemblyFailed,
            [TextGenerationError::BatchAssemblyFailed(
                BatchAddError::NoSequenceIds,
            )],
        );
        assert_each_reported_as(
            GeneratedTokenResult::KvCacheClearFailed,
            [TextGenerationError::KvCacheClearFailed(
                MemorySeqRmError::PartialSequenceNotRemoved {
                    seq_id: 1,
                    p0: 0,
                    p1: 4,
                },
            )],
        );
        assert_each_reported_as(
            GeneratedTokenResult::NoSequenceSlotAvailable,
            [TextGenerationError::NoSequenceSlotAvailable],
        );
    }

    #[test]
    fn reports_a_missing_model_as_model_not_loaded() {
        assert_each_reported_as(
            GeneratedTokenResult::ModelNotLoaded,
            [TextGenerationError::ModelNotLoaded],
        );
    }

    #[test]
    fn reports_chat_template_failures_as_chat_template_errors() {
        assert_each_reported_as(
            GeneratedTokenResult::ChatTemplateError,
            [
                TextGenerationError::ChatTemplateDoesNotCompile(
                    ChatTemplatesError::ModelHasNoChatTemplate,
                ),
                TextGenerationError::ChatTemplateContainsNul(CString::new("{{ \0 }}").unwrap_err()),
                TextGenerationError::ChatSessionCreationFailed(
                    ChatSessionError::TemplateCannotServeRequest {
                        message: "roles must alternate".to_owned(),
                    },
                ),
                TextGenerationError::ChatPromptUnreadable(ChatSessionError::NotEnoughMemory),
            ],
        );
    }

    #[test]
    fn reports_prompt_tokenization_failures_as_prompt_tokenization_failures() {
        assert_each_reported_as(
            GeneratedTokenResult::PromptTokenizationFailed,
            [
                TextGenerationError::ChatPromptTokenizationFailed(
                    ChatSessionError::PromptTokenization(StringToTokenError::NulError(
                        CString::new("a \0 byte").unwrap_err(),
                    )),
                ),
                TextGenerationError::PromptTokenizationFailed(StringToTokenError::NulError(
                    CString::new("a \0 byte").unwrap_err(),
                )),
            ],
        );
    }

    #[test]
    fn reports_an_unavailable_chat_session_grammar_as_a_grammar_initialization_failure() {
        assert_each_reported_as(
            GeneratedTokenResult::GrammarInitializationFailed,
            [TextGenerationError::ChatSessionGrammarUnavailable(
                ChatSessionError::Grammar(GrammarError::NotEnoughMemory),
            )],
        );
    }

    #[test]
    fn reports_answer_classification_failures_as_answer_classification_failures() {
        assert_each_reported_as(
            GeneratedTokenResult::AnswerClassificationFailed,
            [
                TextGenerationError::AnswerClassifierUnavailable(
                    ParseChatMessageError::NotEnoughMemory,
                ),
                TextGenerationError::for_ingest_error(IngestError::Classify(
                    ParseChatMessageError::LlamaCppOutOfMemory,
                )),
            ],
        );
    }

    #[test]
    fn reports_a_detokenization_failure_as_a_detokenization_failure() {
        assert_each_reported_as(
            GeneratedTokenResult::DetokenizationFailed,
            [TextGenerationError::for_ingest_error(
                IngestError::Detokenize(TokenToStringError::InsufficientBufferSpace(-4)),
            )],
        );
    }

    #[test]
    fn reports_multimodal_tokenization_failures_as_multimodal_tokenization_failures() {
        assert_each_reported_as(
            GeneratedTokenResult::MultimodalTokenizationFailed,
            [
                TextGenerationError::MultimodalTokenizationFailed(
                    MtmdTokenizeError::BitmapCountDoesNotMatchMarkerCount,
                ),
                TextGenerationError::RenderedMediaMarkersMismatchImages {
                    images: 2,
                    rendered_media_markers: 1,
                },
            ],
        );
    }

    #[test]
    fn reports_a_sampler_chain_creation_failure_as_a_sampler_chain_creation_failure() {
        assert_each_reported_as(
            GeneratedTokenResult::SamplerChainCreationFailed,
            [TextGenerationError::SamplerChainCreationFailed(
                SamplingError::SamplerUnavailable { sampler: "grammar" },
            )],
        );
    }

    #[test]
    fn reports_unreadable_token_data_as_a_sampler_error() {
        assert_each_reported_as(
            GeneratedTokenResult::SamplerError,
            [TextGenerationError::TokenDataUnreadable(
                LogitsError::NullLogits,
            )],
        );
    }
}
