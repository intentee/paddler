use llama_cpp_bindings_types::decision_answer::DecisionAnswer;
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AnsweredDecisionQuestion {
    pub answer: DecisionAnswer,
    pub id: String,
}
