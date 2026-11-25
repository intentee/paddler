use paddler_messaging::generated_token_final_result::GeneratedTokenFinalResult;
use paddler_messaging::generation_summary::GenerationSummary;
use paddler_messaging::oversized_media_details::OversizedMediaDetails;
use paddler_messaging::oversized_prompt_details::OversizedPromptDetails;
use paddler_messaging::undecodable_media::UndecodableMedia;
use paddler_messaging::unsupported_conversation_feature::UnsupportedConversationFeature;

use crate::generation_failure::GenerationFailure;
use crate::generation_failure_cause::GenerationFailureCause;

fn media_exceeds_micro_batch_message(
    OversizedMediaDetails {
        media_tokens,
        micro_batch_tokens,
    }: &OversizedMediaDetails,
) -> String {
    format!(
        "media required {media_tokens} tokens but one agent micro batch holds {micro_batch_tokens} tokens"
    )
}

fn prompt_exceeds_context_size_message(
    OversizedPromptDetails {
        prompt_tokens,
        sequence_context_size,
    }: &OversizedPromptDetails,
) -> String {
    format!(
        "prompt has {prompt_tokens} tokens but each agent sequence holds {sequence_context_size} tokens; shorten the prompt or raise context_size"
    )
}

#[derive(Debug, Eq, PartialEq)]
pub enum GenerationOutcome {
    Failed(GenerationFailure),
    Finished(GenerationSummary),
}

impl GenerationOutcome {
    const fn failed(cause: GenerationFailureCause, message: String) -> Self {
        Self::Failed(GenerationFailure { cause, message })
    }
}

