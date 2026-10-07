use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_messaging::request_params::decide_params::DecideParams;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_embeddings::embedding_error::EmbeddingError;
use paddler_agent_runtime::agent_request::AgentRequest;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;

pub enum PipelineRequest {
    ContinueFromConversationHistory(
        AgentRequest<
            ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
            GeneratedTokenResult,
        >,
    ),
    ContinueFromRawPrompt(AgentRequest<ContinueFromRawPromptParams, GeneratedTokenResult>),
    Decide(AgentRequest<DecideParams, DecisionResult>),
    GenerateEmbeddingBatch(AgentRequest<GenerateEmbeddingBatchParams, EmbeddingResult>),
}

impl PipelineRequest {
    pub fn reject_because_another_mode_is_served(
        self,
        agent_name: Option<&str>,
        serving_inference_mode: InferenceMode,
    ) {
        match self {
            Self::ContinueFromConversationHistory(AgentRequest { response_tx, .. })
            | Self::ContinueFromRawPrompt(AgentRequest { response_tx, .. }) => {
                TextGenerationError::InferenceModeMismatch {
                    serving_inference_mode,
                }
                .report(agent_name, &response_tx);
            }
            Self::Decide(AgentRequest { response_tx, .. }) => {
                DecisionError::InferenceModeMismatch {
                    serving_inference_mode,
                }
                .report(agent_name, &response_tx);
            }
            Self::GenerateEmbeddingBatch(AgentRequest { response_tx, .. }) => {
                EmbeddingError::InferenceModeMismatch {
                    serving_inference_mode,
                }
                .report(agent_name, &response_tx);
            }
        }
    }

    pub fn reject_because_no_model_is_loaded(self, agent_name: Option<&str>) {
        match self {
            Self::ContinueFromConversationHistory(AgentRequest { response_tx, .. })
            | Self::ContinueFromRawPrompt(AgentRequest { response_tx, .. }) => {
                TextGenerationError::ModelNotLoaded.report(agent_name, &response_tx);
            }
            Self::Decide(AgentRequest { response_tx, .. }) => {
                DecisionError::ModelNotLoaded.report(agent_name, &response_tx);
            }
            Self::GenerateEmbeddingBatch(AgentRequest { response_tx, .. }) => {
                EmbeddingError::ModelNotLoaded.report(agent_name, &response_tx);
            }
        }
    }
}

impl
    From<
        AgentRequest<
            ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
            GeneratedTokenResult,
        >,
    > for PipelineRequest
{
    fn from(
        request: AgentRequest<
            ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
            GeneratedTokenResult,
        >,
    ) -> Self {
        Self::ContinueFromConversationHistory(request)
    }
}

impl From<AgentRequest<ContinueFromRawPromptParams, GeneratedTokenResult>> for PipelineRequest {
    fn from(request: AgentRequest<ContinueFromRawPromptParams, GeneratedTokenResult>) -> Self {
        Self::ContinueFromRawPrompt(request)
    }
}

impl From<AgentRequest<DecideParams, DecisionResult>> for PipelineRequest {
    fn from(request: AgentRequest<DecideParams, DecisionResult>) -> Self {
        Self::Decide(request)
    }
}

impl From<AgentRequest<GenerateEmbeddingBatchParams, EmbeddingResult>> for PipelineRequest {
    fn from(request: AgentRequest<GenerateEmbeddingBatchParams, EmbeddingResult>) -> Self {
        Self::GenerateEmbeddingBatch(request)
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;
    use std::sync::Arc;

    use tokio::sync::mpsc;

    use paddler_agent_runtime::agent_request::AgentRequest;
    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_agent_status::slot_guard::SlotGuard;
    use paddler_messaging::conversation_history::ConversationHistory;
    use paddler_messaging::decision_question::DecisionQuestion;
    use paddler_messaging::decision_result::DecisionResult;
    use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
    use paddler_messaging::embedding_result::EmbeddingResult;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::inference_mode::InferenceMode;
    use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
    use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
    use paddler_messaging::request_params::decide_params::DecideParams;
    use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;

    use super::PipelineRequest;

    #[test]
    fn rejects_a_conversation_because_no_model_is_loaded() {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();
        let (_generate_tokens_stop_tx, generate_tokens_stop_rx) = mpsc::unbounded_channel();

        PipelineRequest::ContinueFromConversationHistory(AgentRequest {
            params: ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(Vec::new()),
                enable_thinking: false,
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                parse_tool_calls: false,
                tools: Vec::new(),
            },
            response_tx: generated_tokens_tx,
            slot_guard: SlotGuard::new(Arc::new(SlotAggregatedStatus::new(1))),
            stop_rx: generate_tokens_stop_rx,
        })
        .reject_because_no_model_is_loaded(Some("agent"));

        assert_eq!(
            generated_tokens_rx.try_recv(),
            Ok(GeneratedTokenResult::ModelNotLoaded(
                "agent: no model is loaded".to_owned()
            ))
        );
    }

