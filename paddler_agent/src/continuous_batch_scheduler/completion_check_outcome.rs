#[derive(Debug, Eq, PartialEq)]
pub enum CompletionCheckOutcome {
    Continue,
    ReachedEog,
    ReachedMaxTokens,
}
