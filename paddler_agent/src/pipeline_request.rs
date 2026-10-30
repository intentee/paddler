use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_decision::reports_decision_outcome::ReportsDecisionOutcome as _;
use paddler_agent_embeddings::embedding_error::EmbeddingError;
use paddler_agent_runtime::agent_request::AgentRequest;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_messaging::request_params::decide_params::DecideParams;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;

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
