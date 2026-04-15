use crate::agent::continue_from_conversation_history_request::ContinueFromConversationHistoryRequest;
use crate::agent::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
use crate::agent::generate_embedding_batch_request::GenerateEmbeddingBatchRequest;

pub enum ContinuousBatchSchedulerCommand {
    ContinueFromConversationHistory(ContinueFromConversationHistoryRequest),
    ContinueFromRawPrompt(ContinueFromRawPromptRequest),
    GenerateEmbeddingBatch(GenerateEmbeddingBatchRequest),
    Shutdown,
}
