use crate::generation_failure_cause::GenerationFailureCause;

#[derive(Debug, Eq, PartialEq)]
pub struct GenerationFailure {
    pub cause: GenerationFailureCause,
    pub message: String,
}
