use crate::prepared_embedding_batch_request::PreparedEmbeddingBatchRequest;
use crate::prepared_generation_request::PreparedGenerationRequest;

pub enum ContinuousBatchSchedulerCommand {
    Generate(Box<PreparedGenerationRequest>),
    GenerateEmbeddingBatch(PreparedEmbeddingBatchRequest),
    Shutdown,
}
