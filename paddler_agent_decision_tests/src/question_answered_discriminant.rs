use std::mem::Discriminant;
use std::mem::discriminant;

use llama_cpp_bindings_types::decision_answer::DecisionAnswer;

use paddler_messaging::answered_decision_question::AnsweredDecisionQuestion;
use paddler_messaging::decision_result::DecisionResult;

#[must_use]
pub fn question_answered_discriminant() -> Discriminant<DecisionResult> {
    discriminant(&DecisionResult::QuestionAnswered(
        AnsweredDecisionQuestion {
            answer: DecisionAnswer::Noul { noul: 0.0 },
            id: String::new(),
        },
    ))
}
