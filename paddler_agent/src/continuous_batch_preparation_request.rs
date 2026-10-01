use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;

use crate::agent_request::AgentRequest;
use crate::embedding_batch_rejection::EmbeddingBatchRejection;
use crate::generation_request_rejection::GenerationRequestRejection;

pub enum ContinuousBatchPreparationRequest {
    ContinueFromConversationHistory(
        AgentRequest<
            ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
            GeneratedTokenResult,
        >,
    ),
    ContinueFromRawPrompt(AgentRequest<ContinueFromRawPromptParams, GeneratedTokenResult>),
    GenerateEmbeddingBatch(AgentRequest<GenerateEmbeddingBatchParams, EmbeddingResult>),
}

impl ContinuousBatchPreparationRequest {
    pub fn reject_because_no_model_is_loaded(self, agent_name: Option<&str>) {
        match self {
            Self::ContinueFromConversationHistory(AgentRequest { response_tx, .. })
            | Self::ContinueFromRawPrompt(AgentRequest { response_tx, .. }) => {
                GenerationRequestRejection::ModelNotLoaded.report(agent_name, &response_tx);
            }
            Self::GenerateEmbeddingBatch(AgentRequest { response_tx, .. }) => {
                EmbeddingBatchRejection::ModelNotLoaded.report(agent_name, &response_tx);
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
    > for ContinuousBatchPreparationRequest
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

impl From<AgentRequest<ContinueFromRawPromptParams, GeneratedTokenResult>>
    for ContinuousBatchPreparationRequest
{
    fn from(request: AgentRequest<ContinueFromRawPromptParams, GeneratedTokenResult>) -> Self {
        Self::ContinueFromRawPrompt(request)
    }
}

impl From<AgentRequest<GenerateEmbeddingBatchParams, EmbeddingResult>>
    for ContinuousBatchPreparationRequest
{
    fn from(request: AgentRequest<GenerateEmbeddingBatchParams, EmbeddingResult>) -> Self {
        Self::GenerateEmbeddingBatch(request)
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;
    use std::sync::Arc;

    use tokio::sync::mpsc;

    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_messaging::conversation_history::ConversationHistory;
    use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
    use paddler_messaging::embedding_result::EmbeddingResult;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
    use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;

    use super::ContinuousBatchPreparationRequest;
    use crate::agent_request::AgentRequest;
    use crate::slot_guard::SlotGuard;

    #[test]
    fn rejects_a_conversation_because_no_model_is_loaded() {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();
        let (_generate_tokens_stop_tx, generate_tokens_stop_rx) = mpsc::unbounded_channel();

        ContinuousBatchPreparationRequest::ContinueFromConversationHistory(AgentRequest {
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

        ContinuousBatchPreparationRequest::GenerateEmbeddingBatch(AgentRequest {
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
}
