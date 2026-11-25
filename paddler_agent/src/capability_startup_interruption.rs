use crate::agent_error::AgentError;
use crate::model_serving::ModelServing;
use crate::replacement_outcome::ReplacementOutcome;

#[derive(Debug)]
pub enum CapabilityStartupInterruption {
    Cancelled,
    Failed(Box<AgentError>),
}

impl CapabilityStartupInterruption {
    #[must_use]
    pub fn into_replacement_outcome(self, held: ModelServing) -> ReplacementOutcome {
        match self {
            Self::Cancelled => ReplacementOutcome::Cancelled { held },
            Self::Failed(error) => ReplacementOutcome::Failed {
                error: *error,
                held,
            },
        }
    }
}
