pub mod raw_decide_params;

use serde::Deserialize;
use serde::Serialize;

use crate::decision_question::DecisionQuestion;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecideParams {
    pub last_question: DecisionQuestion,
    pub leading_questions: Vec<DecisionQuestion>,
    pub state: String,
}