impl From<GeneratedTokenFinalResult> for GenerationOutcome {
    fn from(generated_token_final_result: GeneratedTokenFinalResult) -> Self {
        match generated_token_final_result {
            GeneratedTokenFinalResult::Done(generation_summary) => {
                Self::Finished(generation_summary)
            }
            GeneratedTokenFinalResult::MediaExceedsMicroBatch(oversized_media_details) => {
                Self::failed(
                    GenerationFailureCause::InvalidRequest,
                    media_exceeds_micro_batch_message(&oversized_media_details),
                )
            }
            GeneratedTokenFinalResult::PromptExceedsContextSize(oversized_prompt_details) => {
                Self::failed(
                    GenerationFailureCause::InvalidRequest,
                    prompt_exceeds_context_size_message(&oversized_prompt_details),
                )
            }
            GeneratedTokenFinalResult::ConversationFeatureNotSupported(
                UnsupportedConversationFeature { description, .. },
            )
            | GeneratedTokenFinalResult::GrammarIncompatibleWithThinking(description)
            | GeneratedTokenFinalResult::GrammarSyntaxError(description)
            | GeneratedTokenFinalResult::MediaDecodingFailed(UndecodableMedia {
                description,
                ..
            })
            | GeneratedTokenFinalResult::MultimodalNotSupported(description)
            | GeneratedTokenFinalResult::MultimodalTokenizationFailed(description)
            | GeneratedTokenFinalResult::PromptTokenizationFailed(description)
            | GeneratedTokenFinalResult::PromptTokenizedToNothing(description)
            | GeneratedTokenFinalResult::ToolSchemaInvalid(description) => {
                Self::failed(GenerationFailureCause::InvalidRequest, description)
            }
            GeneratedTokenFinalResult::ModelNotLoaded(description)
            | GeneratedTokenFinalResult::NoSequenceSlotAvailable(description)
            | GeneratedTokenFinalResult::SchedulerUnavailable(description)
            | GeneratedTokenFinalResult::TextGenerationNotServed(description) => {
                Self::failed(GenerationFailureCause::Unavailable, description)
            }
            GeneratedTokenFinalResult::BatchAssemblyFailed(description)
            | GeneratedTokenFinalResult::ChatTemplateError(description)
            | GeneratedTokenFinalResult::DecodeFailed(description)
            | GeneratedTokenFinalResult::DetokenizationFailed(description)
            | GeneratedTokenFinalResult::AnswerClassificationFailed(description)
            | GeneratedTokenFinalResult::GrammarInitializationFailed(description)
            | GeneratedTokenFinalResult::GrammarRejectedModelOutput(description)
            | GeneratedTokenFinalResult::KvCacheClearFailed(description)
            | GeneratedTokenFinalResult::MultimodalIngestionFailed(description)
            | GeneratedTokenFinalResult::SamplerChainCreationFailed(description)
            | GeneratedTokenFinalResult::SamplerError(description)
            | GeneratedTokenFinalResult::SamplingCandidatesExhausted(description) => {
                Self::failed(GenerationFailureCause::AgentFailed, description)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::conversation_feature::ConversationFeature;
    use paddler_messaging::generated_token_final_result::GeneratedTokenFinalResult;
    use paddler_messaging::generation_finish::GenerationFinish;
    use paddler_messaging::generation_summary::GenerationSummary;
    use paddler_messaging::media_kind::MediaKind;
    use paddler_messaging::oversized_media_details::OversizedMediaDetails;
    use paddler_messaging::oversized_prompt_details::OversizedPromptDetails;
    use paddler_messaging::token_usage::TokenUsage;
    use paddler_messaging::undecodable_media::UndecodableMedia;
    use paddler_messaging::unsupported_conversation_feature::UnsupportedConversationFeature;

    use super::GenerationOutcome;
    use crate::generation_failure::GenerationFailure;
    use crate::generation_failure_cause::GenerationFailureCause;

    fn assert_failure_classified_as(
        expected_cause: GenerationFailureCause,
        generated_token_final_result: GeneratedTokenFinalResult,
    ) {
        assert!(matches!(
            GenerationOutcome::from(generated_token_final_result),
            GenerationOutcome::Failed(GenerationFailure { cause, .. }) if cause == expected_cause
        ));
    }

    #[test]
    fn classifies_the_summary_as_a_finished_generation() {
        let generation_summary = GenerationSummary {
            finish: GenerationFinish::EndOfGeneration,
            usage: TokenUsage::default(),
        };

        assert_eq!(
            GenerationOutcome::from(GeneratedTokenFinalResult::Done(generation_summary)),
            GenerationOutcome::Finished(generation_summary)
        );
    }

    #[test]
    fn classifies_client_mistakes_as_invalid_requests() {
        for generated_token_final_result in [
            GeneratedTokenFinalResult::GrammarIncompatibleWithThinking(String::new()),
            GeneratedTokenFinalResult::GrammarSyntaxError(String::new()),
            GeneratedTokenFinalResult::MediaDecodingFailed(UndecodableMedia {
                description: String::new(),
                media: MediaKind::Audio,
            }),
            GeneratedTokenFinalResult::MediaExceedsMicroBatch(OversizedMediaDetails {
                media_tokens: 256,
                micro_batch_tokens: 128,
            }),
            GeneratedTokenFinalResult::MultimodalNotSupported(String::new()),
            GeneratedTokenFinalResult::ConversationFeatureNotSupported(
                UnsupportedConversationFeature {
                    description: String::new(),
                    feature: ConversationFeature::Thinking,
                },
            ),
            GeneratedTokenFinalResult::MultimodalTokenizationFailed(String::new()),
            GeneratedTokenFinalResult::PromptExceedsContextSize(OversizedPromptDetails {
                prompt_tokens: 9895,
                sequence_context_size: 8192,
            }),
            GeneratedTokenFinalResult::PromptTokenizationFailed(String::new()),
            GeneratedTokenFinalResult::PromptTokenizedToNothing(String::new()),
            GeneratedTokenFinalResult::ToolSchemaInvalid(String::new()),
        ] {
            assert_failure_classified_as(
                GenerationFailureCause::InvalidRequest,
                generated_token_final_result,
            );
        }
    }

    #[test]
    fn classifies_agents_that_cannot_serve_as_unavailable() {
        for generated_token_final_result in [
            GeneratedTokenFinalResult::ModelNotLoaded(String::new()),
            GeneratedTokenFinalResult::NoSequenceSlotAvailable(String::new()),
            GeneratedTokenFinalResult::SchedulerUnavailable(String::new()),
            GeneratedTokenFinalResult::TextGenerationNotServed(String::new()),
        ] {
            assert_failure_classified_as(
                GenerationFailureCause::Unavailable,
                generated_token_final_result,
            );
        }
    }

    #[test]
    fn classifies_agent_failures_as_agent_failed() {
        for generated_token_final_result in [
            GeneratedTokenFinalResult::BatchAssemblyFailed(String::new()),
            GeneratedTokenFinalResult::ChatTemplateError(String::new()),
            GeneratedTokenFinalResult::DecodeFailed(String::new()),
            GeneratedTokenFinalResult::DetokenizationFailed(String::new()),
            GeneratedTokenFinalResult::AnswerClassificationFailed(String::new()),
            GeneratedTokenFinalResult::GrammarInitializationFailed(String::new()),
            GeneratedTokenFinalResult::GrammarRejectedModelOutput(String::new()),
            GeneratedTokenFinalResult::KvCacheClearFailed(String::new()),
            GeneratedTokenFinalResult::MultimodalIngestionFailed(String::new()),
            GeneratedTokenFinalResult::SamplerChainCreationFailed(String::new()),
            GeneratedTokenFinalResult::SamplerError(String::new()),
            GeneratedTokenFinalResult::SamplingCandidatesExhausted(String::new()),
        ] {
            assert_failure_classified_as(
                GenerationFailureCause::AgentFailed,
                generated_token_final_result,
            );
        }
    }
}
