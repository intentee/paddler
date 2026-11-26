use base64::DecodeError;
use llama_cpp_bindings::llama_cpp_error::LlamaCppError;
use llama_cpp_chat::chat_error::ChatError;
use llama_cpp_context::context_error::ContextError;
use llama_cpp_context_memory::context_memory_error::ContextMemoryError;
use llama_cpp_gbnf::gbnf_error::GbnfError;
use llama_cpp_model::model_error::ModelError;
use llama_cpp_mtmd::mtmd_error::MtmdError;
use llama_cpp_sampler::sampler_error::SamplerError;
use log::error;
use tokio::sync::mpsc;

use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_runtime::rejection_description::rejection_description;
use paddler_agent_runtime::send_result_or_warn::send_result_or_warn;
use paddler_image_decoder::decoded_image_error::DecodedImageError;
use paddler_inference_parameters::invalid_inference_parameters::InvalidInferenceParameters;
use paddler_messaging::conversation_feature::ConversationFeature;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::media_kind::MediaKind;
use paddler_messaging::oversized_media_details::OversizedMediaDetails;
use paddler_messaging::oversized_prompt_details::OversizedPromptDetails;
use paddler_messaging::undecodable_media::UndecodableMedia;
use paddler_messaging::unsupported_conversation_feature::UnsupportedConversationFeature;
use paddler_tool_call_validator::tool_call_validation_error::ToolCallValidationError;

const fn conversation_feature_not_supported(
    feature: ConversationFeature,
    description: String,
) -> GeneratedTokenResult {
    GeneratedTokenResult::ConversationFeatureNotSupported(UnsupportedConversationFeature {
        description,
        feature,
    })
}

const fn media_decoding_failed(media: MediaKind, description: String) -> GeneratedTokenResult {
    GeneratedTokenResult::MediaDecodingFailed(UndecodableMedia { description, media })
}

