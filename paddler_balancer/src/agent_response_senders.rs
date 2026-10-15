use std::sync::Arc;

use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::model_metadata::ModelMetadata;

use crate::response_senders::ResponseSenders;

#[derive(Clone, Default)]
pub struct AgentResponseSenders {
    pub chat_template_override: Arc<ResponseSenders<Option<ChatTemplate>>>,
    pub decision: Arc<ResponseSenders<DecisionResult>>,
    pub embedding: Arc<ResponseSenders<EmbeddingResult>>,
    pub generated_tokens: Arc<ResponseSenders<GeneratedTokenResult>>,
    pub model_metadata: Arc<ResponseSenders<Option<ModelMetadata>>>,
}
