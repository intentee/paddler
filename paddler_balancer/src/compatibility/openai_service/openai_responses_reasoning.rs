use serde::Deserialize;

use crate::compatibility::openai_service::openai_reasoning_effort::OpenAIReasoningEffort;

#[derive(Deserialize)]
pub struct OpenAIResponsesReasoning {
    pub effort: Option<OpenAIReasoningEffort>,
}

impl OpenAIResponsesReasoning {
    #[must_use]
    pub fn enables_thinking(&self) -> bool {
        self.effort
            .is_none_or(OpenAIReasoningEffort::enables_thinking)
    }
}
