use crate::continue_from_conversation_history_request::ContinueFromConversationHistoryRequest;
use crate::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
use crate::generate_embedding_batch_request::GenerateEmbeddingBatchRequest;

pub enum ContinuousBatchPreparationRequest {
    ContinueFromConversationHistory(ContinueFromConversationHistoryRequest),
    ContinueFromRawPrompt(ContinueFromRawPromptRequest),
    GenerateEmbeddingBatch(GenerateEmbeddingBatchRequest),
}

impl From<ContinueFromConversationHistoryRequest> for ContinuousBatchPreparationRequest {
    fn from(request: ContinueFromConversationHistoryRequest) -> Self {
        Self::ContinueFromConversationHistory(request)
    }
}

impl From<ContinueFromRawPromptRequest> for ContinuousBatchPreparationRequest {
    fn from(request: ContinueFromRawPromptRequest) -> Self {
        Self::ContinueFromRawPrompt(request)
    }
}

impl From<GenerateEmbeddingBatchRequest> for ContinuousBatchPreparationRequest {
    fn from(request: GenerateEmbeddingBatchRequest) -> Self {
        Self::GenerateEmbeddingBatch(request)
    }
}
