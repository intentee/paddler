#[derive(Debug)]
pub enum ContinuousBatchRequestPhase {
    Ingesting,
    Generating,
    Completed,
}
