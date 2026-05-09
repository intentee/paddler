use crate::agent::continuous_batch_arbiter::ContinuousBatchArbiter;

pub enum ContinuousBatchArbiterBuildOutcome {
    NoModelConfigured,
    ReadyToSpawn(ContinuousBatchArbiter),
}
