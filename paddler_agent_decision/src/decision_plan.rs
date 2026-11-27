use llama_cpp_decision::prepared_decision_request::PreparedDecisionRequest;

use crate::planned_decision_prompt::PlannedDecisionPrompt;
use crate::question_lane_demand::QuestionLaneDemand;

pub struct DecisionPlan {
    pub prepared: PreparedDecisionRequest,
    pub prompts: Vec<PlannedDecisionPrompt>,
    pub shared_prefix_length: usize,
}

impl DecisionPlan {
    #[must_use]
    pub const fn question_lane_demand(&self) -> QuestionLaneDemand {
        if self.shared_prefix_length > 0 {
            QuestionLaneDemand::Needed
        } else {
            QuestionLaneDemand::NotNeeded
        }
    }

    #[must_use]
    pub fn required_cells(&self) -> usize {
        self.prompts
            .iter()
            .map(|PlannedDecisionPrompt { prompt, .. }| prompt.tokens().len())
            .fold(0, usize::max)
    }
}
