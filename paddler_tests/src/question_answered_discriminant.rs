use std::mem::Discriminant;
use std::mem::discriminant;

use paddler_messaging::decision_answer::DecisionAnswer;
use paddler_messaging::decision_result::DecisionResult;

#[must_use]
pub fn question_answered_discriminant() -> Discriminant<DecisionResult> {
    discriminant(&DecisionResult::QuestionAnswered(DecisionAnswer {
        id: String::new(),
        probabilities: Vec::new(),
    }))
}
