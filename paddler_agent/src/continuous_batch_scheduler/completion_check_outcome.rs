#[derive(Debug, Eq, PartialEq)]
pub enum CompletionCheckOutcome {
    Continue,
    ReachedContextLimit,
    ReachedEndOfGeneration,
    ReachedMaxTokens,
}
