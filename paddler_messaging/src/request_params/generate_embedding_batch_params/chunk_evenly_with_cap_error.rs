#[derive(Debug, thiserror::Error)]
pub enum ChunkEvenlyWithCapError {
    #[error("agent_count must be non-zero")]
    ZeroAgentCount,
    #[error("max_documents_per_chunk must be non-zero")]
    ZeroMaxDocumentsPerChunk,
}
