use serde::Deserialize;

use crate::reasoning_effort::ReasoningEffort;

#[derive(Deserialize)]
pub struct ResponsesReasoning {
    pub effort: Option<ReasoningEffort>,
}

impl ResponsesReasoning {
    #[must_use]
    pub fn enables_thinking(&self) -> bool {
        self.effort.is_none_or(ReasoningEffort::enables_thinking)
    }
}
