#[derive(Debug, Eq, PartialEq)]
pub enum RetryDelayOutcome {
    Cancelled,
    Elapsed,
}
