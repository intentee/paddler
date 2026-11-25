use std::num::NonZeroU16;

use llama_cpp_bindings_types::decision_request::DecisionRequest;
use serde::Deserialize;
use serde::Serialize;

use crate::constants::DECISION_SLOTS_MINIMUM;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecideParams {
    pub request: DecisionRequest,
}

impl DecideParams {
    #[must_use]
    pub fn slots_needed(&self) -> NonZeroU16 {
        if self.request.questions.len() > 1 {
            DECISION_SLOTS_MINIMUM
        } else {
            NonZeroU16::MIN
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU16;

    use indexmap::IndexMap;
    use llama_cpp_bindings_types::decision_question::DecisionQuestion;
    use llama_cpp_bindings_types::decision_request::DecisionRequest;
    use serde_json::json;

    use super::DecideParams;
    use crate::constants::DECISION_SLOTS_MINIMUM;

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
    fn a_decision_asking_one_question_needs_only_its_state_slot() {
        assert_eq!(
            decision_with_questions(&["only"]).slots_needed(),
            NonZeroU16::MIN
        );
    }

    #[test]
    fn a_decision_asking_several_questions_needs_its_state_and_question_lane_slots() {
        assert_eq!(
            decision_with_questions(&["first", "second"]).slots_needed(),
            DECISION_SLOTS_MINIMUM
        );
    }
}
