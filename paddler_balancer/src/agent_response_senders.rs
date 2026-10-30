use std::sync::Arc;

use log::warn;

use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::management_socket::agent::response::Response as AgentJsonRpcResponse;
use paddler_messaging::model_metadata::ModelMetadata;
use paddler_request_registry::request_delivery::RequestDelivery;

use crate::response_senders::ResponseSenders;

fn forward_agent_response<TResponse>(
    response_senders: &ResponseSenders<TResponse>,
    request_id: &str,
    response: TResponse,
) {
    match response_senders.send_to(request_id, response) {
        RequestDelivery::Delivered => {}
        RequestDelivery::ReceiverDropped | RequestDelivery::RequestNotRegistered => {
            warn!(
                "Dropped an agent response for request {request_id:?}, which is no longer relayed"
            );
        }
    }
}

#[derive(Clone, Default)]
pub struct AgentResponseSenders {
    pub chat_template_override: Arc<ResponseSenders<Option<ChatTemplate>>>,
    pub decision: Arc<ResponseSenders<DecisionResult>>,
    pub embedding: Arc<ResponseSenders<EmbeddingResult>>,
    pub generated_tokens: Arc<ResponseSenders<GeneratedTokenResult>>,
    pub model_metadata: Arc<ResponseSenders<Option<ModelMetadata>>>,
}

impl AgentResponseSenders {
    pub fn forward(&self, request_id: &str, response: AgentJsonRpcResponse) {
        match response {
            AgentJsonRpcResponse::ChatTemplateOverride(chat_template_override) => {
                forward_agent_response(
                    &self.chat_template_override,
                    request_id,
                    chat_template_override,
                );
            }
            AgentJsonRpcResponse::Decision(decision_result) => {
                forward_agent_response(&self.decision, request_id, decision_result);
            }
            AgentJsonRpcResponse::Embedding(embedding_result) => {
                forward_agent_response(&self.embedding, request_id, embedding_result);
            }
            AgentJsonRpcResponse::GeneratedToken(generated_token_result) => {
                forward_agent_response(&self.generated_tokens, request_id, generated_token_result);
            }
            AgentJsonRpcResponse::ModelMetadata(model_metadata) => {
                forward_agent_response(&self.model_metadata, request_id, model_metadata);
            }
        }
    }
}
