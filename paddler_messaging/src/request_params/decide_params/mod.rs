pub mod raw_decide_params;

use std::num::NonZeroU16;

use serde::Deserialize;
use serde::Serialize;

use crate::decision_question::DecisionQuestion;
use crate::decision_slots_for_leading_questions::decision_slots_for_leading_questions;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecideParams {
    pub last_question: DecisionQuestion,
    pub leading_questions: Vec<DecisionQuestion>,
    pub state: String,
}

impl DecideParams {
    #[must_use]
    pub const fn required_slots(&self) -> NonZeroU16 {
        decision_slots_for_leading_questions(self.leading_questions.len())
    }
}
