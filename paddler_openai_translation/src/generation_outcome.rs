use paddler_messaging::generated_token_final_result::GeneratedTokenFinalResult;
use paddler_messaging::generation_summary::GenerationSummary;
use paddler_messaging::oversized_media_details::OversizedMediaDetails;
use paddler_messaging::oversized_prompt_details::OversizedPromptDetails;

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
            GeneratedTokenFinalResult::GrammarIncompatibleWithThinking(description)
            | GeneratedTokenFinalResult::GrammarSyntaxError(description)
            | GeneratedTokenFinalResult::ImageDecodingFailed(description)
            | GeneratedTokenFinalResult::MultimodalNotSupported(description)
            | GeneratedTokenFinalResult::MultimodalTokenizationFailed(description)
            | GeneratedTokenFinalResult::PromptTokenizationFailed(description)
            | GeneratedTokenFinalResult::PromptTokenizedToNothing(description)
            | GeneratedTokenFinalResult::ToolSchemaInvalid(description) => {
                Self::failed(GenerationFailureCause::InvalidRequest, description)
            }
            GeneratedTokenFinalResult::ModelNotLoaded(description)
            | GeneratedTokenFinalResult::NoSequenceSlotAvailable(description)
            | GeneratedTokenFinalResult::SchedulerUnavailable(description) => {
                Self::failed(GenerationFailureCause::Unavailable, description)
            }
            GeneratedTokenFinalResult::BatchAssemblyFailed(description)
            | GeneratedTokenFinalResult::ChatTemplateError(description)
            | GeneratedTokenFinalResult::DecodeFailed(description)
            | GeneratedTokenFinalResult::DetokenizationFailed(description)
            | GeneratedTokenFinalResult::GrammarInitializationFailed(description)
            | GeneratedTokenFinalResult::GrammarRejectedModelOutput(description)
            | GeneratedTokenFinalResult::KvCacheClearFailed(description)
            | GeneratedTokenFinalResult::MediaMicroBatchCheckFailed(description)
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
    use llama_cpp_bindings_types::TokenUsage;

    use paddler_messaging::generated_token_final_result::GeneratedTokenFinalResult;
    use paddler_messaging::generation_finish::GenerationFinish;
    use paddler_messaging::generation_summary::GenerationSummary;
    use paddler_messaging::oversized_media_details::OversizedMediaDetails;
    use paddler_messaging::oversized_prompt_details::OversizedPromptDetails;

    use super::GenerationOutcome;
    use crate::generation_failure::GenerationFailure;
    use crate::generation_failure_cause::GenerationFailureCause;

    fn assert_failures_classified_as(
        expected_cause: GenerationFailureCause,
        failures: Vec<(GeneratedTokenFinalResult, &str)>,
    ) {
        for (generated_token_final_result, expected_message) in failures {
            assert_eq!(
                GenerationOutcome::from(generated_token_final_result),
                GenerationOutcome::Failed(GenerationFailure {
                    cause: expected_cause,
                    message: expected_message.to_owned(),
                })
            );
        }
    }

    #[test]
    fn classifies_the_summary_as_a_finished_generation() {
        let generation_summary = GenerationSummary {
            finish: GenerationFinish::EndOfGeneration,
            usage: TokenUsage::new(),
        };

        assert_eq!(
            GenerationOutcome::from(GeneratedTokenFinalResult::Done(generation_summary)),
            GenerationOutcome::Finished(generation_summary)
        );
    }

    #[test]
    fn classifies_client_mistakes_as_invalid_requests() {
        assert_failures_classified_as(
            GenerationFailureCause::InvalidRequest,
            vec![
                (
                    GeneratedTokenFinalResult::GrammarIncompatibleWithThinking(
                        "client mistake".to_owned(),
                    ),
                    "client mistake",
                ),
                (
                    GeneratedTokenFinalResult::GrammarSyntaxError("client mistake".to_owned()),
                    "client mistake",
                ),
                (
                    GeneratedTokenFinalResult::ImageDecodingFailed("client mistake".to_owned()),
                    "client mistake",
                ),
                (
                    GeneratedTokenFinalResult::MediaExceedsMicroBatch(OversizedMediaDetails {
                        media_tokens: 256,
                        micro_batch_tokens: 128,
                    }),
                    "media required 256 tokens but one agent micro batch holds 128 tokens",
                ),
                (
                    GeneratedTokenFinalResult::MultimodalNotSupported("client mistake".to_owned()),
                    "client mistake",
                ),
                (
                    GeneratedTokenFinalResult::MultimodalTokenizationFailed(
                        "client mistake".to_owned(),
                    ),
                    "client mistake",
                ),
                (
                    GeneratedTokenFinalResult::PromptExceedsContextSize(OversizedPromptDetails {
                        prompt_tokens: 9895,
                        sequence_context_size: 8192,
                    }),
                    "prompt has 9895 tokens but each agent sequence holds 8192 tokens; shorten the prompt or raise context_size",
                ),
                (
                    GeneratedTokenFinalResult::PromptTokenizationFailed(
                        "client mistake".to_owned(),
                    ),
                    "client mistake",
                ),
                (
                    GeneratedTokenFinalResult::PromptTokenizedToNothing(
                        "client mistake".to_owned(),
                    ),
                    "client mistake",
                ),
                (
                    GeneratedTokenFinalResult::ToolSchemaInvalid("client mistake".to_owned()),
                    "client mistake",
                ),
            ],
        );
    }

    #[test]
    fn classifies_agents_that_cannot_serve_as_unavailable() {
        assert_failures_classified_as(
            GenerationFailureCause::Unavailable,
            vec![
                (
                    GeneratedTokenFinalResult::ModelNotLoaded("unavailable".to_owned()),
                    "unavailable",
                ),
                (
                    GeneratedTokenFinalResult::NoSequenceSlotAvailable("unavailable".to_owned()),
                    "unavailable",
                ),
                (
                    GeneratedTokenFinalResult::SchedulerUnavailable("unavailable".to_owned()),
                    "unavailable",
                ),
            ],
        );
    }

    #[test]
    fn classifies_agent_failures_as_agent_failed() {
        assert_failures_classified_as(
            GenerationFailureCause::AgentFailed,
            vec![
                (
                    GeneratedTokenFinalResult::BatchAssemblyFailed("agent failure".to_owned()),
                    "agent failure",
                ),
                (
                    GeneratedTokenFinalResult::ChatTemplateError("agent failure".to_owned()),
                    "agent failure",
                ),
                (
                    GeneratedTokenFinalResult::DecodeFailed("agent failure".to_owned()),
                    "agent failure",
                ),
                (
                    GeneratedTokenFinalResult::DetokenizationFailed("agent failure".to_owned()),
                    "agent failure",
                ),
                (
                    GeneratedTokenFinalResult::GrammarInitializationFailed(
                        "agent failure".to_owned(),
                    ),
                    "agent failure",
                ),
                (
                    GeneratedTokenFinalResult::GrammarRejectedModelOutput(
                        "agent failure".to_owned(),
                    ),
                    "agent failure",
                ),
                (
                    GeneratedTokenFinalResult::KvCacheClearFailed("agent failure".to_owned()),
                    "agent failure",
                ),
                (
                    GeneratedTokenFinalResult::MediaMicroBatchCheckFailed(
                        "agent failure".to_owned(),
                    ),
                    "agent failure",
                ),
                (
                    GeneratedTokenFinalResult::MultimodalIngestionFailed(
                        "agent failure".to_owned(),
                    ),
                    "agent failure",
                ),
                (
                    GeneratedTokenFinalResult::SamplerChainCreationFailed(
                        "agent failure".to_owned(),
                    ),
                    "agent failure",
                ),
                (
                    GeneratedTokenFinalResult::SamplerError("agent failure".to_owned()),
                    "agent failure",
                ),
                (
                    GeneratedTokenFinalResult::SamplingCandidatesExhausted(
                        "agent failure".to_owned(),
                    ),
                    "agent failure",
                ),
            ],
        );
    }
}
