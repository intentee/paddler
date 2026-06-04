use actix_ws::CloseReason;

pub struct ContinuationStopParameters {
    pub close_reason: Option<CloseReason>,
}
