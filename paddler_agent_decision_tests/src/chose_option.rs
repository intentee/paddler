use llama_cpp_bindings_types::decision_answer::DecisionAnswer;

use paddler_messaging::answered_decision_question::AnsweredDecisionQuestion;

#[must_use]
pub fn chose_option(
    answers: &[AnsweredDecisionQuestion],
    question_id: &str,
    expected_option: &str,
) -> bool {
    answers.iter().any(|AnsweredDecisionQuestion { answer, id }| {
        id == question_id
            && matches!(answer, DecisionAnswer::Choice { choice, .. } if choice == expected_option)
    })
}
