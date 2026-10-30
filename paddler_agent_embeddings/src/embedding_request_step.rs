#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmbeddingRequestStep {
    Continuing,
    Ended,
}
