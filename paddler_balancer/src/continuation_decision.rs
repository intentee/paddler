use crate::continuation_stop_parameters::ContinuationStopParameters;

pub enum ContinuationDecision {
    Continue,
    Stop(ContinuationStopParameters),
}
