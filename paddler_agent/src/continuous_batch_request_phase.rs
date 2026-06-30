#[derive(Debug, Eq, PartialEq)]
pub enum ContinuousBatchRequestPhase {
    Ingesting,
    Generating,
    Completed,
}