#[derive(Debug, thiserror::Error)]
pub enum TextGenerationError {
    #[error(transparent)]
    AgentRuntime(#[from] AgentRuntimeError),

    #[error("llama.cpp cannot read the audio")]
    AudioBitmapCreationFailed(#[source] MtmdError),

    #[error("the audio data is not valid base64")]
    AudioDataNotBase64(#[source] DecodeError),

    #[error("the model of the agent cannot hear audio")]
    AudioInputUnsupported,

    #[error("the model has no chat template, so it cannot continue a conversation")]
    ConversationUnsupported,

    #[error("the chat template cannot render the conversation")]
    ChatSessionCreationFailed(#[source] ChatError),

    #[error("unable to read the rendered prompt")]
    ChatPromptUnreadable(#[source] ChatError),

    #[error("failed to tokenize the rendered prompt")]
    ChatPromptTokenizationFailed(#[source] ChatError),

    #[error("failed to build the grammar that constrains the tool calls")]
    ChatSessionGrammarUnavailable(#[source] ChatError),

    #[error("unable to start classifying the answer")]
    AnswerClassifierUnavailable(#[source] LlamaCppError),

    #[error("failed to classify the answer")]
    AnswerClassificationFailed(#[source] ChatError),

    #[error("failed to detokenize a sampled token")]
    TokenDetokenizationFailed(#[source] ModelError),

    #[error("failed to classify a sampled token")]
    TokenIngestionFailed(#[source] LlamaCppError),

    #[error("failed to apply the grammar sampler to the token candidates")]
    GrammarConstraintFailed(#[source] SamplerError),

    #[error("failed to apply the sampler chain to the token candidates")]
    SamplerChainApplicationFailed(#[source] SamplerError),

    #[error("the sampler chain failed to accept the selected token")]
    SamplerChainRejectedToken(#[source] SamplerError),

    #[error("failed to read the token candidates for sampling")]
    TokenDataUnreadable(#[source] ContextError),

    #[error("the sampling parameters do not fit the sequence context")]
    SamplingParametersUnfitForTheSequenceContext(#[source] InvalidInferenceParameters),

    #[error("Grammar constraints are incompatible with thinking mode")]
    GrammarIncompatibleWithThinking,

    #[error("Failed to convert JSON schema to grammar")]
    GrammarConversionFailed(#[source] GbnfError),

    #[error("failed to decode images")]
    ImageDecodingFailed(#[source] DecodedImageError),

    #[error("failed to create image bitmap")]
    ImageBitmapCreationFailed(#[source] MtmdError),

    #[error("received images but model does not support multimodal input")]
    MultimodalNotSupported,

    #[error("the model of the agent generates no text")]
    TextGenerationNotServed,

    #[error("the chat template of the model does not support thinking")]
    ThinkingUnsupported,

    #[error("the chat template of the model does not support tool calls")]
    ToolCallingUnsupported,

    #[error(
        "the chat template of the model makes one tool call per turn, so it cannot call tools in parallel"
    )]
    ParallelToolCallingUnsupported,

    #[error("no model is loaded")]
    ModelNotLoaded,

    #[error(transparent)]
    ToolSchemaInvalid(ToolCallValidationError),

    #[error("failed to tokenize prompt")]
    PromptTokenizationFailed(#[source] ModelError),

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

    #[error("invalid grammar")]
    GrammarInvalid(#[source] SamplerError),

    #[error("failed to initialize grammar sampler")]
    GrammarSamplerInitializationFailed(#[source] SamplerError),

    #[error("failed to create sampler chain")]
    SamplerChainCreationFailed(#[source] SamplerError),

    #[error("failed to tokenize multimodal input")]
    MultimodalTokenizationFailed(#[source] MtmdError),

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

    #[error("failed to ingest multimodal prompt")]
    MultimodalIngestionFailed(#[source] LlamaCppError),

    #[error("failed to add a token to the batch")]
    BatchAssemblyFailed(#[source] ContextError),

    #[error("failed to clear the KV cache of the sequence")]
    KvCacheClearFailed(#[source] ContextMemoryError),
}

impl TextGenerationError {
    #[must_use]
    pub const fn for_gbnf_grammar_error(grammar_error: SamplerError) -> Self {
        match grammar_error {
            SamplerError::GrammarContainsNul(_)
            | SamplerError::GrammarRejected(_)
            | SamplerError::RootContainsNul(_)
            | SamplerError::RootNotFound => Self::GrammarInvalid(grammar_error),
            sampler_initialization_error => {
                Self::GrammarSamplerInitializationFailed(sampler_initialization_error)
            }
        }
    }

    #[must_use]
    pub fn for_ingest_error(ingest_error: LlamaCppError) -> Self {
        match ingest_error {
            LlamaCppError::Detokenize(detokenization_error) => {
                Self::TokenDetokenizationFailed(detokenization_error)
            }
            LlamaCppError::Classify(classification_error) => {
                Self::AnswerClassificationFailed(classification_error)
            }
            other_ingest_error => Self::TokenIngestionFailed(other_ingest_error),
        }
    }

    #[must_use]
    pub fn for_multimodal_evaluation_error(evaluation_error: LlamaCppError) -> Self {
        match evaluation_error {
            LlamaCppError::MultimodalChunksEval(MtmdError::NonCausalChunkExceedsMicroBatch(
                mismatch,
            )) => Self::MediaExceedsMicroBatch {
                details: OversizedMediaDetails {
                    media_tokens: mismatch.chunk_tokens,
                    micro_batch_tokens: mismatch.micro_batch_tokens,
                },
            },
            other_evaluation_error => Self::MultimodalIngestionFailed(other_evaluation_error),
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
                media_decoding_failed(MediaKind::Image, message)
            }
            Self::AudioBitmapCreationFailed(_) | Self::AudioDataNotBase64(_) => {
                media_decoding_failed(MediaKind::Audio, message)
            }
            Self::AudioInputUnsupported => {
                conversation_feature_not_supported(ConversationFeature::AudioInput, message)
            }
            Self::AgentRuntime(_)
            | Self::ModelNotLoaded
            | Self::SamplingParametersUnfitForTheSequenceContext(_) => {
                GeneratedTokenResult::ModelNotLoaded(message)
            }
            Self::MultimodalNotSupported => GeneratedTokenResult::MultimodalNotSupported(message),
            Self::ConversationUnsupported => {
                conversation_feature_not_supported(ConversationFeature::ChatTemplate, message)
            }
            Self::ParallelToolCallingUnsupported => conversation_feature_not_supported(
                ConversationFeature::ParallelToolCalling,
                message,
            ),
            Self::TextGenerationNotServed => GeneratedTokenResult::TextGenerationNotServed(message),
            Self::ThinkingUnsupported => {
                conversation_feature_not_supported(ConversationFeature::Thinking, message)
            }
            Self::ToolCallingUnsupported => {
                conversation_feature_not_supported(ConversationFeature::ToolCalling, message)
            }
            Self::ChatSessionCreationFailed(_) | Self::ChatPromptUnreadable(_) => {
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
            Self::ChatPromptTokenizationFailed(_) | Self::PromptTokenizationFailed(_) => {
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
            | Self::TokenDataUnreadable(_)
            | Self::TokenIngestionFailed(_) => GeneratedTokenResult::SamplerError(message),
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

    pub fn report_model_not_loaded(
        agent_name: Option<&str>,
        generated_tokens_tx: &mpsc::UnboundedSender<GeneratedTokenResult>,
    ) {
        Self::ModelNotLoaded.report(agent_name, generated_tokens_tx);
    }

    pub fn report_text_generation_not_served(
        agent_name: Option<&str>,
        generated_tokens_tx: &mpsc::UnboundedSender<GeneratedTokenResult>,
    ) {
        Self::TextGenerationNotServed.report(agent_name, generated_tokens_tx);
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;
    use std::io;
    use std::mem::discriminant;

    use base64::DecodeError;
    use llama_cpp_bindings::llama_cpp_error::LlamaCppError;
    use llama_cpp_chat::chat_error::ChatError;
    use llama_cpp_context::context_error::ContextError;
    use llama_cpp_context_memory::context_memory_error::ContextMemoryError;
    use llama_cpp_gbnf::gbnf_error::GbnfError;
    use llama_cpp_model::model_error::ModelError;
    use llama_cpp_mtmd::mtmd_error::MtmdError;
    use llama_cpp_mtmd::non_causal_chunk_micro_batch_mismatch::NonCausalChunkMicroBatchMismatch;
    use llama_cpp_sampler::sampler_error::SamplerError;
    use serde_json::Map;
    use serde_json::json;
    use tokio::sync::mpsc;

    use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
    use paddler_image_decoder::decoded_image_error::DecodedImageError;
    use paddler_inference_parameters::invalid_inference_parameters::InvalidInferenceParameters;
    use paddler_messaging::conversation_feature::ConversationFeature;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::media_kind::MediaKind;
    use paddler_messaging::oversized_media_details::OversizedMediaDetails;
    use paddler_messaging::oversized_prompt_details::OversizedPromptDetails;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::FunctionCall;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::function::Function;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters::Parameters;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
    use paddler_messaging::undecodable_media::UndecodableMedia;
    use paddler_messaging::unsupported_conversation_feature::UnsupportedConversationFeature;
    use paddler_tool_call_validator::tool_call_validator::ToolCallValidator;

    use super::TextGenerationError;

    fn reported(rejection: TextGenerationError) -> GeneratedTokenResult {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();

        rejection.report(Some("agent"), &generated_tokens_tx);

        generated_tokens_rx.try_recv().unwrap()
    }

    fn assert_each_reported_as(
        into_result: fn(String) -> GeneratedTokenResult,
        rejections: impl IntoIterator<Item = TextGenerationError>,
    ) {
        for rejection in rejections {
            assert_eq!(
                discriminant(&reported(rejection)),
                discriminant(&into_result(String::new()))
            );
        }
    }

    #[test]
    fn reports_thinking_incompatibility_as_thinking_incompatibility() {
        assert_each_reported_as(
            GeneratedTokenResult::GrammarIncompatibleWithThinking,
            [TextGenerationError::GrammarIncompatibleWithThinking],
        );
    }

    #[test]
    fn reports_client_grammar_failures_as_grammar_syntax_errors() {
        assert_each_reported_as(
            GeneratedTokenResult::GrammarSyntaxError,
            [
                TextGenerationError::GrammarConversionFailed(GbnfError::InvalidJson {
                    message: "not json".to_owned(),
                }),
                TextGenerationError::for_gbnf_grammar_error(SamplerError::GrammarRejected(
                    GbnfError::SyntaxError,
                )),
            ],
        );
    }

    #[test]
    fn a_grammar_the_client_malformed_is_invalid() {
        for client_grammar_error in [
            SamplerError::GrammarContainsNul(CString::new("a \0 grammar").unwrap_err()),
            SamplerError::GrammarRejected(GbnfError::SyntaxError),
            SamplerError::RootContainsNul(CString::new("a \0 root").unwrap_err()),
            SamplerError::RootNotFound,
        ] {
            assert!(matches!(
                TextGenerationError::for_gbnf_grammar_error(client_grammar_error),
                TextGenerationError::GrammarInvalid(_)
            ));
        }
    }

    #[test]
    fn a_grammar_the_agent_cannot_start_sampling_fails_its_sampler_initialization() {
        assert!(matches!(
            TextGenerationError::for_gbnf_grammar_error(SamplerError::GrammarNotEnoughMemory),
            TextGenerationError::GrammarSamplerInitializationFailed(
                SamplerError::GrammarNotEnoughMemory
            )
        ));
    }

    #[test]
    fn reports_chat_template_failures_as_chat_template_errors() {
        assert_each_reported_as(
            GeneratedTokenResult::ChatTemplateError,
            [
                TextGenerationError::ChatSessionCreationFailed(
                    ChatError::TemplateCannotServeRequest {
                        message: "roles must alternate".to_owned(),
                    },
                ),
                TextGenerationError::ChatPromptUnreadable(ChatError::SessionNotEnoughMemory),
            ],
        );
    }

    #[test]
    fn reports_each_media_failure_as_undecodable_media_of_its_kind() {
        for (rejection, expected_media) in [
            (
                TextGenerationError::ImageDecodingFailed(DecodedImageError::MissingCommaSeparator),
                MediaKind::Image,
            ),
            (
                TextGenerationError::ImageBitmapCreationFailed(MtmdError::BitmapNotEnoughMemory),
                MediaKind::Image,
            ),
            (
                TextGenerationError::AudioBitmapCreationFailed(MtmdError::BitmapNotEnoughMemory),
                MediaKind::Audio,
            ),
            (
                TextGenerationError::AudioDataNotBase64(DecodeError::InvalidPadding),
                MediaKind::Audio,
            ),
        ] {
            assert!(matches!(
                reported(rejection),
                GeneratedTokenResult::MediaDecodingFailed(UndecodableMedia { media, .. })
                    if media == expected_media
            ));
        }
    }

    #[test]
    fn reports_each_unsupported_conversation_feature_as_that_feature() {
        for (rejection, expected_feature) in [
            (
                TextGenerationError::ConversationUnsupported,
                ConversationFeature::ChatTemplate,
            ),
            (
                TextGenerationError::ThinkingUnsupported,
                ConversationFeature::Thinking,
            ),
            (
                TextGenerationError::ToolCallingUnsupported,
                ConversationFeature::ToolCalling,
            ),
            (
                TextGenerationError::ParallelToolCallingUnsupported,
                ConversationFeature::ParallelToolCalling,
            ),
            (
                TextGenerationError::AudioInputUnsupported,
                ConversationFeature::AudioInput,
            ),
        ] {
            assert!(matches!(
                reported(rejection),
                GeneratedTokenResult::ConversationFeatureNotSupported(
                    UnsupportedConversationFeature { feature, .. }
                ) if feature == expected_feature
            ));
        }
    }

    #[test]
    fn reports_a_model_generating_no_text_as_text_generation_not_served() {
        assert_each_reported_as(
            GeneratedTokenResult::TextGenerationNotServed,
            [TextGenerationError::TextGenerationNotServed],
        );
    }

    #[test]
    fn reports_missing_multimodal_support_as_multimodal_not_supported() {
        assert_each_reported_as(
            GeneratedTokenResult::MultimodalNotSupported,
            [TextGenerationError::MultimodalNotSupported],
        );
    }

    #[test]
    fn reports_an_invalid_tool_schema_as_an_invalid_tool_schema() {
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
            .unwrap();

        assert_each_reported_as(
            GeneratedTokenResult::ToolSchemaInvalid,
            [TextGenerationError::ToolSchemaInvalid(schema_error)],
        );
    }

    #[test]
    fn reports_failures_to_start_a_grammar_as_grammar_initialization_failures() {
        assert_each_reported_as(
            GeneratedTokenResult::GrammarInitializationFailed,
            [
                TextGenerationError::GrammarSamplerInitializationFailed(
                    SamplerError::GrammarNotEnoughMemory,
                ),
                TextGenerationError::ChatSessionGrammarUnavailable(ChatError::Grammar(
                    SamplerError::GrammarNotEnoughMemory,
                )),
            ],
        );
    }

    #[test]
    fn reports_media_exceeding_the_micro_batch_with_its_token_counts() {
        assert_eq!(
            reported(TextGenerationError::for_multimodal_evaluation_error(
                LlamaCppError::MultimodalChunksEval(MtmdError::NonCausalChunkExceedsMicroBatch(
                    NonCausalChunkMicroBatchMismatch {
                        chunk_tokens: 256,
                        micro_batch_tokens: 128,
                    }
                ))
            )),
            GeneratedTokenResult::MediaExceedsMicroBatch(OversizedMediaDetails {
                media_tokens: 256,
                micro_batch_tokens: 128,
            })
        );
    }

    #[test]
    fn reports_other_multimodal_evaluation_failures_as_ingestion_failures() {
        assert_each_reported_as(
            GeneratedTokenResult::MultimodalIngestionFailed,
            [TextGenerationError::for_multimodal_evaluation_error(
                LlamaCppError::MultimodalChunksEval(MtmdError::EvalNotEnoughMemory),
            )],
        );
    }

    #[test]
    fn reports_an_oversized_prompt_with_its_token_counts() {
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
    fn reports_internal_scheduler_failures_under_their_own_results() {
        assert_each_reported_as(
            GeneratedTokenResult::SchedulerUnavailable,
            [TextGenerationError::SchedulerUnavailable],
        );
        assert_each_reported_as(
            GeneratedTokenResult::NoSequenceSlotAvailable,
            [TextGenerationError::NoSequenceSlotAvailable],
        );
        assert_each_reported_as(
            GeneratedTokenResult::BatchAssemblyFailed,
            [TextGenerationError::BatchAssemblyFailed(
                ContextError::NoSequenceIds,
            )],
        );
        assert_each_reported_as(
            GeneratedTokenResult::KvCacheClearFailed,
            [TextGenerationError::KvCacheClearFailed(
                ContextMemoryError::SeqCpMemoryHandleUnavailable,
            )],
        );
        assert_each_reported_as(
            GeneratedTokenResult::PromptTokenizedToNothing,
            [TextGenerationError::PromptTokenizedToNothing],
        );
    }

    #[test]
    fn reports_failures_of_a_loaded_model_as_model_not_loaded() {
        assert_each_reported_as(
            GeneratedTokenResult::ModelNotLoaded,
            [
                TextGenerationError::ModelNotLoaded,
                TextGenerationError::AgentRuntime(AgentRuntimeError::AvailableParallelismUnknown(
                    io::Error::other("parallelism unknown"),
                )),
                TextGenerationError::SamplingParametersUnfitForTheSequenceContext(
                    InvalidInferenceParameters::PenaltyWindowExceedsSequenceContext {
                        penalty_last_n: 2048,
                        sequence_context_size: 1024,
                    },
                ),
            ],
        );
    }

    #[test]
    fn reports_prompt_tokenization_failures_as_prompt_tokenization_failures() {
        assert_each_reported_as(
            GeneratedTokenResult::PromptTokenizationFailed,
            [
                TextGenerationError::ChatPromptTokenizationFailed(ChatError::PromptTokenization(
                    ModelError::TokenizeNotEnoughMemory,
                )),
                TextGenerationError::PromptTokenizationFailed(ModelError::TokenizeNotEnoughMemory),
            ],
        );
    }

    #[test]
    fn reports_answer_classification_failures_as_answer_classification_failures() {
        assert_each_reported_as(
            GeneratedTokenResult::AnswerClassificationFailed,
            [
                TextGenerationError::AnswerClassifierUnavailable(
                    LlamaCppError::EmptyAnswerReading(ChatError::ParseNotEnoughMemory),
                ),
                TextGenerationError::for_ingest_error(LlamaCppError::Classify(
                    ChatError::ParseLlamaCppOutOfMemory,
                )),
            ],
        );
    }

    #[test]
    fn reports_a_detokenization_failure_as_a_detokenization_failure() {
        assert_each_reported_as(
            GeneratedTokenResult::DetokenizationFailed,
            [TextGenerationError::for_ingest_error(
                LlamaCppError::Detokenize(ModelError::PieceInsufficientBufferSpace(-4)),
            )],
        );
    }

    #[test]
    fn reports_multimodal_tokenization_failures_as_multimodal_tokenization_failures() {
        assert_each_reported_as(
            GeneratedTokenResult::MultimodalTokenizationFailed,
            [
                TextGenerationError::MultimodalTokenizationFailed(
                    MtmdError::BitmapCountDoesNotMatchMarkerCount,
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
                SamplerError::InitNotEnoughMemory,
            )],
        );
    }

    #[test]
    fn reports_sampling_loop_failures_as_sampler_errors() {
        assert_each_reported_as(
            GeneratedTokenResult::SamplerError,
            [
                TextGenerationError::GrammarConstraintFailed(SamplerError::ApplyNotEnoughMemory),
                TextGenerationError::SamplerChainApplicationFailed(
                    SamplerError::ApplyNotEnoughMemory,
                ),
                TextGenerationError::SamplerChainRejectedToken(SamplerError::AcceptNotEnoughMemory),
                TextGenerationError::TokenDataUnreadable(ContextError::LogitsNull),
                TextGenerationError::for_ingest_error(LlamaCppError::Sampler(
                    SamplerError::EveryCandidateExcluded,
                )),
            ],
        );
    }

    #[test]
    fn a_classification_failure_other_than_detokenizing_or_parsing_fails_the_token_ingestion() {
        assert!(matches!(
            TextGenerationError::for_ingest_error(LlamaCppError::Sampler(
                SamplerError::EveryCandidateExcluded
            )),
            TextGenerationError::TokenIngestionFailed(LlamaCppError::Sampler(
                SamplerError::EveryCandidateExcluded
            ))
        ));
    }
}
