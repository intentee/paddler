#[derive(Debug, thiserror::Error)]
pub enum ChunkEvenlyWithCapError {
    #[error("agent_count must be non-zero")]
    ZeroAgentCount,
}
