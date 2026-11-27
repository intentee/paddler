use llama_cpp_bindings_types::decision_request::DecisionRequest;
use serde::Deserialize;
use serde::Serialize;

use crate::decision_slot_demand::DecisionSlotDemand;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecideParams {
    pub request: DecisionRequest,
}

impl DecideParams {
    #[must_use]
    pub fn slot_demand(&self) -> DecisionSlotDemand {
        if self.request.questions.len() > 1 {
            DecisionSlotDemand::SeveralQuestions
        } else {
            DecisionSlotDemand::OneQuestion
        }
    }
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;
    use llama_cpp_bindings_types::decision_question::DecisionQuestion;
    use llama_cpp_bindings_types::decision_request::DecisionRequest;
    use serde_json::json;

    use super::DecideParams;
    use crate::decision_slot_demand::DecisionSlotDemand;

    fn decision_with_questions(question_ids: &[&str]) -> DecideParams {
        DecideParams {
            request: DecisionRequest {
                images: Vec::new(),
                questions: question_ids
                    .iter()
                    .map(|question_id| {
                        (
                            (*question_id).to_owned(),
                            DecisionQuestion::Noul {
                                criteria: IndexMap::new(),
                                instructions: json!("Is it so?"),
                            },
                        )
                    })
                    .collect(),
                state: json!("state"),
            },
        }
    }

    #[test]
    fn a_decision_asking_one_question_demands_the_slots_of_one_question() {
        assert_eq!(
            decision_with_questions(&["only"]).slot_demand(),
            DecisionSlotDemand::OneQuestion
        );
    }

    #[test]
    fn a_decision_asking_several_questions_demands_the_slots_of_several_questions() {
        assert_eq!(
            decision_with_questions(&["first", "second"]).slot_demand(),
            DecisionSlotDemand::SeveralQuestions
        );
    }
}