    #[test]
    fn rejects_an_embedding_batch_because_no_model_is_loaded() {
        let (generated_embedding_tx, mut generated_embedding_rx) = mpsc::unbounded_channel();
        let (_generate_embedding_stop_tx, generate_embedding_stop_rx) = mpsc::unbounded_channel();

        PipelineRequest::GenerateEmbeddingBatch(AgentRequest {
            params: GenerateEmbeddingBatchParams {
                input_batch: Vec::new(),
                normalization_method: EmbeddingNormalizationMethod::None,
            },
            response_tx: generated_embedding_tx,
            slot_guard: SlotGuard::new(Arc::new(SlotAggregatedStatus::new(1))),
            stop_rx: generate_embedding_stop_rx,
        })
        .reject_because_no_model_is_loaded(Some("agent"));

        assert_eq!(
            generated_embedding_rx.try_recv(),
            Ok(EmbeddingResult::ModelNotLoaded(
                "agent: no model is loaded".to_owned()
            ))
        );
    }

    fn decide_request(
        decision_result_tx: mpsc::UnboundedSender<DecisionResult>,
    ) -> PipelineRequest {
        let (_decision_stop_tx, decision_stop_rx) = mpsc::unbounded_channel();

        PipelineRequest::Decide(AgentRequest {
            params: DecideParams {
                last_question: DecisionQuestion {
                    id: "paid".to_owned(),
                    instructions: "Was it paid?".to_owned(),
                    options: vec!["no".to_owned(), "yes".to_owned()],
                },
                leading_questions: Vec::new(),
                state: "state".to_owned(),
            },
            response_tx: decision_result_tx,
            slot_guard: SlotGuard::new(Arc::new(SlotAggregatedStatus::new(1))),
            stop_rx: decision_stop_rx,
        })
    }

    #[test]
    fn rejects_a_decision_because_no_model_is_loaded() {
        let (decision_result_tx, mut decision_result_rx) = mpsc::unbounded_channel();

        decide_request(decision_result_tx).reject_because_no_model_is_loaded(Some("agent"));

        assert_eq!(
            decision_result_rx.try_recv(),
            Ok(DecisionResult::ModelNotLoaded(
                "agent: no model is loaded".to_owned()
            ))
        );
    }

    #[test]
    fn rejects_a_decision_on_an_agent_serving_another_mode() {
        let (decision_result_tx, mut decision_result_rx) = mpsc::unbounded_channel();

        decide_request(decision_result_tx)
            .reject_because_another_mode_is_served(Some("agent"), InferenceMode::Embeddings);

        assert!(matches!(
            decision_result_rx.try_recv(),
            Ok(DecisionResult::InferenceModeMismatch(_))
        ));
    }

    #[test]
    fn rejects_an_embedding_batch_on_an_agent_serving_another_mode() {
        let (generated_embedding_tx, mut generated_embedding_rx) = mpsc::unbounded_channel();
        let (_generate_embedding_stop_tx, generate_embedding_stop_rx) = mpsc::unbounded_channel();

        PipelineRequest::GenerateEmbeddingBatch(AgentRequest {
            params: GenerateEmbeddingBatchParams {
                input_batch: Vec::new(),
                normalization_method: EmbeddingNormalizationMethod::None,
            },
            response_tx: generated_embedding_tx,
            slot_guard: SlotGuard::new(Arc::new(SlotAggregatedStatus::new(1))),
            stop_rx: generate_embedding_stop_rx,
        })
        .reject_because_another_mode_is_served(Some("agent"), InferenceMode::Decision);

        assert!(matches!(
            generated_embedding_rx.try_recv(),
            Ok(EmbeddingResult::InferenceModeMismatch(_))
        ));
    }

    #[test]
    fn rejects_token_generation_on_an_agent_serving_another_mode() {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();
        let (_generate_tokens_stop_tx, generate_tokens_stop_rx) = mpsc::unbounded_channel();

        PipelineRequest::ContinueFromRawPrompt(AgentRequest {
            params: ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                raw_prompt: "Hello".to_owned(),
            },
            response_tx: generated_tokens_tx,
            slot_guard: SlotGuard::new(Arc::new(SlotAggregatedStatus::new(1))),
            stop_rx: generate_tokens_stop_rx,
        })
        .reject_because_another_mode_is_served(Some("agent"), InferenceMode::Decision);

        assert!(matches!(
            generated_tokens_rx.try_recv(),
            Ok(GeneratedTokenResult::InferenceModeMismatch(_))
        ));
    }
}
