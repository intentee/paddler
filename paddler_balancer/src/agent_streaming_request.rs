use std::sync::Arc;

use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::management_socket::agent::request::Request as AgentJsonRpcRequest;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_messaging::request_params::decide_params::DecideParams;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;

use crate::agent_response_senders::AgentResponseSenders;
use crate::response_senders::ResponseSenders;

pub trait AgentStreamingRequest: Into<AgentJsonRpcRequest> {
    type Response: Send + 'static;

    fn response_senders(
        agent_response_senders: &AgentResponseSenders,
    ) -> &Arc<ResponseSenders<Self::Response>>;
}

impl AgentStreamingRequest for ContinueFromConversationHistoryParams<ValidatedParametersSchema> {
    type Response = GeneratedTokenResult;

    fn response_senders(
        agent_response_senders: &AgentResponseSenders,
    ) -> &Arc<ResponseSenders<Self::Response>> {
        &agent_response_senders.generated_tokens
    }
}

impl AgentStreamingRequest for ContinueFromRawPromptParams {
    type Response = GeneratedTokenResult;

    fn response_senders(
        agent_response_senders: &AgentResponseSenders,
    ) -> &Arc<ResponseSenders<Self::Response>> {
        &agent_response_senders.generated_tokens
    }
}

impl AgentStreamingRequest for DecideParams {
    type Response = DecisionResult;

    fn response_senders(
        agent_response_senders: &AgentResponseSenders,
    ) -> &Arc<ResponseSenders<Self::Response>> {
        &agent_response_senders.decision
    }
}

impl AgentStreamingRequest for GenerateEmbeddingBatchParams {
    type Response = EmbeddingResult;

    fn response_senders(
        agent_response_senders: &AgentResponseSenders,
    ) -> &Arc<ResponseSenders<Self::Response>> {
        &agent_response_senders.embedding
    }
}
