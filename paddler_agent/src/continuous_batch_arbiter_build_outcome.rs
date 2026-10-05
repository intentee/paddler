use crate::continuous_batch_arbiter::ContinuousBatchArbiter;

pub enum ContinuousBatchArbiterBuildOutcome {
    NoModelConfigured,
    ReadyToSpawn(Box<ContinuousBatchArbiter>),
}
