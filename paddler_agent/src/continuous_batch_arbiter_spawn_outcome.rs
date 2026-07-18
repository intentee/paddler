use crate::continuous_batch_arbiter_handle::ContinuousBatchArbiterHandle;

pub enum ContinuousBatchArbiterSpawnOutcome {
    Cancelled,
    Ready(ContinuousBatchArbiterHandle),
}
