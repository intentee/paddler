use serde::Deserialize;

use paddler_inference_parameters::model_default_or::ModelDefaultOr;

use crate::reasoning_effort::ReasoningEffort;

#[derive(Deserialize)]
pub struct ResponsesReasoning {
    pub effort: Option<ReasoningEffort>,
}

impl ResponsesReasoning {
    #[must_use]
    pub fn thinking(&self) -> ModelDefaultOr<bool> {
        self.effort
            .map_or(ModelDefaultOr::ModelDefault, ReasoningEffort::thinking)
    }
}
